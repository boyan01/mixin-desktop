use futures::StreamExt as _;
use mixin_desktop_api::MessageListView;
use nsh_core::{file_dialog::SaveDialog, prelude::*};
use std::collections::HashMap;

mod camera;
pub(crate) mod input;
use camera::Camera;

use super::{message_actions::Actions, message_content::available};
use crate::{
    i18n::Key,
    runtime,
    ui::{self, Typography},
};

pub(super) fn screen(
    initial: MessageListView,
    transcript: Option<String>,
    actions: Actions,
) -> impl IntoElement {
    let owner = Owner::current();
    let current = signal(initial.message_id.clone());
    let conversation = stored(initial.conversation_id.clone());
    let rows = signal(Vec::<MessageListView>::new());
    let loading = signal(true);
    let failed = signal(false);
    let busy = signal(false);
    let retry = signal(0_u64);
    let forward_allowed = signal(false);
    let image_failed = signal(false);
    let source = stored(transcript);
    effect(move || {
        let anchor = current.get();
        let request = retry.get();
        let Some(client) = actions.account.client() else {
            return;
        };
        let transcript = source.with(|source| source.clone());
        let conversation = conversation.with(|id| id.clone());
        loading.set(true);
        failed.set(false);
        forward_allowed.set(false);
        image_failed.set(false);
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(
                client.message_changes(Some(conversation.clone())),
            ));
            loop {
                let client = client.clone();
                let conversation = conversation.clone();
                let anchor = anchor.clone();
                let transcript = transcript.clone();
                let expected_anchor = anchor.clone();
                let result = runtime::run(async move {
                    if let Some(root) = transcript {
                        return client
                            .message()
                            .transcript_image_messages_around(root, anchor, 1, 1)
                            .await
                            .map(|rows| (rows, false));
                    }
                    let can_forward = client
                        .message()
                        .message_actions(conversation.clone(), vec![anchor.clone()])
                        .await?
                        .into_iter()
                        .any(|policy| policy.can_forward);
                    let images = if actions.pinned_page {
                        client
                            .message()
                            .pinned_image_messages_around(conversation, anchor, 1, 1)
                            .await?
                    } else {
                        client
                            .message()
                            .image_messages_around(conversation, anchor, 1, 1)
                            .await?
                    };
                    let ids = images.iter().map(|i| i.message_id.clone()).collect();
                    let mut details: HashMap<_, _> = client
                        .message()
                        .message_items_by_ids(ids)
                        .await?
                        .into_iter()
                        .map(|m| (m.message_id.clone(), m))
                        .collect();
                    Ok((
                        images
                            .into_iter()
                            .filter_map(|image| details.remove(&image.message_id))
                            .collect::<Vec<_>>(),
                        can_forward,
                    ))
                })
                .await;
                if !owner.is_live()
                    || current.get_untracked() != expected_anchor
                    || retry.get_untracked() != request
                {
                    return;
                }
                match result {
                    Ok((items, can_forward)) if !items.is_empty() => {
                        rows.set(items);
                        forward_allowed.set(can_forward);
                        failed.set(false);
                    }
                    _ => {
                        rows.set(Vec::new());
                        failed.set(true);
                        forward_allowed.set(false);
                    }
                }
                loading.set(false);
                if changes.next().await.is_none() {
                    break;
                }
            }
        });
    });
    viewer(
        actions,
        rows,
        current,
        loading,
        failed,
        busy,
        retry,
        forward_allowed,
        image_failed,
        source,
        signal(Camera::new((1.0, 1.0), (1.0, 1.0))),
    )
}

/// A round icon command over the picture, on the dark overlay in either appearance.
fn viewer_button(
    icon: ui::Icon,
    label: impl Fn() -> &'static str + Copy + 'static,
    on_press: impl Fn() + 'static,
    disabled: impl Fn() -> bool + 'static,
) -> View {
    view()
        .flex_none()
        .size_9()
        .items_center()
        .justify_center()
        .pointer_events(PointerEvents::Auto)
        .tooltip(ui::hint(move || label().to_owned()))
        .child(
            ui::round_button()
                .size_9()
                .text_color(ui::media_overlay_foreground)
                .hover(|style| style.bg(|| ui::media_overlay_foreground().opacity(0.18)))
                .disabled(disabled)
                .accessibility_label(move || Some(label().into()))
                .on_press(on_press)
                .child(ui::icon(icon).size(ui::icon_lg())),
        )
}

pub(crate) fn photo_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .gap_2()
        .items_center()
        .pointer_events(PointerEvents::Auto)
        .child(
            text(move || i18n.tr("Couldn’t display this photo.", "无法显示此图片。"))
                .text_role(ui::TextRole::Body)
                .text_color(ui::media_overlay_foreground),
        )
        .child(
            ui::link(move || i18n.text(Key::Retry), retry).text_color(ui::media_overlay_foreground),
        )
}

#[allow(clippy::too_many_arguments)]
fn viewer(
    actions: Actions,
    rows: Signal<Vec<MessageListView>>,
    current: Signal<String>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    busy: Signal<bool>,
    retry: Signal<u64>,
    forward_allowed: Signal<bool>,
    image_failed: Signal<bool>,
    source: Stored<Option<String>>,
    camera: Signal<Camera>,
) -> View {
    let i18n = actions.i18n;
    let selected = move || {
        rows.with(|rows| {
            rows.iter()
                .find(|row| row.message_id == current.get())
                .cloned()
        })
    };
    let viewport = signal((1.0_f32, 1.0_f32));
    let identity = stored((String::new(), (0.0_f32, 0.0_f32)));
    effect(move || {
        let viewport = viewport.get();
        let ratio = pixel_ratio().max(1.0);
        if let Some(row) = selected() {
            let dimensions = row
                .media_width
                .zip(row.media_height)
                .filter(|(w, h)| *w > 0 && *h > 0)
                .map(|(w, h)| (w as f32 / ratio, h as f32 / ratio))
                .unwrap_or(viewport);
            let next = (row.message_id, dimensions);
            if identity.with(|old| *old != next) {
                identity.replace(next);
                camera.set(Camera::new(dimensions, viewport));
            } else {
                camera.update(|camera| camera.resize(viewport));
            }
        }
    });
    let zoom_at = move |factor, pointer| camera.update(|camera| camera.zoom(factor, pointer));
    let zoom_center = move |factor| camera.update(|camera| camera.zoom(factor, camera.center()));
    let fit = move || camera.update(Camera::fit);
    let toggle = move |pointer| {
        camera.update(|camera| {
            if (camera.scale - camera.fit_scale()).abs() < 0.001 {
                camera.zoom(1.0 / camera.scale, pointer);
            } else {
                camera.fit();
            }
        })
    };
    let neighbor = move |direction: isize| {
        rows.with(|rows| {
            let index = rows
                .iter()
                .position(|row| row.message_id == current.get())?;
            rows.get(index.checked_add_signed(direction)?)
                .map(|row| row.message_id.clone())
        })
    };
    let go = move |direction| {
        if !loading.get_untracked()
            && let Some(id) = neighbor(direction)
        {
            current.set(id);
        }
    };
    on_key("left", move |_| go(-1));
    on_key("right", move |_| go(1));
    on_key("mod+0", move |_| fit());
    on_key("mod+1", move |_| {
        camera.update(|camera| camera.zoom(1.0 / camera.scale, camera.center()))
    });
    on_key("mod+=", move |_| zoom_center(1.25));
    on_key("mod+-", move |_| zoom_center(0.8));
    let usable = move || {
        !(busy.get()
            || loading.get()
            || image_failed.get()
            || selected().is_none_or(|row| !available(&row)))
    };
    let forward = move || {
        if !forward_allowed.get_untracked() {
            return;
        }
        if let Some(row) = selected() {
            actions.close_preview();
            actions.combine.set(false);
            actions.forwarding.set(vec![row.message_id]);
        }
    };
    let download = move || {
        let Some(row) = selected() else {
            return;
        };
        let Some(client) = actions.account.client() else {
            return;
        };
        let transcript = source.with(|source| source.clone());
        busy.set(true);
        spawn(async move {
            let result = runtime::commit(async move {
                if let Some(parent) = transcript {
                    client
                        .attachment()
                        .download_transcript_attachment(parent, row.message_id)
                        .await
                } else {
                    client
                        .attachment()
                        .download_attachment(row.message_id)
                        .await
                }
            })
            .await;
            actions.failed.set(result.is_err());
            if result.is_ok() {
                image_failed.set(false);
                retry.update(|v| *v += 1);
            }
            busy.set(false);
        });
    };
    let copy = move || {
        let Some(row) = selected() else {
            return;
        };
        let Some(client) = actions.account.client() else {
            return;
        };
        let transcript = source.with(|source| source.clone());
        busy.set(true);
        spawn(async move {
            let result = runtime::run(async move {
                client
                    .attachment()
                    .attachment_image_data_uri(row.message_id, transcript)
                    .await
            })
            .await;
            match result {
                Ok(uri) => {
                    set_clipboard_image(uri);
                    actions.failed.set(false);
                    actions
                        .toasts
                        .success(i18n.tr("Image copied", "图片已复制"));
                }
                Err(_) => actions.failed.set(true),
            }
            busy.set(false);
        });
    };
    let save = move || {
        let Some(row) = selected() else {
            return;
        };
        let Some(client) = actions.account.client() else {
            return;
        };
        let transcript = source.with(|source| source.clone());
        busy.set(true);
        spawn(async move {
            let name = super::message_actions::attachment_name(&row);
            match pick_save_path(SaveDialog::new().file_name(name)).await {
                Ok(Some(path)) => {
                    let saved = runtime::commit(async move {
                        let path = path.to_string_lossy().into_owned();
                        if let Some(root) = transcript {
                            return client
                                .attachment()
                                .save_transcript_attachment_as(root, row.message_id, path)
                                .await;
                        }
                        client
                            .attachment()
                            .save_attachment_as(row.message_id, path)
                            .await
                    })
                    .await
                    .is_ok();
                    actions.failed.set(!saved);
                    if saved {
                        actions.toasts.success(i18n.tr("Saved", "已保存"));
                    }
                }
                Ok(None) => {}
                Err(_) => actions.failed.set(true),
            }
            busy.set(false);
        });
    };
    let canvas = view()
        .flex_1()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .test_id("photo-canvas")
        .on_layout(move |bounds: &Bounds<Points>| {
            viewport.set((bounds.size.width.0.max(1.0), bounds.size.height.0.max(1.0)));
        })
        .child(when(
            move || selected().is_some_and(|m| available(&m)) && !image_failed.get(),
            move || {
                view()
                    .absolute()
                    .left(move || px(camera.get().origin.0))
                    .top(move || px(camera.get().origin.1))
                    .w(move || px(camera.get().rendered().0))
                    .h(move || px(camera.get().rendered().1))
                    .pointer_events(PointerEvents::None)
                    .child(
                        image(move || selected().and_then(|m| m.media_url).unwrap_or_default())
                            .absolute()
                            .left(move || {
                                px((camera.get().rendered().0
                                    - camera.get().image.0 * camera.get().scale)
                                    / 2.0)
                            })
                            .top(move || {
                                px((camera.get().rendered().1
                                    - camera.get().image.1 * camera.get().scale)
                                    / 2.0)
                            })
                            .w(move || px(camera.get().image.0 * camera.get().scale))
                            .h(move || px(camera.get().image.1 * camera.get().scale))
                            .rotate(move || deg(f32::from(camera.get().rotation) * 90.0))
                            .object_fit(ResizeMode::Contain)
                            .label(move || i18n.text(Key::Photo))
                            .on_error(move |_: &str| image_failed.set(true)),
                    )
            },
        ))
        .child(input::surface(
            usable,
            zoom_at,
            move |delta| camera.update(|camera| camera.pan(delta)),
            toggle,
        ));
    let layer = || {
        view()
            .absolute()
            .inset_0()
            .pointer_events(PointerEvents::None)
    };
    let identity_view = move || {
        view()
            .flex_1()
            .min_w_0()
            .flex_row()
            .items_center()
            .gap(ui::space_lg())
            .child(ui::avatar_view(
                move || selected().map(|row| row.sender_id).unwrap_or_default(),
                move || {
                    selected()
                        .map(|row| row.sender_name)
                        .unwrap_or_default()
                        .into()
                },
                move || {
                    selected()
                        .map(|row| row.sender_avatar_url)
                        .unwrap_or_default()
                },
                ui::AvatarSize::Bar,
            ))
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .child(
                        text(move || selected().map(|row| row.sender_name).unwrap_or_default())
                            .text_role(ui::TextRole::Title)
                            .number_of_lines(1)
                            .text_color(ui::media_overlay_foreground),
                    )
                    .child(
                        text(move || {
                            selected()
                                .and_then(|row| row.sender_identity_number)
                                .unwrap_or_default()
                        })
                        .text_role(ui::TextRole::Body)
                        .number_of_lines(1)
                        .text_color(ui::viewer_secondary),
                    ),
            )
    };
    let tools = move || {
        view()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(ui::space_sm())
            .child(viewer_button(
                ui::Icon::ZoomIn,
                move || i18n.tr("Zoom in", "放大"),
                move || zoom_center(1.25),
                move || !usable() || camera.get().scale >= camera.get().limits().1,
            ))
            .child(viewer_button(
                ui::Icon::ZoomOut,
                move || i18n.tr("Zoom out", "缩小"),
                move || zoom_center(0.8),
                move || !usable() || camera.get().scale <= camera.get().limits().0,
            ))
            .child(viewer_button(
                ui::Icon::Rotate,
                move || i18n.tr("Rotate", "旋转"),
                move || camera.update(Camera::turn),
                move || !usable(),
            ))
            .child(when(
                move || source.with(|value| value.is_none()),
                move || {
                    viewer_button(
                        ui::Icon::Forward,
                        move || i18n.tr("Forward…", "转发…"),
                        forward,
                        move || busy.get() || loading.get() || !forward_allowed.get(),
                    )
                },
            ))
            .child(viewer_button(
                ui::Icon::Copy,
                move || i18n.tr("Copy image", "复制图片"),
                copy,
                move || !usable(),
            ))
            .child(viewer_button(
                ui::Icon::Download,
                move || i18n.tr("Save as…", "另存为…"),
                move || {
                    if selected().is_some_and(|row| available(&row)) {
                        save();
                    } else {
                        download();
                    }
                },
                move || busy.get() || loading.get() || selected().is_none(),
            ))
    };
    let close = move || {
        viewer_button(
            ui::Icon::CloseCircle,
            move || i18n.tr("Close", "关闭"),
            move || actions.close_preview(),
            || false,
        )
    };
    let compact = move || viewport.get().0 < ui::viewer_toolbar_wide().0;
    let row = || {
        view()
            .min_h(ui::viewer_toolbar_height())
            .flex_row()
            .items_center()
            .gap(ui::space_lg())
            .px(ui::space_xl())
            .pl(move || ui::title_bar_leading(px(0.0)))
            .py(ui::space_sm())
    };
    let header = view()
        .flex_none()
        .bg(ui::viewer_toolbar_fill)
        .window_drag(true)
        .child(
            when(compact, move || {
                view()
                    .child(row().child(identity_view()).child(close()))
                    .child(tools().self_end().px(ui::space_xl()).pb(ui::space_sm()))
            })
            .otherwise(move || row().child(identity_view()).child(tools()).child(close())),
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
                .child(canvas)
                .child(
                    layer()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(when(loading, move || {
                            text(move || i18n.text(Key::LoadingMessages))
                                .text_role(ui::TextRole::Body)
                                .text_color(ui::media_overlay_foreground)
                        }))
                        .child(when(
                            move || failed.get() || image_failed.get(),
                            move || {
                                photo_error(move || {
                                    image_failed.set(false);
                                    retry.update(|v| *v += 1);
                                })
                            },
                        )),
                )
                .child(
                    layer()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px(ui::space_lg())
                        .child(
                            viewer_button(
                                ui::Icon::ChevronLeft,
                                move || i18n.tr("Previous photo", "上一张"),
                                move || go(-1),
                                move || loading.get() || neighbor(-1).is_none(),
                            )
                            .size_10(),
                        )
                        .child(
                            viewer_button(
                                ui::Icon::ChevronRight,
                                move || i18n.tr("Next photo", "下一张"),
                                move || go(1),
                                move || loading.get() || neighbor(1).is_none(),
                            )
                            .size_10(),
                        ),
                )
                .child(
                    layer()
                        .justify_end()
                        .items_center()
                        .pb(ui::space_lg())
                        .child(when(
                            move || {
                                selected()
                                    .and_then(|row| row.caption)
                                    .is_some_and(|caption| !caption.is_empty())
                            },
                            move || {
                                view()
                                    .max_w(relative(0.8))
                                    .max_h_20()
                                    .px(ui::space_lg())
                                    .py(ui::space_sm())
                                    .rounded(ui::radius_md())
                                    .bg(ui::media_overlay)
                                    .pointer_events(PointerEvents::Auto)
                                    .child(
                                        scroll().child(
                                            text(move || {
                                                selected()
                                                    .and_then(|row| row.caption)
                                                    .unwrap_or_default()
                                            })
                                            .text_role(ui::TextRole::Body)
                                            .text_color(ui::media_overlay_foreground)
                                            .select_text(),
                                        ),
                                    )
                            },
                        )),
                ),
        )
}

#[cfg(debug_assertions)]
pub(crate) fn detached(
    rows: Vec<MessageListView>,
    own_id: String,
    mode: Signal<&'static str>,
) -> View {
    let current = rows
        .first()
        .map(|row| row.message_id.clone())
        .unwrap_or_default();
    let camera = signal(Camera::new((1.0, 1.0), (1.0, 1.0)));
    let failed = signal(false);
    let result = viewer(
        Actions::detached(own_id),
        signal_with(rows, SignalEquals::Never),
        signal(current),
        signal(false),
        signal(false),
        signal(false),
        signal(0),
        signal(false),
        failed,
        stored(None),
        camera,
    );
    effect(move || {
        let mode = mode.get();
        failed.set(mode == "Error");
        camera.update(|camera| {
            camera.rotation = 0;
            camera.fit();
            match mode {
                "Zoom" => camera.zoom(2.0, (camera.viewport.0 * 0.65, camera.viewport.1 * 0.4)),
                "Edge" => {
                    camera.zoom(4.0, (camera.viewport.0 - 1.0, 1.0));
                    camera.pan((-50000.0, 50000.0));
                }
                "Rotate" => camera.turn(),
                _ => {}
            }
        });
    });
    result
}
