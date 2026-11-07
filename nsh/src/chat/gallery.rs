use futures::StreamExt as _;
use mixin_desktop_api::MessageListView;
use nsh_core::{file_dialog::SaveDialog, prelude::*};
use std::collections::HashMap;

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
    let i18n = actions.i18n;
    let owner = Owner::current();
    let current = signal(initial.message_id.clone());
    let conversation = stored(initial.conversation_id.clone());
    let rows = signal(Vec::<MessageListView>::new());
    let loading = signal(true);
    let failed = signal(false);
    let busy = signal(false);
    let retry = signal(0_u64);
    let zoom = signal(1_f32);
    let rotation = signal(0_u8);
    let forward_allowed = signal(false);
    let image_failed = signal(false);
    let source = stored(transcript);
    effect(move || {
        let anchor = current.get();
        let request = retry.get();
        let client = actions.account.with(|a| a.clone());
        let transcript = source.with(|source| source.clone());
        let conversation = conversation.with(|id| id.clone());
        loading.set(true);
        failed.set(false);
        zoom.set(1.0);
        rotation.set(0);
        forward_allowed.set(false);
        image_failed.set(false);
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(client.message_changes()));
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
    let selected = move || {
        rows.with(|rows| {
            rows.iter()
                .find(|row| row.message_id == current.get())
                .cloned()
        })
    };
    let viewport = signal((1.0_f32, 1.0_f32));
    let offset = signal((0.0_f32, 0.0_f32));
    let drag = stored(None::<((f32, f32), (f32, f32))>);
    let horizontal = element_ref();
    let vertical = element_ref();
    // macOS Scroll ignores native zoom props. Size the unrotated image and
    // its rotated scroll canvas explicitly from the actual available viewport.
    let geometry = computed(move || {
        let (vw, vh) = viewport.get();
        let (width, height) = selected()
            .and_then(|row| row.media_width.zip(row.media_height))
            .filter(|(width, height)| *width > 0 && *height > 0)
            .map(|(width, height)| (width as f32, height as f32))
            .unwrap_or((vw, vh));
        let rotated = !rotation.get().is_multiple_of(2);
        let (rw, rh) = if rotated {
            (height, width)
        } else {
            (width, height)
        };
        let scale = (vw / rw).min(vh / rh) * zoom.get();
        (
            width * scale,
            height * scale,
            (rw * scale).max(vw),
            (rh * scale).max(vh),
        )
    });
    // Every length below comes from the picture and the zoom, not from the design.
    let scroll_to = move |next: (f32, f32)| {
        horizontal.scroll_to_offset(px(next.0), false);
        vertical.scroll_to_offset(px(next.1), false);
    };
    let canvas_width = move || px(geometry.get().2);
    effect(move || {
        let (_, _, width, height) = geometry.get();
        let (vw, vh) = viewport.get();
        let next = ((width - vw) / 2.0, (height - vh) / 2.0);
        drag.replace(None);
        offset.set(next);
        scroll_to(next);
    });
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
    on_key("mod+0", move |_| zoom.set(1.0));
    view()
        .flex_1()
        .min_h_0()
        .gap_3()
        .child(when(loading, move || {
            text(move || i18n.text(Key::LoadingMessages)).text_role(ui::TextRole::Body)
        }))
        .child(when(
            move || failed.get() || image_failed.get(),
            move || {
                view()
                    .gap_2()
                    .child(ui::error_text(move || {
                        i18n.tr("Couldn’t display this photo.", "无法显示此图片。")
                    }))
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || {
                            image_failed.set(false);
                            retry.update(|v| *v += 1);
                        },
                    ))
            },
        ))
        .child(
            scroll()
                .horizontal()
                .element_ref(&horizontal)
                .flex_1()
                .min_h_0()
                .w_full()
                .on_layout(move |bounds: &Bounds<Points>| {
                    viewport.set((bounds.size.width.0.max(1.0), bounds.size.height.0.max(1.0)));
                })
                .on_scroll(move |event: &ScrollEvent| {
                    offset.update(|offset| offset.0 = event.offset.x.0);
                })
                .child(
                    scroll()
                        .element_ref(&vertical)
                        .flex_none()
                        .w(canvas_width)
                        .h_full()
                        .on_scroll(move |event: &ScrollEvent| {
                            offset.update(|offset| offset.1 = event.offset.y.0);
                        })
                        .child(when(
                            move || {
                                selected().is_some_and(|m| available(&m)) && !image_failed.get()
                            },
                            move || {
                                view()
                                    .flex_none()
                                    .w(canvas_width)
                                    .h(move || px(geometry.get().3))
                                    .items_center()
                                    .justify_center()
                                    .claim_touches(TouchClaim::on_start().ask(
                                        move |request: &ClaimRequest| {
                                            if zoom.get_untracked() > 1.0 {
                                                request.claim();
                                            }
                                        },
                                    ))
                                    .keep_touches(true)
                                    .on_responder_grant(move |event: &ResponderEvent| {
                                        drag.replace(Some((
                                            (event.page.x.0, event.page.y.0),
                                            offset.get_untracked(),
                                        )));
                                    })
                                    .on_responder_move(move |event: &ResponderEvent| {
                                        let Some((start, origin)) = drag.get() else {
                                            return;
                                        };
                                        let (_, _, width, height) = geometry.get_untracked();
                                        let (vw, vh) = viewport.get_untracked();
                                        let next = (
                                            (origin.0 - (event.page.x.0 - start.0))
                                                .clamp(0.0, (width - vw).max(0.0)),
                                            (origin.1 - (event.page.y.0 - start.1))
                                                .clamp(0.0, (height - vh).max(0.0)),
                                        );
                                        scroll_to(next);
                                    })
                                    .on_responder_release(move |_: &ResponderEvent| {
                                        drag.replace(None);
                                    })
                                    .on_responder_terminate(move |_: &ResponderEvent| {
                                        drag.replace(None);
                                    })
                                    .child(
                                        image(move || {
                                            selected().and_then(|m| m.media_url).unwrap_or_default()
                                        })
                                        .flex_none()
                                        .w(move || px(geometry.get().0))
                                        .h(move || px(geometry.get().1))
                                        .rotate(move || deg(f32::from(rotation.get()) * 90.0))
                                        .object_fit(ResizeMode::Contain)
                                        .label(move || i18n.text(Key::Photo))
                                        .on_error(move |_: &str| image_failed.set(true)),
                                    )
                            },
                        )),
                ),
        )
        .child(
            view()
                .flex_none()
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    ui::action(
                        move || i18n.tr("Rotate", "旋转"),
                        move || rotation.update(|value| *value = (*value + 1) % 4),
                    )
                    .disabled(move || {
                        busy.get()
                            || loading.get()
                            || image_failed.get()
                            || selected().is_none_or(|row| !available(&row))
                    }),
                )
                .child(when(
                    move || source.with(|value| value.is_none()),
                    move || {
                        ui::action(
                            move || i18n.tr("Forward…", "转发…"),
                            move || {
                                if !forward_allowed.get_untracked() {
                                    return;
                                }
                                if let Some(row) = selected() {
                                    actions.close_preview();
                                    actions.combine.set(false);
                                    actions.forwarding.set(vec![row.message_id]);
                                }
                            },
                        )
                        .disabled(move || busy.get() || loading.get() || !forward_allowed.get())
                    },
                ))
                .child(
                    ui::action(move || i18n.tr("Previous photo", "上一张"), move || go(-1))
                        .disabled(move || loading.get() || neighbor(-1).is_none()),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Zoom out", "缩小"),
                        move || zoom.update(|z| *z = (*z / 1.25).max(1.0)),
                    )
                    .disabled(move || zoom.get() <= 1.0),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Actual fit", "适应窗口"),
                        move || zoom.set(1.0),
                    )
                    .child(
                        text(move || format!("{:.0}%", zoom.get() * 100.0))
                            .text_role(ui::TextRole::Caption),
                    ),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Zoom in", "放大"),
                        move || zoom.update(|z| *z = (*z * 1.25).min(8.0)),
                    )
                    .disabled(move || zoom.get() >= 8.0),
                )
                .child(
                    ui::action(move || i18n.tr("Next photo", "下一张"), move || go(1))
                        .disabled(move || loading.get() || neighbor(1).is_none()),
                )
                .child(when(
                    move || selected().is_some_and(|row| !available(&row)) || image_failed.get(),
                    move || {
                        ui::action(
                            move || i18n.tr("Download photo", "下载图片"),
                            move || {
                                let Some(row) = selected() else {
                                    return;
                                };
                                let client = actions.account.with(|a| a.clone());
                                let transcript = source.with(|source| source.clone());
                                busy.set(true);
                                spawn(async move {
                                    let result = runtime::commit(async move {
                                        if let Some(parent) = transcript {
                                            client
                                                .attachment()
                                                .download_transcript_attachment(
                                                    parent,
                                                    row.message_id,
                                                )
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
                            },
                        )
                        .disabled(move || busy.get() || loading.get())
                    },
                ))
                .child(
                    ui::action(
                        move || i18n.tr("Copy image", "复制图片"),
                        move || {
                            let Some(row) = selected() else {
                                return;
                            };
                            let client = actions.account.with(|a| a.clone());
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
                                    }
                                    Err(_) => actions.failed.set(true),
                                }
                                busy.set(false);
                            });
                        },
                    )
                    .disabled(move || {
                        busy.get() || loading.get() || selected().is_none_or(|m| !available(&m))
                    }),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Save as…", "另存为…"),
                        move || {
                            let Some(row) = selected() else {
                                return;
                            };
                            let client = actions.account.with(|a| a.clone());
                            let transcript = source.with(|source| source.clone());
                            busy.set(true);
                            spawn(async move {
                                let name = super::message_actions::attachment_name(&row);
                                match pick_save_path(SaveDialog::new().file_name(name)).await {
                                    Ok(Some(path)) => actions.failed.set(
                                        runtime::commit(async move {
                                            if let Some(root) = transcript {
                                                return client
                                                    .attachment()
                                                    .save_transcript_attachment_as(
                                                        root,
                                                        row.message_id,
                                                        path.to_string_lossy().into_owned(),
                                                    )
                                                    .await;
                                            }
                                            client
                                                .attachment()
                                                .save_attachment_as(
                                                    row.message_id,
                                                    path.to_string_lossy().into_owned(),
                                                )
                                                .await
                                        })
                                        .await
                                        .is_err(),
                                    ),
                                    Ok(None) => {}
                                    Err(_) => actions.failed.set(true),
                                }
                                busy.set(false);
                            });
                        },
                    )
                    .disabled(move || {
                        busy.get() || loading.get() || selected().is_none_or(|m| !available(&m))
                    }),
                ),
        )
        .child(when(
            move || {
                selected()
                    .and_then(|row| row.caption)
                    .is_some_and(|caption| !caption.is_empty())
            },
            move || {
                scroll().flex_none().max_h_20().child(
                    text(move || selected().and_then(|row| row.caption).unwrap_or_default())
                        .px_4()
                        .pb_3()
                        .text_role(ui::TextRole::Body)
                        .select_text(),
                )
            },
        ))
}
