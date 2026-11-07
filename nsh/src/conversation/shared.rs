use mixin_desktop_api::MessageListView;
use nsh_base::tabs;
use nsh_core::element::{ViewableItems, desktop::MenuItem};

use super::*;
use crate::{chat::MediaPreview, presentation};

const PAGE_SIZE: usize = 60;
// The public query accepts at most 200 rows, including our next-page sentinel.
const WINDOW_SIZE: usize = PAGE_SIZE * 3;

#[derive(Clone, Copy)]
struct Shared {
    owner: Owner,
    kind: &'static str,
    rows: Signal<Vec<MessageListView>>,
    offset: Signal<usize>,
    count: Signal<usize>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    more: Signal<bool>,
    revision: Signal<u64>,
    request: Signal<u64>,
    first: Stored<Option<String>>,
    deferred: Stored<Option<bool>>,
    list: ElementRef,
    preview: MediaPreview,
}

impl Shared {
    fn page(self, older: bool) {
        if self.loading.get_untracked() {
            self.deferred.replace(Some(older));
            return;
        }
        if self.failed.get_untracked() {
            return;
        }
        if older && self.more.get_untracked() {
            batch(|| {
                if self.count.get_untracked() < WINDOW_SIZE {
                    self.count.update(|count| *count += PAGE_SIZE);
                } else {
                    self.offset.update(|offset| *offset += PAGE_SIZE);
                }
            });
        } else if !older && self.offset.get_untracked() > 0 {
            self.offset
                .update(|offset| *offset = offset.saturating_sub(PAGE_SIZE));
        }
    }

    fn available(message: &MessageListView) -> bool {
        matches!(message.media_status.as_str(), "DONE" | "READ")
            && message
                .media_url
                .as_ref()
                .is_some_and(|path| !path.is_empty())
    }

    fn apply(self, rows: Vec<MessageListView>, ticket: u64) {
        let previous = self.rows.get_untracked();
        let anchor = self.first.with(|id| id.clone());
        if let Some(phase) =
            presentation::message_window::splice_phase(&previous, &rows, anchor.as_deref())
        {
            self.rows.set(phase);
            after_answer(move || {
                if self.request.get_untracked() == ticket {
                    self.apply(rows, ticket);
                }
            });
            return;
        }
        self.rows.set(rows);
        self.loading.set(false);
        if let Some(older) = self.deferred.with(|edge| edge.take()) {
            self.page(older);
        }
    }
}

pub(super) fn screen(state: State, target: Signal<Option<String>>) -> impl IntoElement {
    let i18n = i18n::current();
    let selected = signal("media");
    let shown = signal(vec!["media"]);
    let preview = MediaPreview::new(state.client());
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .children(["media", "post", "file"].into_iter().map(move |kind| {
            when(
                move || shown.with(|shown| shown.contains(&kind)),
                move || {
                    view()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .style_when(
                            move || selected.get() != kind,
                            |style| style.hidden().invisible(),
                        )
                        .child(panel(state, target, kind, preview))
                },
            )
        }))
        .child(
            tabs()
                .flex_row()
                .flex_shrink_0()
                .min_h(ui::row_height())
                .border_t_1()
                .border_color(ui::border)
                .children(
                    [
                        ("media", "Media", "媒体"),
                        ("post", "Posts", "文章"),
                        ("file", "Files", "文件"),
                    ]
                    .into_iter()
                    .map(move |(value, en, zh)| {
                        ui::tab(move || i18n.tr(en, zh), move || selected.get() == value).on_press(
                            move || {
                                batch(|| {
                                    shown.update(|shown| {
                                        if !shown.contains(&value) {
                                            shown.push(value);
                                        }
                                    });
                                    selected.set(value);
                                });
                            },
                        )
                    }),
                ),
        )
        .child(preview.surface())
}

fn panel(
    state: State,
    target: Signal<Option<String>>,
    kind: &'static str,
    preview: MediaPreview,
) -> impl IntoElement {
    let i18n = i18n::current();
    let shared = Shared {
        owner: Owner::current(),
        kind,
        rows: signal_with(Vec::new(), SignalEquals::Never),
        offset: signal(0),
        count: signal(PAGE_SIZE),
        loading: signal(true),
        failed: signal(false),
        more: signal(false),
        revision: signal(0),
        request: signal(0),
        first: stored(None),
        deferred: stored(None),
        list: element_ref(),
        preview,
    };
    effect(move || {
        let kind = shared.kind;
        let offset = shared.offset.get();
        let count = shared.count.get();
        shared.revision.get();
        let ticket = shared.request.get_untracked().wrapping_add(1);
        shared.request.set(ticket);
        let client = state.client();
        let id = state.id();
        shared.loading.set(true);
        shared.failed.set(false);
        spawn(async move {
            let result = runtime::run(async move {
                client
                    .message()
                    .shared_messages(id, kind.into(), offset, count + 1)
                    .await
            })
            .await;
            if shared.request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok(mut values) => {
                    shared.more.set(values.len() > count);
                    values.truncate(count);
                    shared.apply(values, ticket);
                }
                Err(_) => {
                    shared.failed.set(true);
                    shared.loading.set(false);
                    shared.deferred.replace(None);
                }
            }
        });
    });
    let account = state.client();
    // The stream covers every conversation and every attachment state change.
    spawn(async move {
        parts::throttled(
            Box::pin(runtime::observe(account.message_changes())),
            move || shared.revision.update(|revision| *revision += 1),
        )
        .await;
    });
    let navigation = use_context::<crate::sidebar::Navigation>().expect("Home provides navigation");
    // A memo, so resizing the window rebuilds the list only when the count changes.
    let columns = computed(move || {
        let content = window().width.0 - navigation.width().0;
        let panel = if content < 620.0 {
            content
        } else {
            content - ui::sidebar_width().0
        };
        if panel < 620.0 { 4 } else { 3 }
    });
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .child(when(
            move || shared.loading.get() && shared.rows.with(Vec::is_empty),
            move || {
                parts::status(move || i18n.tr("Loading…", "加载中…"))
                    .px(ui::space_lg())
                    .py(ui::space_sm())
            },
        ))
        .child(when(
            move || shared.failed.get() || shared.preview.failed.get(),
            move || {
                parts::failure(
                    move || i18n.text(crate::i18n::Key::CommandFailed),
                    move || {
                        shared.preview.failed.set(false);
                        shared.revision.update(|revision| *revision += 1);
                    },
                )
                .px(ui::space_lg())
                .py(ui::space_sm())
            },
        ))
        .child(when(
            move || {
                !shared.loading.get() && !shared.failed.get() && shared.rows.with(Vec::is_empty)
            },
            move || {
                ui::placeholder(move || match kind {
                    "post" => i18n.tr("No posts", "暂无文章"),
                    "file" => i18n.tr("No files", "暂无文件"),
                    _ => i18n.tr("No shared media", "暂无共享媒体"),
                })
            },
        ))
        .child(when(
            move || !shared.rows.with(Vec::is_empty),
            move || {
                dynamic(move || {
                    let kind = shared.kind;
                    let columns = if kind == "media" { columns.get() } else { 1 };
                    untracked(move || {
                        let first = shared
                            .first
                            .with(|anchor| {
                                anchor.as_ref().and_then(|anchor| {
                                    shared.rows.with(|rows| {
                                        rows.iter().position(|row| &row.message_id == anchor)
                                    })
                                })
                            })
                            .unwrap_or_default();
                        virtual_list(
                            move || shared.rows.with(Vec::len),
                            move |at| {
                                let message =
                                    move || shared.rows.with(|rows| rows.get(at.index()).cloned());
                                item_menu(
                                    message,
                                    state,
                                    shared,
                                    target,
                                    if kind == "media" {
                                        media(message, shared).into_element()
                                    } else {
                                        view()
                                            .min_w_0()
                                            .w_full()
                                            .px(ui::space_md())
                                            .py(ui::space_sm())
                                            .gap(ui::space_sm())
                                            .child(dynamic(move || {
                                                let message = message();
                                                untracked(move || match message {
                                                    Some(message) => view()
                                                        .p(ui::space_sm())
                                                        .min_w_0()
                                                        .style_when(
                                                            move || kind == "post",
                                                            |style| {
                                                                style
                                                                    .bg(ui::surface_card)
                                                                    .rounded(ui::radius_md())
                                                                    .max_h_80()
                                                                    .overflow_hidden()
                                                            },
                                                        )
                                                        .child(shared.preview.content(message))
                                                        .into_element(),
                                                    None => view().hidden().into_element(),
                                                })
                                            }))
                                            .child(
                                                text(move || {
                                                    message()
                                                        .map(|row| {
                                                            format!(
                                                                "{} · {}",
                                                                row.sender_name,
                                                                presentation::time(
                                                                    i18n,
                                                                    row.created_at_micros / 1000
                                                                )
                                                            )
                                                        })
                                                        .unwrap_or_default()
                                                })
                                                .text_role(ui::TextRole::Caption)
                                                .number_of_lines(1)
                                                .text_color(ui::fg_secondary),
                                            )
                                            .into_element()
                                    },
                                )
                            },
                        )
                        .keys(move || {
                            shared.rows.with(|rows| {
                                rows.iter()
                                    .map(|row| row.message_id.clone())
                                    .collect::<Vec<_>>()
                            })
                        })
                        .columns(columns)
                        .row_gap(ui::space_xs())
                        .column_gap(ui::space_xs())
                        .estimated_item_size(ui::list_row_height())
                        .on_viewable_items_change(move |items: &ViewableItems| {
                            shared.first.replace(items.viewable.and_then(|(first, _)| {
                                shared
                                    .rows
                                    .with(|rows| rows.get(first).map(|row| row.message_id.clone()))
                            }));
                        })
                        .on_load_more(move || shared.page(true))
                        .on_load_more_at_start(move || shared.page(false))
                        .load_more_threshold(0.5)
                        .load_more_at_start_threshold(0.5)
                        .element_ref(&shared.list)
                        .initial_scroll_index(first)
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                    })
                })
            },
        ))
}

fn media(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    shared: Shared,
) -> impl IntoElement {
    let i18n = i18n::current();
    let progress = use_context::<crate::home::AttachmentProgress>()
        .expect("Home provides attachment progress");
    let thumbnail =
        crate::chat::message_media::thumbnail(move || message().and_then(|row| row.thumb_image));
    ui::command()
        .p_0()
        .min_h_0()
        .min_w_0()
        .w_full()
        .aspect_ratio(1.0)
        .overflow_hidden()
        .bg(ui::surface_card)
        .on_press(move || {
            if let Some(row) = message() {
                shared.preview.open(row);
            }
        })
        .accessibility_label(move || {
            Some(
                message()
                    .map(|row| presentation::preview(i18n, &row.category, &row.content))
                    .unwrap_or_default()
                    .into(),
            )
        })
        .child(
            image(move || {
                message()
                    .map(|row| {
                        let available = Shared::available(&row);
                        (available && row.category.ends_with("_IMAGE"))
                            .then(|| row.media_url.clone())
                            .flatten()
                            .or(row
                                .thumb_url
                                .filter(|url| !url.is_empty())
                                .or_else(|| thumbnail().map(|source| source.to_string())))
                            .or_else(|| available.then_some(row.media_url).flatten())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default()
            })
            .w_full()
            .h_full()
            .object_fit(ResizeMode::Cover)
            .label(move || i18n.tr("Shared media preview", "共享媒体预览")),
        )
        .child(
            view()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .p(ui::space_xs())
                .bg(ui::media_overlay)
                .text_color(ui::media_overlay_foreground)
                .style_when(
                    move || {
                        message().is_none_or(|row| {
                            Shared::available(&row) && !row.category.ends_with("_VIDEO")
                        })
                    },
                    |style| style.hidden(),
                )
                .child(
                    text(move || {
                        message()
                            .map(|row| {
                                if let Some((done, total)) = progress.bytes(None, &row.message_id) {
                                    if total > 0 {
                                        return format!(
                                            "{:.0}%",
                                            (done as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                                        );
                                    }
                                    return i18n.tr("Transferring…", "正在传输…").into();
                                }
                                if !Shared::available(&row) {
                                    return if matches!(
                                        row.media_status.as_str(),
                                        "PENDING" | "PROCESSING"
                                    ) {
                                        i18n.tr("Cancel transfer", "取消传输")
                                    } else {
                                        i18n.tr("Download", "下载")
                                    }
                                    .into();
                                }
                                if row.category.ends_with("_VIDEO") {
                                    let seconds =
                                        row.media_duration.parse::<u64>().unwrap_or_default()
                                            / 1000;
                                    return format!("▶ {}:{:02}", seconds / 60, seconds % 60);
                                }
                                String::new()
                            })
                            .unwrap_or_default()
                    })
                    .text_role(ui::TextRole::Caption),
                ),
        )
}

fn item_menu(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    state: State,
    shared: Shared,
    target: Signal<Option<String>>,
    body: AnyElement,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::native_context_menu(body, move || {
        let Some(message) = message() else {
            return Vec::new();
        };
        let locate = message.message_id.clone();
        let mut file = Vec::new();
        if Shared::available(&message) {
            let save = message.clone();
            file.push(
                MenuItem::new(i18n.tr("Save as", "另存为"))
                    .on_select(move || shared.preview.save(save.clone())),
            );
            if let Some(path) = message.media_url {
                file.push(
                    MenuItem::new(i18n.tr("Show in folder", "在文件夹中显示")).on_select(
                        move || {
                            let path = path.clone();
                            // The list cell showing this menu can go before the reveal completes.
                            shared.owner.run(|| {
                                spawn(async move {
                                    shared.preview.failed.set(reveal_path(path).await.is_err());
                                })
                            });
                        },
                    ),
                );
            }
        }
        ui::menu_groups([
            vec![
                MenuItem::new(i18n.tr("Locate in Chat", "前往聊天")).on_select(move || {
                    target.set(Some(locate.clone()));
                    state.close();
                }),
            ],
            file,
        ])
    })
}
