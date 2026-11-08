//! Page-scoped, bounded audio queues over the account's one playback observer.
use std::{collections::HashMap, sync::Arc};

use mixin_desktop_api::{
    AccountClient, AudioPlaybackItem, AudioPlaybackStatus, ClientError, ClientResult,
    MessageListView,
};
use nsh_core::prelude::*;

use super::Playback;
use crate::runtime;

const PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub(super) enum Scope {
    Conversation(String),
    Pinned(String),
    Transcript(String),
}

impl Scope {
    fn item_key(&self, child: &str) -> String {
        match self {
            Self::Transcript(root) => format!("transcript:{root}:{child}"),
            _ => child.to_owned(),
        }
    }
}

#[derive(Clone)]
pub(super) struct Queue {
    request: u64,
    scope: Scope,
    tail: Option<String>,
    transcript_audio: HashMap<String, String>,
    accepted: bool,
    pending_end: Option<String>,
}

impl Queue {
    pub(super) fn transcript_source(&self, id: &str) -> Option<(String, String)> {
        let Scope::Transcript(root) = &self.scope else {
            return None;
        };
        self.transcript_audio
            .get(id)
            .map(|child| (root.clone(), child.clone()))
    }
}

pub(super) async fn load(
    account: Arc<AccountClient>,
    scope: Scope,
    anchor: String,
    after: i64,
) -> ClientResult<Vec<MessageListView>> {
    let messages = account.message();
    match scope {
        Scope::Conversation(conversation) => {
            messages
                .audio_messages_around(conversation, anchor, 0, after)
                .await
        }
        Scope::Pinned(conversation) => {
            messages
                .pinned_audio_messages_around(conversation, anchor, 0, after)
                .await
        }
        Scope::Transcript(root) => {
            messages
                .transcript_audio_messages_around(root, anchor, 0, after)
                .await
        }
    }
}

impl Playback {
    pub(super) fn keep_accepted_queue(self) {
        self.queue.update(|queue| {
            if let Some(queue) = queue.as_mut().filter(|queue| queue.accepted) {
                queue.request = self.request.get_untracked();
            }
        });
    }

    pub(super) async fn play_rows(
        self,
        messages: Vec<MessageListView>,
        anchor: String,
        scope: Scope,
        request: u64,
    ) -> ClientResult<()> {
        if self.request.get_untracked() != request {
            return Ok(());
        }
        let start = messages
            .iter()
            .position(|row| row.message_id == anchor)
            .ok_or_else(|| ClientError::Internal("audio message is no longer available".into()))?;
        let tail =
            (messages.len() == PAGE_SIZE).then(|| messages.last().unwrap().message_id.clone());
        let mut transcript_audio = HashMap::new();
        let playlist = messages
            .into_iter()
            .map(|row| {
                let id = scope.item_key(&row.message_id);
                if matches!(scope, Scope::Transcript(_)) {
                    transcript_audio.insert(id.clone(), row.message_id);
                }
                AudioPlaybackItem {
                    id,
                    path: row.media_url.unwrap_or_default(),
                    duration_millis: row.media_duration.parse().unwrap_or_default(),
                }
            })
            .collect();
        let previous = self.queue.get_untracked();
        self.queue.set(Some(Queue {
            request,
            scope,
            tail,
            transcript_audio,
            accepted: false,
            pending_end: None,
        }));
        let playback = self
            .media
            .with(|media| media.play_audio(playlist, start as u64));
        if let Err(error) = runtime::run(playback).await {
            if self.queue.with_untracked(|queue| {
                queue.as_ref().is_some_and(|queue| queue.request == request)
            }) {
                self.queue.set(previous);
            }
            return Err(error);
        }
        let mut pending_end = None;
        self.queue.update(|queue| {
            if let Some(queue) = queue.as_mut().filter(|queue| queue.request == request) {
                queue.accepted = true;
                pending_end = queue.pending_end.take();
            }
        });
        if let Some(id) = pending_end {
            self.finish_queue(id);
        }
        Ok(())
    }

    /// Called only by the existing explicit end event, never an Idle inference.
    pub(super) fn finish_queue(self, id: String) {
        let snapshot = self.media.with(|media| media.audio_playback_snapshot());
        let mut continuation = None;
        self.queue.update(|queue| {
            let Some(queue) = queue.as_mut() else { return };
            if queue.request != self.request.get_untracked()
                || queue
                    .tail
                    .as_ref()
                    .is_none_or(|child| queue.scope.item_key(child) != id)
            {
                return;
            }
            if !queue.accepted {
                // A very short clip can end before the play future is observed.
                // Reconcile that explicit event after accepting this request.
                queue.pending_end = Some(id.clone());
                return;
            }
            if snapshot.status != AudioPlaybackStatus::Idle
                || snapshot.item.as_ref().is_none_or(|item| item.id != id)
            {
                return;
            }
            continuation = queue
                .tail
                .take()
                .map(|tail| (queue.scope.clone(), tail, queue.request));
        });
        let Some((scope, anchor, request)) = continuation else {
            return;
        };
        let account = self.account();
        self.owner.run(move || {
            spawn(async move {
                let query_scope = scope.clone();
                let query_anchor = anchor.clone();
                let result = runtime::run(async move {
                    load(account, query_scope, query_anchor, PAGE_SIZE as i64).await
                })
                .await;
                if self.request.get_untracked() != request {
                    return;
                }
                let snapshot = self.media.with(|media| media.audio_playback_snapshot());
                if snapshot.status != AudioPlaybackStatus::Idle
                    || snapshot.item.as_ref().is_none_or(|item| item.id != id)
                {
                    return;
                }
                let result = match result {
                    Ok(mut messages)
                        if messages.first().is_some_and(|row| row.message_id == anchor) =>
                    {
                        messages.remove(0);
                        if messages.is_empty() {
                            return;
                        }
                        let next = messages[0].message_id.clone();
                        self.play_rows(messages, next, scope, request).await
                    }
                    Ok(_) => return,
                    Err(error) => Err(error),
                };
                if self.request.get_untracked() == request {
                    self.failed.set(result.is_err());
                }
            })
        });
    }
}
