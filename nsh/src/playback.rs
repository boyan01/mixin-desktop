//! Account-session playback presentation and controls shared across chat navigation.
use std::sync::Arc;

mod queue;

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, AudioPlaybackEvent, AudioPlaybackSnapshot, AudioPlaybackStatus, ClientError,
    ClientResult, MediaClient, MessageListView,
};
use nsh_base::SliderValue;
use nsh_core::prelude::*;

use crate::{
    navigation::ChatNavigation,
    runtime,
    ui::{self, Typography},
};

/// The playback speeds on offer, with their menu labels.
pub(crate) const SPEEDS: [(f64, &str); 3] = [(1.0, "1×"), (1.5, "1.5×"), (2.0, "2×")];

#[derive(Clone, Copy)]
pub(crate) struct Playback {
    /// `None` only in the debug preview, which plays nothing.
    account: Stored<Option<Arc<AccountClient>>>,
    media: Stored<Arc<MediaClient>>,
    snapshot: Signal<AudioPlaybackSnapshot>,
    owner: Owner,
    request: Signal<u64>,
    row: Signal<Option<MessageListView>>,
    title: Signal<String>,
    failed: Signal<bool>,
    queue: Signal<Option<queue::Queue>>,
}

impl Playback {
    /// Idle playback over `media`, for the debug preview: no account, no queue.
    #[cfg(debug_assertions)]
    pub(crate) fn detached(media: Arc<MediaClient>) -> Self {
        Self::new(None, media)
    }

    fn new(account: Option<Arc<AccountClient>>, media: Arc<MediaClient>) -> Self {
        Self {
            account: stored(account),
            snapshot: signal_with(media.audio_playback_snapshot(), SignalEquals::Never),
            media: stored(media),
            owner: Owner::current(),
            request: signal(0),
            row: signal_with(None, SignalEquals::Never),
            title: signal(String::new()),
            failed: signal(false),
            queue: signal_with(None, SignalEquals::Never),
        }
    }

    fn account(self) -> Arc<AccountClient> {
        self.account
            .with(|account| account.clone())
            .expect("the debug preview has no account")
    }

    pub(crate) fn snapshot_signal(self) -> Signal<AudioPlaybackSnapshot> {
        self.snapshot
    }

    pub(crate) fn cancel_pending(self) {
        self.request.update(|value| *value = value.wrapping_add(1));
    }

    /// Runs under the account owner: the bar that started it may unmount first.
    fn command(
        self,
        work: impl std::future::Future<Output = ClientResult<()>> + crate::runtime::MaybeSend + 'static,
    ) {
        self.owner.run(move || {
            spawn(async move { self.failed.set(runtime::run(work).await.is_err()) });
        });
    }

    pub(crate) fn stop(self) {
        self.cancel_pending();
        let request = self.request.get_untracked();
        let stopping = self.media.with(|media| media.stop_audio());
        self.owner.run(move || {
            spawn(async move {
                let result = runtime::run(stopping).await;
                if self.request.get_untracked() == request {
                    self.failed.set(result.is_err());
                }
            });
        });
    }

    /// The account owner keeps accepted playback alive across chat navigation.
    pub(crate) fn play_message(
        self,
        message: MessageListView,
        transcript: Option<String>,
        pinned_page: bool,
    ) -> impl std::future::Future<Output = ClientResult<()>> {
        self.cancel_pending();
        let request = self.request.get_untracked();
        let client = self.account();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.owner.run(move || {
            spawn(async move {
                let scope = match transcript {
                    Some(root) => queue::Scope::Transcript(root),
                    None if pinned_page => queue::Scope::Pinned(message.conversation_id.clone()),
                    None => queue::Scope::Conversation(message.conversation_id.clone()),
                };
                let query_scope = scope.clone();
                let anchor = message.message_id;
                let query_anchor = anchor.clone();
                let result = runtime::run(async move {
                    queue::load(client, query_scope, query_anchor, 99).await
                })
                .await;
                let result = match result {
                    Ok(messages) => self.play_rows(messages, anchor, scope, request).await,
                    Err(error) => Err(error),
                };
                if self.request.get_untracked() == request {
                    self.failed.set(result.is_err());
                }
                let _ = sender.send(result);
            });
        });
        async move {
            receiver
                .await
                .map_err(|_| ClientError::Internal("playback session was closed".into()))?
        }
    }
}

pub(crate) fn mount(account: Arc<AccountClient>) -> Playback {
    let media = Arc::new(MediaClient::new());
    // Descendant chat and composer controls share this account session only.
    provide_context(stored(Some(media.clone())));
    let state = Playback::new(Some(account.clone()), media.clone());
    let cleanup_media = media.clone();
    cleanup(move || {
        // Closing invalidates synchronously; device cleanup stays on the runtime.
        drop(runtime::commit(cleanup_media.close()));
    });
    spawn(async move {
        let mut events = Box::pin(runtime::observe(media.subscribe_audio_playback()));
        while let Some(event) = events.next().await {
            match event {
                AudioPlaybackEvent::Changed(snapshot) => state.snapshot.set(snapshot),
                AudioPlaybackEvent::Failed { .. } => state.failed.set(true),
                AudioPlaybackEvent::Finished(id) => state.finish_queue(id),
            }
        }
    });
    let item = computed(move || {
        state
            .snapshot
            .with(|snapshot| snapshot.item.as_ref().map(|item| item.id.clone()))
    });
    let query_account = account.clone();
    effect(move || {
        let id = item.get();
        state.row.set(None);
        state.title.set(String::new());
        let Some(id) = id else {
            return;
        };
        if id.starts_with("transcript:") || std::path::Path::new(&id).is_absolute() {
            return;
        }
        let client = query_account.clone();
        // A newer item reruns this effect, which cancels this query.
        spawn(async move {
            let result = runtime::run(async move {
                let message = client
                    .message()
                    .message_items_by_ids(vec![id])
                    .await?
                    .into_iter()
                    .find(|row| row.category.ends_with("_AUDIO"));
                let title = if let Some(message) = &message {
                    client
                        .conversation()
                        .conversation_items_by_ids(vec![message.conversation_id.clone()])
                        .await?
                        .into_iter()
                        .next()
                        .map(|row| row.name)
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                Ok((message, title))
            })
            .await;
            match result {
                Ok((row, title)) => {
                    state.row.set(row);
                    state.title.set(title);
                }
                Err(_) => state.failed.set(true),
            }
        });
    });
    let read_account = account.clone();
    let playing = computed(move || {
        state.snapshot.with(|snapshot| {
            (snapshot.status == AudioPlaybackStatus::Playing)
                .then(|| snapshot.item.as_ref().map(|item| item.id.clone()))
                .flatten()
        })
    });
    let read_target = computed(move || {
        let id = playing.get()?;
        if let Some((root, child)) = state.queue.with(|queue| {
            queue
                .as_ref()
                .and_then(|queue| queue.transcript_source(&id))
        }) {
            Some((id, Some(root), child))
        } else {
            state.row.with(|row| {
                row.as_ref()
                    .filter(|row| row.message_id == id)
                    .map(|_| (id.clone(), None, id.clone()))
            })
        }
    });
    effect(move || {
        let Some((key, root, message_id)) = read_target.get() else {
            return;
        };
        let client = read_account.clone();
        // Capture this accepted item's identity. A later queue cannot retarget
        // the durable acknowledgement or make an old failure affect a new item.
        spawn(async move {
            let result = runtime::commit(async move {
                if let Some(root) = root {
                    client
                        .attachment()
                        .mark_transcript_audio_read(root, message_id)
                        .await
                } else {
                    client.attachment().mark_audio_read(message_id).await
                }
            })
            .await;
            if result.is_err()
                && read_target
                    .get_untracked()
                    .is_some_and(|(current, _, _)| current == key)
            {
                state.failed.set(true);
            }
        });
    });
    state
}

fn tr(en: &'static str, zh: &'static str) -> &'static str {
    crate::i18n::tr(en, zh)
}

pub(crate) fn bar(
    state: Playback,
    selected: Signal<Option<mixin_desktop_api::ConversationListData>>,
) -> impl IntoElement {
    let navigation = use_context::<ChatNavigation>().expect("Home provides navigation");
    view().flex_none().min_w_0().child(when(
        move || {
            state
                .snapshot
                .with(|s| s.status != AudioPlaybackStatus::Idle)
                && state.row.with(|row| {
                    row.as_ref().is_some_and(|row| {
                        !selected.with(|selected| {
                            selected
                                .as_ref()
                                .is_some_and(|s| s.conversation_id == row.conversation_id)
                        })
                    })
                })
        },
        move || {
            view()
                .min_w_0()
                .border_t_1()
                .border_color(ui::border)
                .p_3()
                .gap_2()
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .child(
                            ui::command()
                                .flex_1()
                                .min_w_20()
                                .justify_start()
                                .on_press(move || {
                                    if let Some(row) = state.row.get_untracked() {
                                        navigation.open_conversation(
                                            state.account(),
                                            row.conversation_id,
                                            Some(row.message_id),
                                            state.failed,
                                        );
                                    }
                                })
                                .child(
                                    text(move || state.title.get())
                                        .text_role(ui::TextRole::Body)
                                        .truncate(),
                                ),
                        )
                        .child(
                            ui::menu()
                                .trigger(
                                    ui::command()
                                        .accessibility_label(|| {
                                            Some(tr("Playback speed", "播放速度").into())
                                        })
                                        .child(
                                            text(move || {
                                                format!(
                                                    "{}×",
                                                    state.snapshot.with(|snapshot| snapshot.speed)
                                                )
                                            })
                                            .text_role(ui::TextRole::Caption),
                                        ),
                                )
                                .children(SPEEDS.into_iter().map(move |(speed, label)| {
                                    ui::menu_row(move || label).on_select(move || {
                                        let media = state.media.with(|media| media.clone());
                                        state.command(
                                            async move { media.set_audio_speed(speed).await },
                                        );
                                    })
                                })),
                        )
                        .child(
                            ui::command()
                                .on_press(move || {
                                    state.cancel_pending();
                                    state.keep_accepted_queue();
                                    let media = state.media.with(|value| value.clone());
                                    let playing = state
                                        .snapshot
                                        .with(|s| s.status == AudioPlaybackStatus::Playing);
                                    state.command(async move {
                                        if playing {
                                            media.pause_audio().await
                                        } else {
                                            media.resume_audio().await
                                        }
                                    });
                                })
                                .child(
                                    text(move || {
                                        if state
                                            .snapshot
                                            .with(|s| s.status == AudioPlaybackStatus::Playing)
                                        {
                                            tr("Pause", "暂停")
                                        } else {
                                            tr("Play", "播放")
                                        }
                                    })
                                    .text_role(ui::TextRole::Caption),
                                ),
                        )
                        .child(ui::command().on_press(move || state.stop()).child(
                            text(move || tr("Close", "关闭")).text_role(ui::TextRole::Caption),
                        )),
                )
                .child(
                    ui::slider_compact()
                        .min(0.0)
                        .max(move || state.snapshot.with(|s| s.duration_millis.max(1) as f64))
                        .step(100.0)
                        .value(move || {
                            Some(SliderValue::Single(
                                state.snapshot.with(|s| s.position_millis as f64),
                            ))
                        })
                        .accessibility_label(|| {
                            Some(tr("Voice message position", "语音播放进度").into())
                        })
                        .disabled(move || state.snapshot.with(|s| s.duration_millis == 0))
                        .on_change(move |value: &SliderValue| {
                            if let SliderValue::Single(value) = value {
                                state.media.with(|m| m.seek_audio(value.max(0.0) as u64));
                            }
                        }),
                )
                .child(
                    text(move || {
                        state.snapshot.with(|snapshot| {
                            format!(
                                "{}:{:02} / {}:{:02}",
                                snapshot.position_millis / 60_000,
                                (snapshot.position_millis / 1000) % 60,
                                snapshot.duration_millis / 60_000,
                                (snapshot.duration_millis / 1000) % 60,
                            )
                        })
                    })
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::fg_secondary),
                )
                .child(when(state.failed, || {
                    ui::error_text(move || tr("Playback failed", "播放失败"))
                }))
        },
    ))
}
