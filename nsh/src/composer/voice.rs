use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus, MediaClient,
    MessageListView, VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus,
};
use nsh_core::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

mod surface;

use super::{clear_quote, media_client, quote_id, tr};
use crate::{
    i18n::{self, Key},
    runtime,
};

#[derive(Clone, Copy)]
pub(crate) struct Voice {
    // Commands start under the surface branch for the current status, and a
    // status event can dispose that branch before their result arrives.
    owner: Owner,
    media: Stored<Arc<MediaClient>>,
    playback_context: crate::playback::Playback,
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    session: Stored<Arc<AtomicU64>>,
    disposed: Stored<Arc<AtomicBool>>,
    snapshot: Signal<VoiceRecorderSnapshot>,
    playback: Signal<AudioPlaybackSnapshot>,
    open: Signal<bool>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    quote: Signal<Option<MessageListView>>,
    discarding: Signal<bool>,
    anchor: ElementRef,
    bounds: Signal<Option<Measurement>>,
    measuring: Signal<u64>,
}
fn idle() -> VoiceRecorderSnapshot {
    VoiceRecorderSnapshot {
        session_id: None,
        status: VoiceRecorderStatus::Idle,
        recording: None,
        duration_millis: 0,
        waveform: Vec::new(),
    }
}

impl Voice {
    fn active(self) -> bool {
        self.open.get()
    }

    fn task(self, work: impl std::future::Future<Output = ()> + 'static) {
        self.owner.run(|| {
            spawn(work);
        });
    }

    fn measure(self) {
        self.measuring
            .update(|version| *version = version.wrapping_add(1));
        let version = self.measuring.get_untracked();
        after_answer(move || {
            self.task(async move {
                let measured = self.anchor.measure().await;
                if self.measuring.get_untracked() == version {
                    self.bounds.set(measured);
                }
            });
        });
    }

    fn request_discard(self) {
        if self.discarding.get_untracked()
            || self.busy.get_untracked()
            || !self.open.get_untracked()
            || self
                .snapshot
                .with_untracked(|s| s.status != VoiceRecorderStatus::Recording)
        {
            return;
        }
        let id = self.session.with(|id| id.load(Ordering::Acquire));
        let i18n = i18n::current();
        self.discarding.set(true);
        self.task(async move {
            let result = nsh_base::alert_dialog()
                .title(tr(i18n, "Discard recording?", "放弃录音？"))
                .description(tr(
                    i18n,
                    "This recording will be deleted.",
                    "这段录音将被删除。",
                ))
                .cancel("cancel", i18n.text(Key::Cancel))
                .destructive("discard", tr(i18n, "Discard", "放弃"))
                .show()
                .await;
            self.discarding.set(false);
            // A maximum-duration stop can turn capture into a preview while
            // the native confirmation is open. Never discard that newer state.
            if result.is_ok_and(|answer| answer.action.as_deref() == Some("discard"))
                && self.session.with(|current| current.load(Ordering::Acquire)) == id
                && self
                    .snapshot
                    .with_untracked(|s| s.status == VoiceRecorderStatus::Recording)
                && !self.busy.get_untracked()
            {
                self.cancel();
            }
        });
    }

    fn clear(self) {
        // Release the visible recording when its surface finishes dismissing.
        self.session.with(|id| id.store(0, Ordering::Release));
        self.open.set(false);
    }
    fn start(self) {
        if self.busy.get_untracked() {
            return;
        }
        let preview = self
            .snapshot
            .with(|s| s.recording.as_ref().map(|r| r.path.clone()));
        self.snapshot.set(idle());
        let media = self.media.with(|m| m.clone());
        let session = self.session.with(|s| s.clone());
        let disposed = self.disposed.with(|d| d.clone());
        self.busy.set(true);
        self.failed.set(false);
        self.open.set(true);
        self.task(async move {
            let result = runtime::commit(async move {
                stop_preview(&media, preview.as_deref()).await?;
                let id = media.start_voice_session().await?;
                session.store(id, Ordering::Release);
                if disposed.load(Ordering::Acquire) {
                    media.cancel_voice_session(id).await?;
                    return Ok(None);
                }
                Ok(Some(media.voice_recorder_snapshot()))
            })
            .await;
            match result {
                Ok(Some(snapshot)) => {
                    self.snapshot.set(snapshot);
                }
                Ok(None) => self.clear(),
                Err(_) => {
                    self.failed.set(true);
                    self.open.set(false);
                }
            }
            self.busy.set(false);
        });
    }
    fn stop(self, send: bool) {
        if !self.open.get_untracked() || self.busy.get_untracked() {
            return;
        }
        let id = self.session.with(|id| id.load(Ordering::Acquire));
        let media = self.media.with(|m| m.clone());
        let disposed = self.disposed.with(|d| d.clone());
        self.busy.set(true);
        self.task(async move {
            let result = runtime::commit(async move {
                let recording = media.stop_voice_session(id).await?;
                if disposed.load(Ordering::Acquire) {
                    media.cancel_voice_session(id).await?;
                }
                Ok(recording)
            })
            .await;
            match result {
                Ok(recording) => {
                    self.snapshot.update(|snapshot| {
                        snapshot.session_id = Some(id);
                        snapshot.status = VoiceRecorderStatus::Recorded;
                        snapshot.duration_millis = recording.duration_millis;
                        snapshot.waveform = recording.waveform.clone();
                        snapshot.recording = Some(recording);
                    });
                    self.busy.set(false);
                    if send {
                        self.send();
                    }
                    return;
                }
                Err(_) => {
                    let snapshot = self.media.with(|media| media.voice_recorder_snapshot());
                    if snapshot.session_id.is_none_or(|current| current == id) {
                        self.snapshot.set(snapshot);
                    }
                    self.failed.set(true);
                }
            }
            self.busy.set(false);
        });
    }
    fn cancel(self) {
        if !self.open.get_untracked() || self.busy.get_untracked() {
            return;
        }
        let id = self.session.with(|id| id.load(Ordering::Acquire));
        let media = self.media.with(|m| m.clone());
        let preview = self
            .snapshot
            .with(|s| s.recording.as_ref().map(|r| r.path.clone()));
        self.busy.set(true);
        self.task(async move {
            self.failed.set(
                runtime::commit(async move {
                    let stopping = stop_preview(&media, preview.as_deref()).await;
                    let cancelling = media.cancel_voice_session(id).await;
                    stopping?;
                    cancelling
                })
                .await
                .is_err(),
            );
            self.clear();
            self.busy.set(false);
        });
    }
    fn play(self) {
        if !self.open.get_untracked() || self.busy.get_untracked() {
            return;
        }
        let Some(recording) = self.snapshot.with(|s| s.recording.clone()) else {
            return;
        };
        let media = self.media.with(|m| m.clone());
        let current = self.playback.get_untracked();
        self.playback_context.cancel_pending();
        let disposed = self.disposed.with(|d| d.clone());
        self.task(async move {
            self.failed.set(
                runtime::run(async move {
                    if disposed.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    let result = if current
                        .item
                        .as_ref()
                        .is_some_and(|item| item.id == recording.path)
                        && current.status != AudioPlaybackStatus::Idle
                    {
                        if current.status == AudioPlaybackStatus::Playing {
                            media.pause_audio().await
                        } else {
                            media.resume_audio().await
                        }
                    } else {
                        media
                            .play_audio(
                                vec![AudioPlaybackItem {
                                    id: recording.path.clone(),
                                    path: recording.path.clone(),
                                    duration_millis: recording.duration_millis,
                                }],
                                0,
                            )
                            .await
                    };
                    if disposed.load(Ordering::Acquire) {
                        stop_preview(&media, Some(&recording.path)).await?;
                    }
                    result
                })
                .await
                .is_err(),
            );
        });
    }
    fn send(self) {
        if !self.open.get_untracked() || self.busy.get_untracked() {
            return;
        }
        let Some(recording) = self.snapshot.with(|s| s.recording.clone()) else {
            return;
        };
        let media = self.media.with(|m| m.clone());
        let client = self.account.with(|a| a.clone());
        let conversation = self.conversation.with(|id| id.clone());
        let sent_quote = quote_id(self.quote);
        let quote = sent_quote.clone();
        self.busy.set(true);
        self.failed.set(false);
        self.task(async move {
            let result = runtime::commit(async move {
                // The click already holds the immutable source lease. Cancelling
                // this session releases capture without deleting the leased file.
                media.cancel_voice_session(recording.session_id).await?;
                stop_preview(&media, Some(&recording.path)).await?;
                let result = client
                    .message()
                    .send_audio_receipt(
                        conversation,
                        recording.path.clone(),
                        recording.duration_millis as i64,
                        recording.waveform.clone(),
                        quote,
                    )
                    .await;
                drop(recording);
                result
            })
            .await;
            self.busy.set(false);
            match result {
                Ok(receipt) => {
                    self.failed.set(receipt.upload_error.is_some());
                    clear_quote(self.quote, sent_quote);
                    self.clear();
                }
                Err(_) => self.failed.set(true),
            }
        });
    }
}

async fn stop_preview(
    media: &MediaClient,
    path: Option<&str>,
) -> mixin_desktop_api::ClientResult<()> {
    let current = media.audio_playback_snapshot();
    if current
        .item
        .as_ref()
        .is_some_and(|item| Some(item.id.as_str()) == path)
    {
        media.stop_audio().await?;
    }
    Ok(())
}

impl Voice {
    pub(crate) fn mount(
        account: Arc<AccountClient>,
        conversation: String,
        failed: Signal<bool>,
        quote: Signal<Option<MessageListView>>,
    ) -> Self {
        let media = media_client();
        let session = Arc::new(AtomicU64::new(0));
        let disposed = Arc::new(AtomicBool::new(false));
        let playback_context =
            use_context::<crate::playback::Playback>().expect("Home provides playback");
        let voice = Voice {
            owner: Owner::current(),
            media: stored(media.clone()),
            playback_context,
            account: stored(account),
            conversation: stored(conversation),
            session: stored(session.clone()),
            disposed: stored(disposed.clone()),
            snapshot: signal(idle()),
            playback: playback_context.snapshot_signal(),
            open: signal(false),
            busy: signal(false),
            failed,
            quote,
            discarding: signal(false),
            anchor: element_ref(),
            bounds: signal(None),
            measuring: signal(0),
        };
        let observe = media.clone();
        spawn(async move {
            let mut events = Box::pin(runtime::observe(observe.subscribe_voice_recorder()));
            while let Some(event) = events.next().await {
                let current = voice.session.with(|s| s.load(Ordering::Acquire));
                match event {
                    VoiceRecorderEvent::Changed(value)
                        if value.session_id == Some(current) && current != 0 =>
                    {
                        voice.snapshot.set(value)
                    }
                    VoiceRecorderEvent::Failed(_)
                        if current != 0
                            && observe
                                .voice_recorder_snapshot()
                                .session_id
                                .is_none_or(|id| id == current) =>
                    {
                        // Device errors do not complete an in-flight stop/send.
                        // Only its result may re-enable these commands.
                        if !voice.busy.get_untracked() {
                            voice.snapshot.set(observe.voice_recorder_snapshot());
                        }
                        voice.failed.set(true);
                    }
                    _ => {}
                }
            }
        });
        cleanup(move || {
            disposed.store(true, Ordering::Release);
            let id = session.swap(0, Ordering::AcqRel);
            // Signals are already disposed with the owner here; ask the recorder.
            let preview = media
                .voice_recorder_snapshot()
                .recording
                .as_ref()
                .map(|r| r.path.clone());
            drop(runtime::commit(async move {
                let stopping = stop_preview(&media, preview.as_deref()).await;
                let cancelling = media.cancel_voice_session(id).await;
                stopping?;
                cancelling
            }));
        });
        on_back_press(move || {
            if !voice.open.get_untracked()
                || voice.busy.get_untracked()
                || voice
                    .snapshot
                    .with_untracked(|snapshot| snapshot.status == VoiceRecorderStatus::Recording)
            {
                return false;
            }
            voice.cancel();
            true
        });
        voice
    }
}
