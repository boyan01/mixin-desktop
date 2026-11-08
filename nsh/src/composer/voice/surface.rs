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

    pub(crate) fn button(self, disabled: Memo<bool>) -> impl IntoElement {
        let i18n = i18n::current();
        super::super::surface::hinted(
            tool_button()
                .disabled(move || disabled.get() || self.busy.get() || self.open.get())
                .accessibility_label(move || Some(i18n.text(Key::Record).into()))
                .child(ui::icon(ui::Icon::Microphone))
                .on_press(move || self.start()),
            move || tr(i18n, "Record voice message", "录制语音消息").to_owned(),
        )
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
    RecordingPanel {
        snapshot: voice.snapshot,
        playing: computed(move || {
            voice.playback.with(|p| {
                p.status == AudioPlaybackStatus::Playing
                    && voice.snapshot.with(|s| {
                        s.recording
                            .as_ref()
                            .is_some_and(|r| p.item.as_ref().is_some_and(|i| i.id == r.path))
                    })
            })
        }),
        busy: voice.busy,
        failed: voice.failed,
        cancel: std::rc::Rc::new(move || voice.cancel()),
        play: std::rc::Rc::new(move || voice.play()),
        toggle: std::rc::Rc::new(move || {
            if voice
                .snapshot
                .with_untracked(|s| s.status == VoiceRecorderStatus::Recording)
            {
                voice.stop(false);
            } else {
                voice.start();
            }
        }),
        send: std::rc::Rc::new(move || {
            if voice
                .snapshot
                .with_untracked(|s| s.status == VoiceRecorderStatus::Recording)
            {
                voice.stop(true);
            } else {
                voice.send();
            }
        }),
    }
    .view()
}

/// A recording's presentation. The recorder owns state and commands.
pub(crate) struct RecordingPanel {
    pub snapshot: Signal<VoiceRecorderSnapshot>,
    pub playing: Memo<bool>,
    pub busy: Signal<bool>,
    pub failed: Signal<bool>,
    pub cancel: std::rc::Rc<dyn Fn()>,
    pub play: std::rc::Rc<dyn Fn()>,
    pub toggle: std::rc::Rc<dyn Fn()>,
    pub send: std::rc::Rc<dyn Fn()>,
}

impl RecordingPanel {
    pub(crate) fn view(self) -> impl IntoElement {
        use super::super::surface::{frame, hinted, send_button};
        let Self {
            snapshot,
            playing,
            busy,
            failed,
            cancel,
            play,
            toggle,
            send,
        } = self;
        let i18n = i18n::current();
        let recording = move || snapshot.with(|s| s.status == VoiceRecorderStatus::Recording);
        let toggle_label = move || {
            if recording() {
                tr(i18n, "Stop and preview", "停止并试听")
            } else {
                tr(i18n, "Record again", "重新录音")
            }
        };
        let cancel = hinted(
            tool_button()
                .disabled(busy)
                .accessibility_label(move || Some(i18n.text(Key::Cancel).into()))
                .on_press(move || cancel())
                .child(ui::icon(ui::Icon::Close)),
            move || tr(i18n, "Discard recording", "放弃录音").to_owned(),
        );
        let toggle = hinted(
            tool_button()
                .disabled(busy)
                .accessibility_label(move || {
                    Some(
                        if recording() {
                            i18n.text(Key::Stop)
                        } else {
                            toggle_label()
                        }
                        .into(),
                    )
                })
                .on_press(move || toggle())
                .child(
                    when(recording, || ui::icon(ui::Icon::Stop))
                        .otherwise(|| ui::icon(ui::Icon::Rotate)),
                ),
            move || toggle_label().to_owned(),
        );
        let send = hinted(
            send_button()
                .disabled(move || {
                    busy.get() || snapshot.with(|s| s.status == VoiceRecorderStatus::Idle)
                })
                .accessibility_label(move || Some(i18n.text(Key::Send).into()))
                .on_press(move || send())
                .child(ui::icon(ui::Icon::Send).text_color(ui::fg_on_accent)),
            move || tr(i18n, "Send voice message", "发送语音").to_owned(),
        );
        view().px_4().pt_2().pb_3().child(
            frame()
                .p_3()
                .gap_2()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .min_h_10()
                        .child(
                            when(recording, move || {
                                view().size_2().rounded_full().bg(ui::danger)
                            })
                            .otherwise(move || {
                                let play = play.clone();
                                let label = move || {
                                    if playing.get() {
                                        tr(i18n, "Pause", "暂停")
                                    } else {
                                        tr(i18n, "Listen to recording", "试听录音")
                                    }
                                };
                                hinted(
                                    tool_button()
                                        .disabled(busy)
                                        .accessibility_label(move || Some(label().into()))
                                        .on_press(move || play())
                                        .child(
                                            when(playing, || ui::icon(ui::Icon::Pause))
                                                .otherwise(|| ui::icon(ui::Icon::Play)),
                                        ),
                                    move || label().to_owned(),
                                )
                            }),
                        )
                        .child(
                            view()
                                .flex_shrink_0()
                                .gap_1()
                                .child(
                                    text(move || {
                                        if busy.get() {
                                            tr(i18n, "Working…", "处理中…")
                                        } else if recording() {
                                            tr(i18n, "Recording", "正在录音")
                                        } else {
                                            tr(i18n, "Voice message", "语音消息")
                                        }
                                    })
                                    .text_role(ui::TextRole::Body)
                                    .text_emphasis(),
                                )
                                .child(
                                    text(move || {
                                        snapshot.with(|s| {
                                            format!(
                                                "{:02}:{:02}",
                                                s.duration_millis / 60_000,
                                                (s.duration_millis / 1000) % 60
                                            )
                                        })
                                    })
                                    .text_role(ui::TextRole::Caption)
                                    .text_color(ui::fg_secondary),
                                ),
                        )
                        .child(waveform(snapshot)),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(cancel)
                        .child(view().flex_1())
                        .child(toggle)
                        .child(send),
                )
                .child(when(failed, move || {
                    ui::error_text(move || i18n.text(Key::CommandFailed))
                })),
        )
    }
}

fn waveform(snapshot: Signal<VoiceRecorderSnapshot>) -> impl IntoElement {
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
                    let amplitude = snapshot.with(|s| {
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
