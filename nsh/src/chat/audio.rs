use base64::{Engine as _, engine::general_purpose::STANDARD};
use mixin_desktop_api::{AudioPlaybackStatus, MessageListView};
use nsh_base::{SliderValue, menu, menu_item, slider, theme};
use nsh_core::prelude::*;

use super::message_actions::Actions;
use crate::{i18n::Key, runtime, ui};

pub(super) fn body(
    message: MessageListView,
    transcript: Option<String>,
    actions: Actions,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let playback = use_context::<crate::playback::Playback>().expect("Home provides playback");
    let playback_key = stored(
        transcript
            .as_ref()
            .map(|id| format!("transcript:{id}:{}", message.message_id))
            .unwrap_or_else(|| message.message_id.clone()),
    );
    let waveform = message
        .media_waveform
        .as_deref()
        .and_then(|s| STANDARD.decode(s).ok())
        .unwrap_or_default();
    let waveform = stored(waveform);
    let row = stored(message);
    let active = move || {
        actions.playback.with(|p| {
            p.item
                .as_ref()
                .is_some_and(|item| playback_key.with(|key| item.id == *key))
        })
    };
    let duration = move || {
        if active() {
            actions.playback.with(|p| p.duration_millis)
        } else {
            row.with(|row| row.media_duration.parse::<u64>().unwrap_or_default())
        }
    };
    let position = move || {
        if active() {
            actions.playback.with(|p| p.position_millis)
        } else {
            0
        }
    };
    let busy = signal(false);
    view()
        .w(px(216.0))
        .max_w(relative(1.0))
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    ui::command()
                        .disabled(busy)
                        .on_press(move || {
                            play(
                                row.with(|row| row.clone()),
                                transcript.clone(),
                                playback_key.with(|key| key.clone()),
                                playback,
                                actions,
                                busy,
                            )
                        })
                        .child(
                            text(move || {
                                if active()
                                    && actions
                                        .playback
                                        .with(|p| p.status == AudioPlaybackStatus::Playing)
                                {
                                    i18n.tr("Stop", "停止")
                                } else {
                                    i18n.text(Key::Play)
                                }
                            })
                            .text_sm(),
                        ),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .h_8()
                        .children((0..16).map(move |index| {
                            view()
                                .w(px(2.0))
                                .rounded_full()
                                .bg(move || {
                                    if duration() > 0
                                        && (index as f64 / 16.0)
                                            < position() as f64 / duration() as f64
                                    {
                                        theme().primary
                                    } else {
                                        theme().muted_foreground.opacity(0.5)
                                    }
                                })
                                .h(move || {
                                    px(2.0
                                        + waveform.with(|waveform| {
                                            let start = index * waveform.len() / 16;
                                            let end = ((index + 1) * waveform.len() / 16)
                                                .max(start + 1)
                                                .min(waveform.len());
                                            f32::from(
                                                waveform
                                                    .get(start..end)
                                                    .and_then(|s| s.iter().max())
                                                    .copied()
                                                    .unwrap_or_default(),
                                            ) / 255.0
                                                * 28.0
                                        }))
                                })
                        })),
                ),
        )
        .child(
            slider()
                .min(0.0)
                .max(move || duration().max(1) as f64)
                .step(100.0)
                .value(move || Some(SliderValue::Single(position() as f64)))
                .disabled(move || !active() || duration() == 0)
                .accessibility_label(move || {
                    Some(i18n.tr("Voice message position", "语音播放进度").into())
                })
                .track_class("h-6 justify-center disabled:opacity-40")
                .indicator_class("h-1 rounded-full bg-muted-foreground/30")
                .range_class("rounded-full bg-primary")
                .thumb_class("size-3 rounded-full bg-primary focus:outline-2 focus:outline-ring")
                .on_change(move |value: &SliderValue| {
                    if let SliderValue::Single(value) = *value
                        && active()
                    {
                        actions.media.with(|m| m.seek_audio(value.max(0.0) as u64));
                    }
                }),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || {
                        if active() {
                            format!("{} / {}", time(position()), time(duration()))
                        } else {
                            time(duration())
                        }
                    })
                    .text_xs()
                    .text_color(move || theme().muted_foreground)
                    .flex_1(),
                )
                .child(
                    menu()
                        .popup_style(ui::popup)
                        .trigger(
                            ui::command()
                                .min_h_6()
                                .px_2()
                                .py_1()
                                .disabled(busy)
                                .accessibility_label(move || {
                                    Some(i18n.tr("Playback speed", "播放速度").into())
                                })
                                .child(
                                    text(move || {
                                        format!("{}×", actions.playback.with(|p| p.speed))
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
                                    let media = actions.media.with(|m| m.clone());
                                    busy.set(true);
                                    spawn(async move {
                                        actions.failed.set(
                                            runtime::run(async move {
                                                media.set_audio_speed(speed).await
                                            })
                                            .await
                                            .is_err(),
                                        );
                                        busy.set(false);
                                    });
                                })
                        })),
                ),
        )
}
fn time(millis: u64) -> String {
    format!("{}:{:02}", millis / 60_000, (millis / 1000) % 60)
}

fn play(
    message: MessageListView,
    transcript: Option<String>,
    key: String,
    playback: crate::playback::Playback,
    actions: Actions,
    busy: Signal<bool>,
) {
    if busy.get_untracked() {
        return;
    }
    let current = actions.playback.get_untracked();
    let media = actions.media.with(|m| m.clone());
    busy.set(true);
    if current.item.as_ref().is_some_and(|item| item.id == key)
        && current.status == AudioPlaybackStatus::Playing
    {
        playback.cancel_pending();
        spawn(async move {
            actions
                .failed
                .set(runtime::run(media.stop_audio()).await.is_err());
            busy.set(false);
        });
        return;
    }
    let playing = playback.play_message(message, transcript, actions.pinned_page);
    spawn(async move {
        actions.failed.set(playing.await.is_err());
        busy.set(false);
    });
}
