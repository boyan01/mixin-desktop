//! Account-session playback presentation and controls shared across chat navigation.
use std::sync::Arc;

mod queue;

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, AudioPlaybackEvent, AudioPlaybackSnapshot, AudioPlaybackStatus, ClientError,
    ClientResult, MediaClient, MessageListView,
};
use nsh_base::{SliderValue, menu, menu_item, slider, theme};
use nsh_core::prelude::*;

use crate::{navigation::ChatNavigation, runtime, ui};

#[derive(Clone, Copy)]
pub(crate) struct Playback {
    account: Stored<Arc<AccountClient>>,
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
    pub(crate) fn snapshot_signal(self) -> Signal<AudioPlaybackSnapshot> {
        self.snapshot
    }

    pub(crate) fn cancel_pending(self) {
        self.request.update(|value| *value = value.wrapping_add(1));
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
        let client = self.account.with(|client| client.clone());
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
    let state = Playback {
        account: stored(account.clone()),
        snapshot: signal_with(media.audio_playback_snapshot(), SignalEquals::Never),
        media: stored(media.clone()),
        owner: Owner::current(),
        request: signal(0),
        row: signal_with(None, SignalEquals::Never),
        title: signal(String::new()),
        failed: signal(false),
        queue: signal_with(None, SignalEquals::Never),
    };
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
        spawn(async move {
            let requested_id = id.clone();
            let result = runtime::run(async move {
                let message = client
                    .message()
                    .message_items_by_ids(vec![requested_id])
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
            if item.get_untracked().as_deref() != Some(&id) {
                return;
            }
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
            state.row.with(Option::is_some)
                && state
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
                .border_color(move || theme().border)
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
                                .min_w(px(80.0))
                                .justify_start()
                                .on_press(move || {
                                    if let Some(row) = state.row.get_untracked() {
                                        navigation.open_conversation(
                                            state.account.with(|value| value.clone()),
                                            row.conversation_id,
                                            Some(row.message_id),
                                            state.failed,
                                        );
                                    }
                                })
                                .child(text(move || state.title.get()).text_sm().truncate()),
                        )
                        .child(
                            menu()
                                .popup_style(ui::popup)
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
                                            .text_xs(),
                                        ),
                                )
                                .children([1.0, 1.5, 2.0].into_iter().map(move |speed| {
                                    menu_item()
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .hover(|s| s.bg(move || theme().accent))
                                        .child(text(format!("{speed}×")).text_sm())
                                        .on_select(move || {
                                            let media = state.media.with(|media| media.clone());
                                            state.owner.run(move || {
                                                spawn(async move {
                                                    state.failed.set(
                                                        runtime::run(async move {
                                                            media.set_audio_speed(speed).await
                                                        })
                                                        .await
                                                        .is_err(),
                                                    );
                                                });
                                            });
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
                                    spawn(async move {
                                        state.failed.set(
                                            runtime::run(async move {
                                                if playing {
                                                    media.pause_audio().await
                                                } else {
                                                    media.resume_audio().await
                                                }
                                            })
                                            .await
                                            .is_err(),
                                        );
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
                                    .text_xs(),
                                ),
                        )
                        .child(
                            ui::command()
                                .on_press(move || state.stop())
                                .child(text(move || tr("Close", "关闭")).text_xs()),
                        ),
                )
                .child(
                    slider()
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
                        .track_class("h-6 justify-center disabled:opacity-40")
                        .indicator_class("h-1 rounded-full bg-muted-foreground/30")
                        .range_class("rounded-full bg-primary")
                        .thumb_class(
                            "size-3 rounded-full bg-primary focus:outline-2 focus:outline-ring",
                        )
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
                    .text_xs()
                    .text_color(move || theme().muted_foreground),
                )
                .child(when(state.failed, || {
                    text(move || tr("Playback failed", "播放失败"))
                        .text_xs()
                        .text_color(move || theme().destructive)
                }))
        },
    ))
}
