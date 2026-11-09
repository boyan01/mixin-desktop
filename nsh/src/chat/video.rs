//! A video uses the photo viewer's window chrome and its own transport controls.
use std::{cell::Cell, rc::Rc};

use mixin_desktop_api::MessageListView;
use nsh_base::SliderValue;
use nsh_core::{
    desktop::WindowMode,
    element::{
        VideoInfo, VideoProgress,
        desktop::{current_window, window_state},
    },
    file_dialog::SaveDialog,
    prelude::*,
};

use super::{
    media_viewer,
    message_actions::{Actions, attachment_name},
};
use crate::{
    i18n::Key,
    runtime,
    ui::{self, Typography},
};

#[derive(Clone, Copy)]
struct Playback {
    paused: Signal<bool>,
    loaded: Signal<bool>,
    buffering: Signal<bool>,
    failed: Signal<bool>,
    ended: Signal<bool>,
    replaying: Signal<bool>,
    position: Signal<f64>,
    duration: Signal<f64>,
    volume: Signal<f32>,
    muted: Signal<bool>,
    rate: Signal<f32>,
}

impl Playback {
    fn new(paused: bool) -> Self {
        Self {
            paused: signal(paused),
            loaded: signal(false),
            buffering: signal(false),
            failed: signal(false),
            ended: signal(false),
            replaying: signal(false),
            position: signal(0.0),
            duration: signal(0.0),
            volume: signal(1.0),
            muted: signal(false),
            rate: signal(1.0),
        }
    }
    fn ready(self) -> bool {
        self.loaded.get() && !self.failed.get() && !self.replaying.get()
    }
}

pub(super) fn screen(
    row: MessageListView,
    transcript: Option<String>,
    actions: Actions,
    presence: crate::motion::Presence,
) -> View {
    surface(
        row,
        transcript,
        actions,
        Playback::new(false),
        signal(false),
        presence,
    )
}

fn surface(
    row: MessageListView,
    transcript: Option<String>,
    actions: Actions,
    state: Playback,
    can_forward: Signal<bool>,
    presence: crate::motion::Presence,
) -> View {
    let i18n = actions.i18n;
    let security = use_context::<crate::security::Handle>();
    let row = stored(row);
    let transcript = stored(transcript);
    let player = element_ref();
    let busy = signal(false);
    let native_window = current_window();
    // Cleanup runs after reactive storage is disposed, so restoration belongs to
    // the native window handle and a plain cell, not a signal or Stored value.
    let restore_mode = Rc::new(Cell::new(None::<WindowMode>));
    let fullscreen =
        move || window_state().is_some_and(|state| state.mode == WindowMode::FullScreen);
    let toggle_fullscreen = {
        let native_window = native_window.clone();
        let restore_mode = restore_mode.clone();
        move || {
            if fullscreen() {
                native_window.set_mode(restore_mode.take().unwrap_or(WindowMode::Normal));
            } else {
                restore_mode.set(Some(
                    native_window
                        .state()
                        .map_or(WindowMode::Normal, |state| state.mode),
                ));
                native_window.set_mode(WindowMode::FullScreen);
            }
        }
    };
    cleanup(move || {
        if let Some(mode) = restore_mode.get()
            && native_window
                .state()
                .is_some_and(|state| state.mode == WindowMode::FullScreen)
        {
            native_window.set_mode(mode);
        }
    });
    let toggle = move || {
        if !state.ready() {
            return;
        }
        if state.ended.get_untracked() {
            // AVPlayer seeks asynchronously. Resume only after it reports the
            // beginning, so setting rate at the old end cannot stop the replay.
            state.replaying.set(true);
            state.position.set(0.0);
            player.seek(0.0);
        } else {
            state.paused.update(|paused| *paused = !*paused);
        }
    };
    let seek = move |seconds: f64| {
        if state.ready() && seconds.is_finite() && state.duration.get_untracked() > 0.0 {
            let seconds = seconds.clamp(0.0, state.duration.get_untracked());
            state.position.set(seconds);
            let ended = seconds >= state.duration.get_untracked();
            state.ended.set(ended);
            if ended {
                state.paused.set(true);
            }
            player.seek(seconds);
        }
    };
    let mute = move || {
        if state.volume.get_untracked() <= 0.0 {
            state.volume.set(1.0);
            state.muted.set(false);
        } else {
            state.muted.update(|muted| *muted = !*muted);
        }
    };
    on_key("space", move |_| toggle());
    on_key("left", move |_| seek(state.position.get_untracked() - 5.0));
    on_key("right", move |_| seek(state.position.get_untracked() + 5.0));
    on_key("m", move |_| mute());
    let key_fullscreen = toggle_fullscreen.clone();
    on_key("f", move |_| key_fullscreen());
    let save = move || {
        if busy.get_untracked() {
            return;
        }
        let Some(client) = actions.account.client() else {
            return;
        };
        let row = row.get();
        let transcript = transcript.get();
        busy.set(true);
        spawn(async move {
            let result =
                match pick_save_path(SaveDialog::new().file_name(attachment_name(&row))).await {
                    Ok(Some(path)) => Some(
                        runtime::commit(async move {
                            let path = path.to_string_lossy().into_owned();
                            if let Some(root) = transcript {
                                client
                                    .attachment()
                                    .save_transcript_attachment_as(root, row.message_id, path)
                                    .await
                            } else {
                                client
                                    .attachment()
                                    .save_attachment_as(row.message_id, path)
                                    .await
                            }
                        })
                        .await
                        .is_ok(),
                    ),
                    Ok(None) => None,
                    Err(_) => Some(false),
                };
            if let Some(ok) = result {
                if ok {
                    actions.toasts.success(i18n.tr("Saved", "已保存"));
                } else {
                    actions.toasts.error(i18n.text(Key::CommandFailed));
                }
            }
            busy.set(false);
        });
    };
    effect(move || {
        if transcript.with(|root| root.is_some()) {
            return;
        }
        let Some(client) = actions.account.client() else {
            return;
        };
        let row = row.get();
        spawn(async move {
            let allowed = runtime::run(async move {
                client
                    .message()
                    .message_actions(row.conversation_id, vec![row.message_id])
                    .await
            })
            .await
            .is_ok_and(|policies| policies.iter().any(|policy| policy.can_forward));
            can_forward.set(allowed);
        });
    });
    let tools = move || {
        view()
            .flex_row()
            .items_center()
            .gap(ui::space_sm())
            .child(when(
                move || transcript.with(|root| root.is_none()),
                move || {
                    media_viewer::button(
                        ui::Icon::Forward,
                        move || i18n.tr("Forward…", "转发…"),
                        move || {
                            if !can_forward.get_untracked() || actions.account.client().is_none() {
                                return;
                            }
                            actions.close_preview();
                            actions.combine.set(false);
                            actions
                                .forwarding
                                .set(vec![row.with(|row| row.message_id.clone())]);
                        },
                        move || !can_forward.get() || busy.get(),
                    )
                },
            ))
            .child(media_viewer::button(
                ui::Icon::Download,
                move || i18n.tr("Save as…", "另存为…"),
                save,
                move || busy.get(),
            ))
    };
    let header = media_viewer::header(
        move || Some(row.get()),
        tools,
        move || actions.back_preview(),
        // Two video actions fit beside the identity at the minimum window width.
        || false,
    );
    let video = when(
        move || !state.failed.get(),
        move || {
            let src = row.with(|row| row.media_url.clone().unwrap_or_default());
            video(src)
                .element_ref(&player)
                .controls(false)
                .object_fit(ResizeMode::Contain)
                .paused(move || {
                    state.paused.get()
                        || !presence.interactive()
                        || security.is_some_and(|security| security.blocked())
                })
                .volume(state.volume)
                .muted(state.muted)
                .rate(state.rate)
                .progress_interval(250.0)
                .flex_1()
                .min_h_0()
                .w_full()
                .label(move || i18n.tr("Video", "视频"))
                .on_load_start(move || {
                    state.loaded.set(false);
                    state.buffering.set(true);
                })
                .on_load(move |info: &VideoInfo| {
                    state.duration.set(f64::from(info.duration).max(0.0));
                    state.loaded.set(true);
                    state.buffering.set(false);
                })
                .on_progress(move |progress: &VideoProgress| {
                    if state.replaying.get_untracked() && progress.current_time <= 0.25 {
                        state.replaying.set(false);
                        state.ended.set(false);
                        state.paused.set(false);
                    }
                    if !state.ended.get_untracked() {
                        state
                            .position
                            .set(f64::from(progress.current_time).max(0.0));
                    }
                    state.duration.set(f64::from(progress.duration).max(0.0));
                })
                .on_buffer(move |buffering: &bool| state.buffering.set(*buffering))
                .on_end(move || {
                    if state.replaying.get_untracked() {
                        return;
                    }
                    state.ended.set(true);
                    state.paused.set(true);
                    state.position.set(state.duration.get_untracked());
                })
                .on_error(move |_: &str| {
                    state.failed.set(true);
                    state.buffering.set(false);
                    state.loaded.set(false);
                    state.replaying.set(false);
                })
        },
    );
    let transport = view()
        .flex_none()
        .bg(ui::viewer_toolbar_fill)
        .px(move || {
            if window().width.0 < ui::viewer_toolbar_wide().0 {
                ui::space_md()
            } else {
                ui::space_xl()
            }
        })
        .py(ui::space_sm())
        .child(
            ui::slider_media()
                .min(0.0)
                .max(move || state.duration.get().max(1.0))
                .step(0.1)
                .value(move || Some(SliderValue::Single(state.position.get())))
                .disabled(move || !state.ready() || state.duration.get() <= 0.0)
                .accessibility_label(move || Some(i18n.tr("Playback position", "播放进度").into()))
                .on_change(move |value: &SliderValue| {
                    if let SliderValue::Single(seconds) = *value {
                        seek(seconds);
                    }
                }),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(move || {
                    if window().width.0 < ui::viewer_toolbar_wide().0 {
                        ui::space_xs()
                    } else {
                        ui::space_md()
                    }
                })
                .child(
                    view().relative().size_9().child(
                        when(
                            move || state.paused.get(),
                            move || {
                                media_viewer::button(
                                    ui::Icon::Play,
                                    move || i18n.tr("Play", "播放"),
                                    toggle,
                                    move || !state.ready(),
                                )
                            },
                        )
                        .otherwise(move || {
                            media_viewer::button(
                                ui::Icon::Pause,
                                move || i18n.tr("Pause", "暂停"),
                                toggle,
                                move || !state.ready(),
                            )
                        }),
                    ),
                )
                .child(
                    text(move || {
                        format!(
                            "{} / {}",
                            timestamp(state.position.get()),
                            timestamp(state.duration.get())
                        )
                    })
                    .text_role(ui::TextRole::Caption)
                    .tabular_nums()
                    .text_color(ui::media_overlay_foreground),
                )
                .child(view().flex_1().min_w_0())
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(ui::space_xs())
                        .child(
                            when(
                                move || state.muted.get() || state.volume.get() <= 0.0,
                                move || {
                                    media_viewer::button(
                                        ui::Icon::VolumeOff,
                                        move || i18n.tr("Unmute", "取消静音"),
                                        mute,
                                        || false,
                                    )
                                },
                            )
                            .otherwise(move || {
                                media_viewer::button(
                                    ui::Icon::Volume,
                                    move || i18n.tr("Mute", "静音"),
                                    mute,
                                    || false,
                                )
                            }),
                        )
                        .child(
                            ui::slider_media()
                                .w(ui::video_volume_width())
                                .style_when(
                                    move || window().width.0 < ui::viewer_toolbar_wide().0,
                                    |style| style.hidden(),
                                )
                                .min(0.0)
                                .max(1.0)
                                .step(0.05)
                                .value(move || {
                                    Some(SliderValue::Single(if state.muted.get() {
                                        0.0
                                    } else {
                                        f64::from(state.volume.get())
                                    }))
                                })
                                .accessibility_label(move || Some(i18n.tr("Volume", "音量").into()))
                                .on_change(move |value: &SliderValue| {
                                    if let SliderValue::Single(volume) = *value {
                                        state.volume.set(volume as f32);
                                        state.muted.set(false);
                                    }
                                }),
                        ),
                )
                .child(speed_menu(state))
                .child(
                    when(fullscreen, {
                        let toggle = toggle_fullscreen.clone();
                        move || {
                            media_viewer::button(
                                ui::Icon::ExitFullScreen,
                                move || i18n.tr("Exit full screen", "退出全屏"),
                                toggle.clone(),
                                || false,
                            )
                        }
                    })
                    .otherwise(move || {
                        media_viewer::button(
                            ui::Icon::FullScreen,
                            move || i18n.tr("Full screen", "全屏"),
                            toggle_fullscreen.clone(),
                            || false,
                        )
                    }),
                ),
        );
    view()
        .flex_1()
        .min_h_0()
        .w_full()
        .bg(ui::viewer_background)
        .child(header)
        .child(
            view()
                .flex_1()
                .min_h_0()
                .relative()
                .child(video)
                .child(when(
                    move || state.buffering.get() && !state.failed.get(),
                    move || {
                        view()
                            .absolute()
                            .inset_0()
                            .items_center()
                            .justify_center()
                            .pointer_events(PointerEvents::None)
                            .child(
                                text(move || i18n.tr("Loading video…", "正在加载视频…"))
                                    .text_role(ui::TextRole::Body)
                                    .text_color(ui::media_overlay_foreground),
                            )
                    },
                ))
                .child(when(state.failed, move || {
                    view()
                        .absolute()
                        .inset_0()
                        .items_center()
                        .justify_center()
                        .gap(ui::space_md())
                        .child(
                            text(move || {
                                i18n.tr("Couldn’t play this video.", "暂时无法播放此视频")
                            })
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::media_overlay_foreground),
                        )
                        .child(
                            ui::command()
                                .text_color(ui::media_overlay_foreground)
                                .on_press(move || {
                                    state.position.set(0.0);
                                    state.ended.set(false);
                                    state.loaded.set(false);
                                    state.failed.set(false);
                                })
                                .child(text(move || i18n.text(Key::Retry))),
                        )
                })),
        )
        .child(when(
            move || {
                row.with(|row| {
                    row.caption
                        .as_ref()
                        .is_some_and(|text| !text.trim().is_empty())
                })
            },
            move || {
                view()
                    .flex_none()
                    .items_center()
                    .bg(ui::viewer_toolbar_fill)
                    .px(ui::space_xl())
                    .pt(ui::space_sm())
                    .child(
                        text(move || row.with(|row| row.caption.clone().unwrap_or_default()))
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::viewer_secondary)
                            .number_of_lines(2)
                            .select_text(),
                    )
            },
        ))
        .child(transport)
}

fn speed_menu(state: Playback) -> impl IntoElement {
    let i18n = crate::i18n::current();
    let mut menu = ui::menu().trigger(
        ui::command()
            .min_w_12()
            .h_9()
            .text_color(ui::media_overlay_foreground)
            .disabled(move || !state.ready())
            .accessibility_label(move || Some(i18n.tr("Playback speed", "播放速度").into()))
            .child(text(move || format!("{}×", state.rate.get())).text_role(ui::TextRole::Body)),
    );
    for (label, rate) in [("0.5×", 0.5), ("1×", 1.0), ("1.5×", 1.5), ("2×", 2.0)] {
        menu = menu.child(
            ui::menu_row(move || label)
                .checked(move || Some(state.rate.get() == rate))
                .indicator(ui::icon(ui::Icon::Check).size(ui::icon_sm()))
                .on_select(move || state.rate.set(rate)),
        );
    }
    menu
}

fn timestamp(seconds: f64) -> String {
    let seconds = if seconds.is_finite() {
        seconds.max(0.0) as u64
    } else {
        0
    };
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(debug_assertions)]
pub(crate) fn detached(row: MessageListView, actions: Actions, mode: Signal<&'static str>) -> View {
    let state = Playback::new(true);
    let presence = use_context::<crate::motion::Presence>().expect("Video preview provides motion");
    let result = surface(row, None, actions, state, signal(true), presence);
    effect(move || match mode.get() {
        "Error" => state.failed.set(true),
        "Playing" => {
            state.failed.set(false);
            state.paused.set(false);
        }
        "Muted" => {
            state.failed.set(false);
            state.paused.set(true);
            state.muted.set(true);
        }
        _ => {
            state.failed.set(false);
            state.paused.set(true);
            state.muted.set(false);
        }
    });
    result
}
