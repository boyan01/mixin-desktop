use super::*;
use crate::{composer::tool_button, ui};
use nsh_base::theme;

impl Voice {
    pub(crate) fn composer(self, content: View) -> impl IntoElement {
        view()
            .relative()
            .element_ref(&self.anchor)
            .on_layout(move |_: &Bounds<Points>| self.measure())
            .child(content.style_when(move || self.active(), |style| style.invisible()))
            .child(self.surface())
    }

    pub(crate) fn button(self) -> impl IntoElement {
        let i18n = i18n::current();
        tool_button()
            .disabled(move || self.busy.get() || self.open.get())
            .accessibility_label(move || Some(i18n.text(Key::Record).into()))
            .child(ui::icon(ui::Icon::Microphone).size_6())
            .on_press(move || self.start())
    }

    fn surface(self) -> impl IntoElement {
        let voice = self;
        let presence = crate::motion::Presence::bottom(move || voice.open.get());
        when(
            move || presence.mounted(),
            move || {
                cleanup(move || {
                    if voice.open.try_get() == Some(false) {
                        let _ = voice.snapshot.try_set(idle());
                    }
                });
                view()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(when(
                        move || {
                            voice
                                .snapshot
                                .with(|s| s.status == VoiceRecorderStatus::Recording)
                                && voice.bounds.with(Option::is_some)
                        },
                        move || {
                            // Only capture intercepts outside interaction. The native
                            // surface keeps pointer/keyboard focus out of the composer.
                            nsh_base::dialog()
                                .animation(ModalAnimation::None)
                                .close_on_escape(false)
                                .close_on_backdrop_press(false)
                                .on_cancel(move || voice.request_discard())
                                .flex_1()
                                .child(
                                    view()
                                        .flex_1()
                                        .child(
                                            view()
                                                .absolute()
                                                .inset_0()
                                                .on_press(move || voice.request_discard()),
                                        )
                                        .child(
                                            presence.surface(
                                                view()
                                                    .absolute()
                                                    .left(move || {
                                                        px(voice.bounds.with(|b| {
                                                            b.map_or(0.0, |b| b.page.x.0)
                                                        }))
                                                    })
                                                    .w(move || {
                                                        px(voice.bounds.with(|b| {
                                                            b.map_or(0.0, |b| b.frame.size.width.0)
                                                        }))
                                                    })
                                                    .bottom(move || {
                                                        px(voice.bounds.with(|b| {
                                                            b.map_or(0.0, |b| {
                                                                (window().height.0
                                                                    - b.page.y.0
                                                                    - b.frame.size.height.0)
                                                                    .max(0.0)
                                                            })
                                                        }))
                                                    })
                                                    .child(bar(voice)),
                                            ),
                                        ),
                                )
                        },
                    ))
                    .child(when(
                        move || {
                            voice
                                .snapshot
                                .with(|s| s.status != VoiceRecorderStatus::Recording)
                        },
                        move || presence.surface(view().child(bar(voice))),
                    ))
            },
        )
    }
}

fn bar(voice: Voice) -> impl IntoElement {
    let i18n = i18n::current();
    let recording = move || {
        voice
            .snapshot
            .with(|s| s.status == VoiceRecorderStatus::Recording)
    };
    view()
        .min_h_14()
        .px_4()
        .py_2()
        .gap_1()
        .bg(move || theme().background)
        .border_t_1()
        .border_color(move || theme().border)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    tool_button()
                        .disabled(voice.busy)
                        .accessibility_label(move || Some(i18n.text(Key::Cancel).into()))
                        .child(ui::icon(ui::Icon::Close).size_5())
                        .on_press(move || voice.cancel()),
                )
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(when(recording, move || {
                            view()
                                .size_2()
                                .rounded_full()
                                .bg(move || theme().destructive)
                        }))
                        .child(
                            text(move || {
                                voice.snapshot.with(|s| {
                                    format!(
                                        "{:02}:{:02}",
                                        s.duration_millis / 60_000,
                                        (s.duration_millis / 1000) % 60
                                    )
                                })
                            })
                            .text_sm(),
                        )
                        .child(waveform(voice)),
                )
                .child(when(
                    move || voice.snapshot.with(|s| s.recording.is_some()),
                    move || {
                        ui::action(
                            move || {
                                let playing = voice.playback.with(|p| {
                                    p.status == AudioPlaybackStatus::Playing
                                        && voice.snapshot.with(|s| {
                                            s.recording.as_ref().is_some_and(|r| {
                                                p.item.as_ref().is_some_and(|i| i.id == r.path)
                                            })
                                        })
                                });
                                if playing {
                                    tr(i18n, "Pause", "暂停")
                                } else {
                                    i18n.text(Key::Play)
                                }
                            },
                            move || voice.play(),
                        )
                        .disabled(voice.busy)
                    },
                ))
                .child(
                    ui::action(
                        move || {
                            if voice.busy.get() {
                                tr(i18n, "Working…", "处理中…")
                            } else if recording() {
                                i18n.text(Key::Stop)
                            } else {
                                tr(i18n, "Record again", "重录")
                            }
                        },
                        move || {
                            if recording() {
                                voice.stop(false);
                            } else {
                                voice.start();
                            }
                        },
                    )
                    .disabled(voice.busy),
                )
                .child(
                    ui::primary()
                        .disabled(move || {
                            voice.busy.get()
                                || voice
                                    .snapshot
                                    .with(|s| s.status == VoiceRecorderStatus::Idle)
                        })
                        .accessibility_label(move || Some(i18n.text(Key::Send).into()))
                        .on_press(move || {
                            if recording() {
                                voice.stop(true);
                            } else {
                                voice.send();
                            }
                        })
                        .child(ui::icon(ui::Icon::Send).size_5()),
                ),
        )
        .child(when(voice.failed, move || {
            text(move || i18n.text(Key::CommandFailed))
                .text_sm()
                .text_color(move || theme().destructive)
        }))
}

fn waveform(voice: Voice) -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .h_8()
        .overflow_hidden()
        .children((0..32).map(move |index| {
            view()
                .flex_1()
                .max_w(px(4.0))
                .min_w(px(1.0))
                .rounded_full()
                .bg(move || theme().primary)
                .h(move || {
                    let amplitude = voice.snapshot.with(|s| {
                        let start = index * s.waveform.len() / 32;
                        let end = ((index + 1) * s.waveform.len() / 32)
                            .max(start + 1)
                            .min(s.waveform.len());
                        s.waveform
                            .get(start..end)
                            .and_then(|values| values.iter().max())
                            .copied()
                            .unwrap_or_default()
                    });
                    px(2.0 + f32::from(amplitude) / 255.0 * 26.0)
                })
        }))
}
