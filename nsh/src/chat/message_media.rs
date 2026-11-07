//! Image/video cards share transfers and account-owned previews with the timeline.
use std::sync::Arc;

use mixin_desktop_api::{MediaClient, MessageListView};
use nsh_base::theme;
use nsh_core::prelude::*;

use super::{message_actions::Actions, message_content::available};
use crate::{appearance, i18n::Key, presentation, runtime, ui};

#[derive(Clone, Copy)]
struct Viewport(Signal<f32>);

/// Install once on an actual message region, never by polling individual cells.
pub(crate) fn viewport() -> impl Fn(&Bounds<Points>) + Copy + 'static {
    let width = signal(640.0_f32);
    provide_context(Viewport(width));
    move |bounds: &Bounds<Points>| width.set(bounds.size.width.0.max(1.0))
}

/// The source memo prevents unrelated message/progress updates from decoding again.
pub(crate) fn thumbnail(
    source: impl Fn() -> Option<String> + 'static,
) -> impl Fn() -> Option<SharedString> + Copy + 'static {
    let source = computed(source);
    let loaded = signal(None::<SharedString>);
    let owner = Owner::current();
    let media = use_context::<Stored<Option<Arc<MediaClient>>>>()
        .expect("Home provides media")
        .with(|media| media.as_ref().expect("Media is initialized").clone());
    effect(move || {
        let data = source.get();
        loaded.set(None);
        let Some(data) = data.filter(|data| !data.is_empty()) else {
            return;
        };
        let media = media.clone();
        let expected = data.clone();
        spawn(async move {
            let result = runtime::run(async move { media.thumbnail_image(data).await }).await;
            if owner.is_live()
                && source.get_untracked().as_deref() == Some(expected.as_str())
                && let Ok(value) = result
            {
                loaded.set(value.map(Into::into));
            }
        });
    });
    move || loaded.get()
}

pub(crate) fn inline_metadata(row: &MessageListView) -> bool {
    row.category.ends_with("_VIDEO")
        || (row.category.ends_with("_IMAGE")
            && row
                .caption
                .as_deref()
                .is_none_or(|caption| caption.trim().is_empty()))
}

fn overlay_color() -> nsh_core::element::AdaptiveColor {
    theme()
        .color("media-overlay")
        .unwrap_or(theme().foreground.opacity(0.6))
}

fn overlay_foreground() -> nsh_core::element::AdaptiveColor {
    theme()
        .color("media-overlay-foreground")
        .unwrap_or(theme().foreground)
}

pub(super) fn card(
    row: MessageListView,
    transcript: Option<String>,
    actions: Actions,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let viewport = use_context::<Viewport>();
    let video = row.category.ends_with("_VIDEO");
    let done = available(&row);
    let pending = matches!(row.media_status.as_str(), "PENDING" | "PROCESSING");
    let expired = row.media_status == "EXPIRED";
    let metadata = transcript.is_none() && inline_metadata(&row);
    let mine = row.sender_id == actions.account.with(|account| account.account_id());
    let retry_upload = mine && matches!(row.status.as_str(), "SENDING" | "FAILED");
    let (width, height) = match (row.media_width, row.media_height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => (width as f32, height as f32),
        _ => (200.0, 200.0),
    };
    let source_aspect = width / height;
    let aspect = if (0.01..=100.0).contains(&source_aspect) {
        source_aspect
    } else {
        1.0
    };
    let size = move || {
        let available = viewport.map_or(640.0, |viewport| viewport.0.get()).max(1.0);
        let maximum = (available * 0.6).min(if video { 200.0 } else { 300.0 });
        let width = if video {
            width.min(maximum)
        } else {
            (width / pixel_ratio())
                .min(maximum)
                .max((available * 0.2).max(200.0))
        }
        .min((available - 64.0).max(1.0));
        // The reference caps long images; the preview preserves the full source.
        let height = if video {
            width / aspect
        } else {
            (width / aspect).min(window().height.0 * 2.0 / 3.0)
        }
        .max(1.0);
        (width.max(1.0), height)
    };
    let row = stored(row);
    let root = stored(transcript);
    let preview = thumbnail(move || row.with(|row| row.thumb_image.clone()));
    let progress = use_context::<crate::home::AttachmentProgress>()
        .expect("Home provides attachment progress");
    let bytes =
        move || root.with(|root| row.with(|row| progress.bytes(root.as_deref(), &row.message_id)));
    let transferring = move || pending || bytes().is_some();
    let submitting = signal(false);
    let local_failed = signal(false);
    let remote_failed = signal(false);
    let full_loaded = signal(false);
    let remote = row.with(|row| {
        row.thumb_url
            .clone()
            .filter(|url| web_image(url))
            .or_else(|| {
                (row.media_mime_type.as_deref() == Some("image/gif")
                    && row.media_size.unwrap_or_default() == 0)
                    .then(|| row.thumb_image.clone().filter(|url| web_image(url)))
                    .flatten()
            })
    });
    let remote = stored(remote);
    let owner = Owner::current();
    ui::command()
        .p_0()
        .min_h_0()
        .min_w_0()
        .max_w(relative(1.0))
        .w(move || px(size().0))
        .aspect_ratio(move || size().0 / size().1)
        .disabled(move || expired || submitting.get())
        .accessibility_label(move || {
            Some(
                if expired {
                    i18n.tr("Expired", "已过期")
                } else if transferring() {
                    i18n.tr("Cancel transfer", "取消传输")
                } else if done {
                    if video {
                        i18n.text(Key::Play)
                    } else {
                        i18n.text(Key::Photo)
                    }
                } else if retry_upload {
                    i18n.tr("Retry", "重试")
                } else {
                    i18n.tr("Download", "下载")
                }
                .into(),
            )
        })
        .on_press(move || {
            if expired || submitting.get_untracked() {
                return;
            }
            if done {
                let message = row.with(|value| value.clone());
                if video {
                    actions.open_preview(message);
                } else {
                    actions.open_image(message, root.with(|value| value.clone()));
                }
                return;
            }
            let cancel = transferring();
            let client = actions.account.with(|value| value.clone());
            let id = row.with(|row| row.message_id.clone());
            let transcript = root.with(|value| value.clone());
            submitting.set(true);
            let command = runtime::commit(async move {
                if cancel {
                    if let Some(root) = transcript {
                        client
                            .attachment()
                            .cancel_transcript_attachment(root, id)
                            .await
                    } else {
                        client.attachment().cancel_attachment(id).await
                    }
                } else {
                    client.attachment().resume_attachment(id, transcript).await
                }
            });
            spawn(async move {
                let result = command.await;
                if owner.is_live() {
                    submitting.set(false);
                    actions.failed.set(result.is_err());
                }
            });
        })
        .child(
            view()
                .w_full()
                .h_full()
                .relative()
                .rounded_lg()
                .overflow_hidden()
                .bg(move || theme().muted)
                .child(
                    view()
                        .absolute()
                        .inset_0()
                        .items_center()
                        .justify_center()
                        .child(
                            text(move || {
                                if video {
                                    i18n.text(Key::Video)
                                } else {
                                    i18n.text(Key::Photo)
                                }
                            })
                            .text_size(move || appearance.secondary_size())
                            .text_color(move || theme().muted_foreground),
                        ),
                )
                .child(when(
                    move || preview().is_some(),
                    move || {
                        image(move || preview().unwrap_or_default())
                            .absolute()
                            .inset_0()
                            .w_full()
                            .h_full()
                            .object_fit(ResizeMode::Cover)
                    },
                ))
                .child(when(
                    move || remote.with(|value| value.is_some()),
                    move || {
                        image(remote.with(|value| value.clone()).unwrap_or_default())
                            .absolute()
                            .inset_0()
                            .w_full()
                            .h_full()
                            .object_fit(ResizeMode::Cover)
                            .style_when(remote_failed, |style| style.hidden())
                            .on_error(move |_: &str| remote_failed.set(true))
                    },
                ))
                .child(when(
                    move || done && !video,
                    move || {
                        image(row.with(|row| row.media_url.clone().unwrap_or_default()))
                            .absolute()
                            .inset_0()
                            .w_full()
                            .h_full()
                            .object_fit(ResizeMode::Cover)
                            .fade_duration(|| crate::motion::duration(100.0))
                            .style_when(
                                move || !full_loaded.get() || local_failed.get(),
                                |style| style.opacity(0.0),
                            )
                            .on_load(move |_: &Size<DevicePixels>| full_loaded.set(true))
                            .on_error(move |_: &str| local_failed.set(true))
                    },
                ))
                .child(when(
                    move || !done || video || local_failed.get(),
                    move || {
                        view()
                            .absolute()
                            .inset_0()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .child(
                                view()
                                    .min_w_11()
                                    .min_h_11()
                                    .p_2()
                                    .rounded_full()
                                    .bg(overlay_color)
                                    .items_center()
                                    .justify_center()
                                    .text_color(overlay_foreground)
                                    .child(when(transferring, move || {
                                        view()
                                            .flex_row()
                                            .gap_1()
                                            .items_center()
                                            .child(ui::busy_indicator(
                                                move || bytes().is_none_or(|(_, total)| total == 0),
                                                move || {
                                                    i18n.tr("Transferring…", "正在传输…").into()
                                                },
                                            ))
                                            .child(ui::icon(ui::Icon::Close).size_5())
                                            .child(when(
                                                move || bytes().is_some_and(|(_, total)| total > 0),
                                                move || {
                                                    text(move || {
                                                        bytes()
                                                            .map(|(done, total)| {
                                                                format!(
                                                                    "{:.0}%",
                                                                    (done as f64
                                                                        / total.max(1) as f64
                                                                        * 100.0)
                                                                        .clamp(0.0, 100.0)
                                                                )
                                                            })
                                                            .unwrap_or_default()
                                                    })
                                                    .text_size(move || appearance.tertiary_size())
                                                },
                                            ))
                                    }))
                                    .child(when(
                                        move || !transferring(),
                                        move || {
                                            if expired || local_failed.get() {
                                                text("!").text_lg().into_element()
                                            } else {
                                                ui::icon(if done {
                                                    ui::Icon::Play
                                                } else if retry_upload {
                                                    ui::Icon::Upload
                                                } else {
                                                    ui::Icon::Download
                                                })
                                                .size_6()
                                                .into_element()
                                            }
                                        },
                                    )),
                            )
                            .child(when(local_failed, move || {
                                text(move || {
                                    i18n.tr("Couldn’t display this photo.", "无法显示此图片。")
                                })
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(overlay_color)
                                .text_color(overlay_foreground)
                                .text_size(move || appearance.tertiary_size())
                                .number_of_lines(2)
                            }))
                            .child(when(
                                move || expired,
                                move || {
                                    text(move || i18n.tr("Expired", "已过期"))
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .bg(overlay_color)
                                        .text_color(overlay_foreground)
                                        .text_size(move || appearance.tertiary_size())
                                },
                            ))
                    },
                ))
                .child(when(
                    move || video,
                    move || {
                        view()
                            .absolute()
                            .top_1()
                            .left_1()
                            .px_1()
                            .py_1()
                            .rounded_md()
                            .bg(overlay_color)
                            .child(
                                text(row.with(|row| duration(&row.media_duration)))
                                    .text_color(overlay_foreground)
                                    .text_size(move || appearance.tertiary_size()),
                            )
                    },
                ))
                .child(when(
                    move || metadata,
                    move || {
                        view()
                            .absolute()
                            .bottom_1()
                            .right_1()
                            .px_1()
                            .py_1()
                            .rounded_full()
                            .bg(overlay_color)
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(when(
                                move || {
                                    row.with(|row| {
                                        row.category.starts_with("SIGNAL_")
                                            || row.category.starts_with("ENCRYPTED_")
                                    })
                                },
                                move || {
                                    ui::icon(ui::Icon::Lock)
                                        .size_3()
                                        .text_color(overlay_foreground)
                                },
                            ))
                            .child(
                                text(move || {
                                    row.with(|row| {
                                        presentation::time(i18n, row.created_at_micros / 1000)
                                    })
                                })
                                .text_size(move || appearance.tertiary_size())
                                .text_color(overlay_foreground),
                            )
                            .child(when(
                                move || mine,
                                move || {
                                    dynamic(move || {
                                        row.with(|row| {
                                            appearance::delivery_icon_on_media(&row.status)
                                        })
                                    })
                                },
                            ))
                    },
                )),
        )
}

fn web_image(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

fn duration(value: &str) -> String {
    let seconds = value.parse::<u64>().unwrap_or_default() / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
