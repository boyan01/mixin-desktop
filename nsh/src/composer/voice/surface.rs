use super::*;
use crate::{
    composer::tool_button,
    ui::{self, Typography as _},
};

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
            .child(ui::icon(ui::Icon::Microphone))
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
                                                        voice.bounds.with(|b| {
                                                            b.map_or(Points::ZERO, |b| b.page.x)
                                                        })
                                                    })
                                                    .w(move || {
                                                        voice.bounds.with(|b| {
                                                            b.map_or(Points::ZERO, |b| {
                                                                b.frame.size.width
                                                            })
                                                        })
                                                    })
                                                    .bottom(move || {
                                                        voice.bounds.with(|b| {
                                                            b.map_or(Points::ZERO, |b| {
                                                                (window().height
                                                                    - b.page.y
                                                                    - b.frame.size.height)
                                                                    .max(Points::ZERO)
                                                            })
                                                        })
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
        .bg(ui::surface_pane)
        .border_t_1()
        .border_color(ui::border)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    tool_button()
                        .disabled(voice.busy)
                        .accessibility_label(move || Some(i18n.text(Key::Cancel).into()))
                        .child(ui::icon(ui::Icon::Close))
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
                                .size(ui::space_sm())
                                .rounded(ui::radius_full())
                                .bg(ui::danger)
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
                            .text_role(ui::TextRole::Body),
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
                        .child(ui::icon(ui::Icon::Send)),
                ),
        )
        .child(when(voice.failed, move || {
            ui::error_text(move || i18n.text(Key::CommandFailed))
        }))
}

fn waveform(voice: Voice) -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .flex_row()
        .items_center()
        .gap(ui::space_2xs())
        .h_8()
        .overflow_hidden()
        .children((0..32).map(move |index| {
            view()
                .flex_1()
                .max_w(ui::space_xs())
                .min_w(ui::hairline_width())
                .rounded(ui::radius_full())
                .bg(ui::accent)
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
                    // Measured data: the sample's amplitude, within the 32pt row.
                    px(ui::space_2xs().0 + f32::from(amplitude) / 255.0 * 26.0)
                })
        }))
}
