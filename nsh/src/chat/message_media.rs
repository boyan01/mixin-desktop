//! Image/video cards share transfers and account-owned previews with the timeline.

use mixin_desktop_api::MessageListView;
use nsh_core::{element::ImageSrc, prelude::*};

use super::{message_actions::Actions, message_appearance, message_content::available};
use crate::{i18n::Key, runtime, ui};

#[derive(Clone, Copy)]
struct Viewport(Signal<f32>);

/// Install once on an actual message region, never by polling individual cells.
pub(crate) fn viewport() -> impl Fn(&Bounds<Points>) + Copy + 'static {
    let width = signal(640.0_f32);
    provide_context(Viewport(width));
    move |bounds: &Bounds<Points>| width.set(bounds.size.width.0.max(1.0))
}

/// The width of the message region, for what Flutter sizes against it.
pub(super) fn region() -> impl Fn() -> f32 + Copy + 'static {
    let viewport = use_context::<Viewport>();
    move || viewport.map_or(640.0, |viewport| viewport.0.get()).max(1.0)
}

/// The source memo prevents unrelated message/progress updates from decoding again.
pub(crate) fn thumbnail(
    source: impl Fn() -> Option<String> + 'static,
) -> impl Fn() -> Option<SharedString> + Copy + 'static {
    let source = computed(source);
    let loaded = signal(None::<SharedString>);
    let owner = Owner::current();
    let media = crate::media_client();
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

/// One attachment's transfer: its queued state, live bytes and toggle command.
#[derive(Clone, Copy)]
pub(super) struct Transfer {
    id: Stored<String>,
    root: Stored<Option<String>>,
    queued: bool,
    progress: crate::attachment_progress::AttachmentProgress,
}

impl Transfer {
    pub fn of(row: &MessageListView, transcript: Option<String>) -> Self {
        Self {
            id: stored(row.message_id.clone()),
            root: stored(transcript),
            queued: matches!(row.media_status.as_str(), "PENDING" | "PROCESSING"),
            progress: use_context::<crate::attachment_progress::AttachmentProgress>()
                .expect("Home provides attachment progress"),
        }
    }

    fn bytes(self) -> Option<(u64, u64)> {
        self.root.with(|root| {
            self.id
                .with(|id| self.progress.bytes(root.as_deref(), id.as_str()))
        })
    }

    pub fn active(self) -> bool {
        self.queued || self.bytes().is_some()
    }

    /// `None` until the transfer reports a total: never a made-up figure.
    pub fn percent(self) -> Option<String> {
        self.bytes()
            .filter(|(_, total)| *total > 0)
            .map(|(done, total)| {
                format!(
                    "{:.0}%",
                    (done as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                )
            })
    }

    /// Cancels an active transfer, else starts or resumes it.
    pub fn toggle(self, actions: Actions) {
        let cancel = self.active();
        let client = actions.account.with(|account| account.clone());
        let id = self.id.with(|id| id.clone());
        let root = self.root.with(|root| root.clone());
        actions.commit(async move {
            match (cancel, root) {
                (true, Some(root)) => {
                    client
                        .attachment()
                        .cancel_transcript_attachment(root, id)
                        .await
                }
                (true, None) => client.attachment().cancel_attachment(id).await,
                (false, root) => client.attachment().resume_attachment(id, root).await,
            }
        });
    }
}

fn overlay_label(
    label: impl Fn() -> &'static str + 'static,
    appearance: crate::appearance::Appearance,
) -> impl IntoElement {
    text(label)
        .px_2()
        .py_1()
        .rounded(ui::radius_sm())
        .bg(ui::media_overlay)
        .text_color(ui::media_overlay_foreground)
        .text_size(move || appearance.tertiary_size())
        .number_of_lines(2)
}

/// `status` follows delivery without rebuilding the card; only a timeline row has it.
pub(super) fn card(
    row: MessageListView,
    transcript: Option<String>,
    actions: Actions,
    status: Option<Memo<String>>,
    framed: bool,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let region = region();
    let video = row.category.ends_with("_VIDEO");
    let done = available(&row);
    let transfer = Transfer::of(&row, transcript.clone());
    let expired = row.media_status == "EXPIRED";
    let metadata = (transcript.is_none() || framed) && inline_metadata(&row);
    let mine = row.sender_id == actions.account.id();
    let retry_upload = mine && matches!(row.status.as_str(), "SENDING" | "FAILED");
    let delivery = (mine && transcript.is_none()).then(|| {
        status.unwrap_or_else(|| {
            let status = row.status.clone();
            computed(move || status.clone())
        })
    });
    // Flutter sizes a bare picture with its nip's gutter, which stays empty.
    let gutter = if framed
        && message_appearance::Frame::of(&row).inset == message_appearance::Inset::Flush
    {
        ui::bubble_nip_width().0
    } else {
        0.0
    };
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
        let available = region();
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
        ((width - gutter).max(1.0), height)
    };
    let row = stored(row);
    let root = stored(transcript);
    let preview = thumbnail(move || row.with(|row| row.thumb_image.clone()));
    let transferring = move || transfer.active();
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
    ui::command()
        .p_0()
        .min_h_0()
        .min_w_0()
        .max_w(relative(1.0))
        // Sized from the picture's own dimensions, within the viewport.
        .w(move || px(size().0))
        .aspect_ratio(move || size().0 / size().1)
        .disabled(move || expired)
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
            if expired {
                return;
            }
            if !done {
                transfer.toggle(actions);
            } else if video {
                actions.open_video(row.with(|value| value.clone()), root.get());
            } else {
                actions.open_image(
                    row.with(|value| value.clone()),
                    root.with(|value| value.clone()),
                );
            }
        })
        .child(
            view()
                .w_full()
                .h_full()
                .relative()
                // A framed picture takes the corners its bubble clips.
                .style_when(move || !framed, |style| style.rounded(ui::radius_md()))
                .overflow_hidden()
                .bg(ui::surface_card)
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
                            .text_color(ui::fg_secondary),
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
                                    .min_w(ui::status_disc())
                                    .min_h(ui::status_disc())
                                    .p_1()
                                    .rounded(ui::radius_full())
                                    .bg(ui::media_overlay)
                                    .items_center()
                                    .justify_center()
                                    .text_color(ui::media_overlay_foreground)
                                    .child(when(transferring, move || {
                                        view()
                                            .flex_row()
                                            .gap_1()
                                            .items_center()
                                            .child(ui::busy_indicator(
                                                move || transfer.percent().is_none(),
                                                move || {
                                                    i18n.tr("Transferring…", "正在传输…").into()
                                                },
                                            ))
                                            .child(ui::icon(ui::Icon::Close))
                                            .child(when(
                                                move || transfer.percent().is_some(),
                                                move || {
                                                    text(move || {
                                                        transfer.percent().unwrap_or_default()
                                                    })
                                                    .text_size(move || appearance.tertiary_size())
                                                },
                                            ))
                                    }))
                                    .child(when(
                                        move || !transferring(),
                                        move || {
                                            if expired || local_failed.get() {
                                                svg(ImageSrc::svg_markup(include_str!(
                                                    "../../assets/message/warning.svg"
                                                )))
                                                .size(ui::icon_lg())
                                                .text_color(ui::media_overlay_foreground)
                                                .into_element()
                                            } else {
                                                ui::icon(if done {
                                                    ui::Icon::Play
                                                } else if retry_upload {
                                                    ui::Icon::Upload
                                                } else {
                                                    ui::Icon::Download
                                                })
                                                .size(ui::icon_lg())
                                                .into_element()
                                            }
                                        },
                                    )),
                            )
                            .child(when(local_failed, move || {
                                overlay_label(
                                    move || {
                                        i18n.tr("Couldn’t display this photo.", "无法显示此图片。")
                                    },
                                    appearance,
                                )
                            }))
                            .child(when(
                                move || expired,
                                move || {
                                    overlay_label(move || i18n.tr("Expired", "已过期"), appearance)
                                },
                            ))
                    },
                ))
                .child(when(
                    move || video,
                    move || {
                        // Flutter's 6 from the picture's edge; 5 past an incoming nip.
                        let chip_left = if mine {
                            ui::space_xs() + ui::space_2xs()
                        } else {
                            ui::stamp_inset_x()
                        };
                        view()
                            .absolute()
                            .top(ui::space_xs() + ui::space_2xs())
                            .left(chip_left)
                            .p_1()
                            .rounded(ui::radius_sm())
                            .bg(ui::media_stamp)
                            .child(
                                text(row.with(|row| duration(&row.media_duration)))
                                    .text_color(ui::media_overlay_foreground)
                                    .text_size(move || appearance.tertiary_size()),
                            )
                    },
                ))
                .child(when(
                    move || metadata,
                    move || {
                        // Flutter's 12 and 4 from the end of the bubble's box.
                        let end = if mine {
                            ui::stamp_inset_y()
                        } else {
                            ui::space_xs()
                        };
                        row.with(|row| message_appearance::media_stamp(row, delivery))
                            .absolute()
                            .bottom_1()
                            .right(end)
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
