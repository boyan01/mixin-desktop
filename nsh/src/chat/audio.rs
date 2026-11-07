use base64::{Engine as _, engine::general_purpose::STANDARD};
use mixin_desktop_api::{AudioPlaybackStatus, MessageListView};
use nsh_base::SliderValue;
use nsh_core::{TransformKind, animation::Easing, element::ImageSrc, prelude::*};

use super::{message_actions::Actions, message_appearance};
use crate::{i18n::Key, runtime, ui};

/// Flutter's most bars in a waveform, which fill `audio_wave_width`.
const BARS: usize = 60;

/// Flutter's fold: each bar is the loudest sample of its share of the recording.
fn bars(samples: &[u8]) -> [u8; BARS] {
    let mut bars = [0; BARS];
    for (index, sample) in samples.iter().enumerate() {
        let bar = index * BARS / samples.len();
        bars[bar] = bars[bar].max(*sample);
    }
    bars
}

pub(super) fn body(
    message: MessageListView,
    transcript: Option<String>,
    actions: Actions,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let playback = use_context::<crate::playback::Playback>().expect("Home provides playback");
    let mine = message.sender_id == actions.account.with(|account| account.account_id());
    // A voice message waits in the accent color until it is heard.
    let heard = mine || message.media_status == "READ";
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
    let waveform = bars(&waveform);
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
    let playing = move || {
        active()
            && actions
                .playback
                .with(|p| p.status == AudioPlaybackStatus::Playing)
    };
    let scale = ui::audio_wave_height().0;
    let strip = move |lit: bool| {
        view()
            .absolute()
            .inset_0()
            .flex_row()
            .items_end()
            .gap(ui::space_2xs())
            .overflow_hidden()
            .children(waveform.into_iter().map(move |level| {
                view()
                    .w(ui::space_2xs())
                    .flex_shrink_0()
                    // A bar is as tall as its sample's level, within the strip.
                    .h(px((f32::from(level) / 255.0 * scale).max(ui::space_2xs().0)))
                    .rounded_t(ui::hairline_width())
                    .bg(move || {
                        if lit {
                            ui::waveform_progress()
                        } else if heard {
                            ui::waveform_track()
                        } else {
                            ui::accent()
                        }
                    })
            }))
    };
    // The played part is a blue strip under a grey one, which a window uncovers from the left
    // edge. The window and the grey strip move by opposite amounts on the native clock, so the
    // edge glides between the player's reports instead of stepping with them.
    let width = signal(ui::audio_wave_width().0);
    let shift = animated_value(0.0);
    let unshift = animated_value(0.0);
    let run = stored(None::<(std::time::Instant, u64, f64, u64, f32)>);
    effect(move || {
        let (active, playing, position, duration, speed, width) = (
            active(),
            playing(),
            position(),
            duration(),
            actions.playback.with(|p| p.speed),
            width.get(),
        );
        untracked(move || {
            let place = |at: f32| {
                shift.set(at);
                unshift.set(-at);
            };
            if !active || duration == 0 {
                run.replace(None);
                return place(0.0);
            }
            let at = position as f32 / duration as f32 * width;
            if !playing {
                run.replace(None);
                return place(at);
            }
            let current = run.get();
            let keeps = current.is_some_and(|(since, from, rate, total, wide)| {
                let expected = from as f64 + since.elapsed().as_millis() as f64 * rate;
                total == duration
                    && wide == width
                    && rate == speed
                    && (expected - position as f64).abs() < 400.0
            });
            if keeps {
                return;
            }
            run.replace(Some((
                std::time::Instant::now(),
                position,
                speed,
                duration,
                width,
            )));
            place(at);
            let ms = (duration - position.min(duration)) as f32 / speed.max(0.1) as f32;
            let _ = shift.timing(width, ms, Easing::Linear);
            let _ = unshift.timing(-width, ms, Easing::Linear);
        });
    });
    let wave = view()
        .relative()
        .w(ui::audio_wave_width())
        .max_w(relative(1.0))
        .h(ui::audio_wave_height())
        .on_layout(move |bounds: &Bounds<Points>| width.set(bounds.size.width.0.max(1.0)))
        .child(when(move || heard, move || strip(true)))
        .child(
            view().absolute().inset_0().overflow_hidden().child(
                view()
                    .absolute()
                    .inset_0()
                    .overflow_hidden()
                    .bind_transform(TransformKind::TranslateX, &shift)
                    .child(strip(false).bind_transform(TransformKind::TranslateX, &unshift)),
            ),
        )
        .child(when(active, move || {
            ui::slider_over_wave()
                .absolute()
                .inset_0()
                .min(0.0)
                .max(move || duration().max(1) as f64)
                .step(100.0)
                .value(move || Some(SliderValue::Single(position() as f64)))
                .disabled(move || duration() == 0)
                .accessibility_label(move || {
                    Some(i18n.tr("Voice message position", "语音播放进度").into())
                })
                .on_change(move |value: &SliderValue| {
                    if let SliderValue::Single(value) = *value
                        && active()
                    {
                        actions.media.with(|m| m.seek_audio(value.max(0.0) as u64));
                    }
                })
        }));
    let glyph = |source: &'static str| {
        svg(ImageSrc::svg_markup(source))
            .size(ui::audio_icon())
            .text_color(ui::accent)
            .accessibility_elements_hidden(true)
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .min_w_0()
        .max_w(relative(1.0))
        .child(
            message_appearance::media_disc().child(
                ui::round_button()
                    .size(ui::status_disc())
                    .disabled(busy)
                    .accessibility_label(move || {
                        Some(
                            if playing() {
                                i18n.tr("Stop", "停止")
                            } else {
                                i18n.text(Key::Play)
                            }
                            .into(),
                        )
                    })
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
                    .child(when(playing, move || {
                        glyph(include_str!("../../assets/message/audio_stop.svg"))
                    }))
                    .child(when(
                        move || !playing(),
                        move || glyph(include_str!("../../assets/message/audio_play.svg")),
                    )),
            ),
        )
        .child(
            view().min_w_0().flex_shrink(1.0).gap_2().child(wave).child(
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
                        .text_size(move || appearance.tertiary_size())
                        .text_color(ui::message_secondary)
                        .flex_1(),
                    )
                    .child(when(active, move || {
                        ui::menu()
                            .trigger(
                                ui::command()
                                    .min_h_0()
                                    .px_1()
                                    .py_0()
                                    .disabled(busy)
                                    .accessibility_label(move || {
                                        Some(i18n.tr("Playback speed", "播放速度").into())
                                    })
                                    .child(
                                        text(move || {
                                            format!("{}×", actions.playback.with(|p| p.speed))
                                        })
                                        .text_size(move || appearance.tertiary_size())
                                        .text_color(ui::message_secondary),
                                    ),
                            )
                            .children(crate::playback::SPEEDS.into_iter().map(
                                move |(speed, label)| {
                                    ui::menu_row(move || label).on_select(move || {
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
                                },
                            ))
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
