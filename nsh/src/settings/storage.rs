use std::{collections::HashSet, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ConversationStorageUsage, SettingsClient, StorageCategoryUsage,
};
use nsh_core::prelude::*;

use super::{Feedback, client};
use crate::{
    conversation::parts,
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

#[derive(Clone, Copy)]
enum Download {
    Photos,
    Videos,
    Files,
}

impl Download {
    async fn save(
        self,
        client: SettingsClient,
        checked: bool,
    ) -> mixin_desktop_api::ClientResult<()> {
        match self {
            Self::Photos => client.set_photo_auto_download(checked).await,
            Self::Videos => client.set_video_auto_download(checked).await,
            Self::Files => client.set_file_auto_download(checked).await,
        }
    }
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    title: impl Fn() -> &'static str + Copy + 'static,
    on_back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let selected = signal(None::<(String, String)>);
    let revision = signal(0_u64);
    view().flex_1().min_h_0().child(dynamic(move || {
        let target = selected.get();
        let account = account.clone();
        untracked(|| match target {
            Some((id, name)) => detail(account, id, name, move || {
                selected.set(None);
                revision.update(|r| *r += 1);
            })
            .into_element(),
            None => view()
                .flex_1()
                .min_h_0()
                .child(parts::page_header(title, parts::back_button(on_back)))
                .child(overview(account, selected, revision))
                .into_element(),
        })
    }))
}

fn overview(
    account: Arc<AccountClient>,
    selected: Signal<Option<(String, String)>>,
    revision: Signal<u64>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let rows = signal_with(Vec::<ConversationStorageUsage>::new(), SignalEquals::Never);
    let loading = signal(true);
    let failed = signal(false);
    let client = account;
    effect(move || {
        let _revision = revision.get();
        let client = client.clone();
        loading.set(true);
        failed.set(false);
        spawn(async move {
            match runtime::run(async move { client.storage_usage().await }).await {
                Ok(next) => rows.set(next),
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    parts::page_body()
        .flex_1()
        .min_h_0()
        .child(
            ui::section(move || i18n.tr("Automatic media download", "自动下载媒体"))
                .child(
                    ui::card()
                        .child(download_toggle(Download::Photos))
                        .child(download_toggle(Download::Videos))
                        .child(download_toggle(Download::Files)),
                )
                .child(parts::note(move || {
                    i18n.tr(
                        "Automatically download media when messages arrive.",
                        "收到消息时自动下载媒体文件。",
                    )
                })),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .gap(ui::space_sm())
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(ui::space_md())
                        .child(
                            ui::section_label(move || i18n.tr("Storage usage", "存储用量"))
                                .flex_1(),
                        )
                        .child(
                            text(move || {
                                bytes(rows.with(|rows| rows.iter().map(|r| r.size_bytes).sum()))
                            })
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::fg_secondary),
                        )
                        .child(
                            ui::command()
                                .disabled(loading)
                                .on_press(move || revision.update(|v| *v += 1))
                                .child(text(move || i18n.tr("Refresh", "刷新"))),
                        ),
                )
                .child(when(
                    move || loading.get() && rows.with(Vec::is_empty),
                    move || {
                        parts::status(move || i18n.tr("Calculating storage…", "正在计算存储用量…"))
                    },
                ))
                .child(when(failed, move || {
                    parts::failure(
                        move || i18n.text(Key::CommandFailed),
                        move || revision.update(|r| *r += 1),
                    )
                }))
                .child(when(
                    move || !loading.get() && !failed.get() && rows.with(Vec::is_empty),
                    move || parts::note(move || i18n.tr("No stored media", "暂无媒体缓存")),
                ))
                .child(
                    // Hugs its rows, and shrinks to scroll them when they outgrow the page.
                    ui::card()
                        .flex_shrink(1.0)
                        .min_h_0()
                        .style_when(move || rows.with(Vec::is_empty), |s| s.hidden())
                        .child(
                            virtual_list(
                                move || rows.with(Vec::len),
                                move |at| storage_row(rows, selected, at),
                            )
                            .keys(move || {
                                rows.with(|r| {
                                    r.iter()
                                        .map(|r| r.conversation.conversation_id.clone())
                                        .collect::<Vec<_>>()
                                })
                            })
                            .estimated_item_size(ui::row_height())
                            .h(move || px(rows.with(Vec::len) as f32 * ui::row_height().0))
                            .flex_shrink(1.0)
                            .min_h_0(),
                        ),
                ),
        )
}

fn storage_row(
    rows: Signal<Vec<ConversationStorageUsage>>,
    selected: Signal<Option<(String, String)>>,
    at: Position,
) -> impl IntoElement {
    ui::card_row()
        .child(ui::conversation_avatar(
            move || rows.with(|r| r.get(at.index()).map(|r| r.conversation.clone())),
            ui::AvatarSize::Medium,
        ))
        .child(
            text(move || {
                rows.with(|r| {
                    r.get(at.index())
                        .map(|r| r.conversation.name.clone())
                        .unwrap_or_default()
                })
            })
            .text_role(ui::TextRole::Label)
            .flex_1()
            .min_w_0()
            .number_of_lines(1),
        )
        .child(
            text(move || {
                rows.with(|r| {
                    r.get(at.index())
                        .map(|r| bytes(r.size_bytes))
                        .unwrap_or_default()
                })
            })
            .text_role(ui::TextRole::Body)
            .text_color(ui::fg_secondary),
        )
        .child(ui::chevron())
        .on_press(move || {
            if let Some(target) = rows.with(|r| {
                r.get(at.index()).map(|r| {
                    (
                        r.conversation.conversation_id.clone(),
                        r.conversation.name.clone(),
                    )
                })
            }) {
                selected.set(Some(target));
            }
        })
}

fn download_toggle(kind: Download) -> impl IntoElement {
    let i18n = i18n::current();
    let checked = signal(false);
    let ready = signal(false);
    let failed = signal(false);
    let busy = signal(false);
    let retry = signal(0_u64);
    effect(move || {
        let _revision = retry.get();
        failed.set(false);
        spawn(async move {
            let settings = match runtime::run(client()).await {
                Ok(settings) => settings,
                Err(_) => {
                    failed.set(true);
                    return;
                }
            };
            let changes: std::pin::Pin<
                Box<dyn futures::Stream<Item = mixin_desktop_api::ClientResult<bool>> + Send>,
            > = match kind {
                Download::Photos => Box::pin(settings.subscribe_photo_auto_download()),
                Download::Videos => Box::pin(settings.subscribe_video_auto_download()),
                Download::Files => Box::pin(settings.subscribe_file_auto_download()),
            };
            let mut changes = Box::pin(runtime::observe(changes));
            while let Some(next) = changes.next().await {
                match next {
                    Ok(value) => {
                        checked.set(value);
                        ready.set(true);
                        failed.set(false);
                    }
                    Err(_) => failed.set(true),
                }
            }
        });
    });
    view()
        .child(ui::toggle_row(
            move || match kind {
                Download::Photos => i18n.tr("Photos", "图片"),
                Download::Videos => i18n.tr("Videos", "视频"),
                Download::Files => i18n.tr("Files", "文件"),
            },
            move || checked.get(),
            move || !ready.get() || busy.get(),
            move |value| {
                let previous = checked.get_untracked();
                checked.set(value);
                busy.set(true);
                failed.set(false);
                let task = runtime::commit(async move { kind.save(client().await?, value).await });
                spawn(async move {
                    if task.await.is_err() {
                        checked.set(previous);
                        failed.set(true);
                    }
                    busy.set(false);
                });
            },
        ))
        .child(when(failed, move || {
            parts::failure(
                move || i18n.text(Key::CommandFailed),
                move || retry.update(|r| *r += 1),
            )
            .px(ui::space_lg())
            .pb(ui::space_sm())
        }))
}

fn detail(
    account: Arc<AccountClient>,
    id: String,
    name: String,
    on_back: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let rows = signal_with(Vec::<StorageCategoryUsage>::new(), SignalEquals::Never);
    let selected = signal(HashSet::<String>::new());
    let revision = signal(0_u64);
    let loading = signal(true);
    let load_failed = signal(false);
    let feedback = Feedback::new();
    let confirming = signal(false);
    let presence = crate::motion::Presence::modal(move || confirming.get());
    let clear_disabled = move || {
        loading.get()
            || feedback.busy.get()
            || !rows.with(|rows| {
                rows.iter().any(|row| {
                    row.size_bytes > 0 && selected.with(|selected| selected.contains(&row.category))
                })
            })
    };
    let client = account.clone();
    let conversation = id.clone();
    effect(move || {
        let _revision = revision.get();
        let client = client.clone();
        let id = conversation.clone();
        loading.set(true);
        load_failed.set(false);
        spawn(async move {
            match runtime::run(async move { client.conversation_storage_usage(id).await }).await {
                Ok(next) => rows.set(next),
                Err(_) => load_failed.set(true),
            }
            loading.set(false);
        });
    });
    let clear = move || {
        if !presence.interactive() || untracked(clear_disabled) {
            return;
        }
        let account = account.clone();
        let id = id.clone();
        let categories = selected.with(|s| s.iter().cloned().collect());
        feedback.start();
        let task =
            runtime::commit(
                async move { account.clear_conversation_storage(id, categories).await },
            );
        feedback.owner.run(|| {
            spawn(async move {
                let result = task.await;
                if result.is_ok() {
                    selected.set(HashSet::new());
                    confirming.set(false);
                }
                feedback.finish(result.is_ok());
                // A failed clear may have removed some categories already.
                revision.update(|v| *v += 1);
            })
        });
    };
    view()
        .flex_1()
        .min_h_0()
        .child(parts::page_header(
            name,
            parts::back_button(on_back).disabled(feedback.busy),
        ))
        .child(
            scroll().flex_1().min_h_0().child(
                parts::page_body()
                    .child(parts::note(move || {
                        i18n.tr(
                            "Clear cached media from this device. Messages stay in the conversation.",
                            "清除此设备上的媒体缓存，会话中的消息会保留。",
                        )
                    }))
                    .child(when(
                        move || loading.get() && rows.with(Vec::is_empty),
                        move || {
                            parts::status(move || {
                                i18n.tr("Calculating storage…", "正在计算存储用量…")
                            })
                        },
                    ))
                    .child(when(load_failed, move || {
                        parts::failure(
                            move || i18n.text(Key::CommandFailed),
                            move || revision.update(|v| *v += 1),
                        )
                    }))
                    .child(ui::card().children(["photos", "videos", "audio", "files"].into_iter().map(
                        move |category| {
                            category_row(category, rows, selected, move || {
                                loading.get() || feedback.busy.get()
                            })
                        },
                    )))
                    .child(
                        ui::card().child(
                            ui::settings_row(None, move || {
                                i18n.tr("Clear selected cache…", "清除选中的缓存…")
                            })
                            .text_color(ui::danger)
                            .disabled(clear_disabled)
                            .on_press(move || {
                                if !untracked(clear_disabled) {
                                    confirming.set(true);
                                }
                            }),
                        ),
                    )
                    .child(feedback.view(move || i18n.tr("Clearing cache…", "正在清除缓存…")))
                    .child(when(
                        move || presence.mounted(),
                        move || clear_dialog(presence, confirming, feedback, clear_disabled, clear.clone()),
                    )),
            ),
        )
}

fn category_row(
    category: &'static str,
    rows: Signal<Vec<StorageCategoryUsage>>,
    selected: Signal<HashSet<String>>,
    busy: impl Fn() -> bool + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let chosen = move || selected.with(|s| s.contains(category));
    let unavailable = move || {
        !rows.with(|rows| {
            rows.iter()
                .any(|row| row.category == category && row.size_bytes > 0)
        })
    };
    ui::checkbox_row(
        chosen,
        move || busy() || unavailable(),
        move |checked| {
            selected.update(|s| {
                if checked {
                    s.insert(category.into());
                } else {
                    s.remove(category);
                }
            })
        },
    )
    .child(
        text(move || match category {
            "photos" => i18n.tr("Photos", "图片"),
            "videos" => i18n.tr("Videos", "视频"),
            "audio" => i18n.tr("Audio", "音频"),
            _ => i18n.tr("Files", "文件"),
        })
        .flex_1()
        .text_role(ui::TextRole::Label),
    )
    .child(
        text(move || {
            rows.with(|rows| {
                rows.iter()
                    .find(|row| row.category == category)
                    .map(|row| bytes(row.size_bytes))
                    .unwrap_or_default()
            })
        })
        .text_role(ui::TextRole::Body)
        .text_color(ui::fg_secondary),
    )
}

fn clear_dialog(
    presence: crate::motion::Presence,
    confirming: Signal<bool>,
    feedback: Feedback,
    disabled: impl Fn() -> bool + 'static,
    clear: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::dialog(
        presence,
        move || {
            if !feedback.busy.get_untracked() {
                confirming.set(false);
            }
        },
        ui::dialog_card(ui::DialogWidth::Regular)
            .child(ui::dialog_title(move || {
                i18n.tr("Clear the selected media cache?", "清除选中的媒体缓存？")
            }))
            .child(
                text(move || {
                    i18n.tr(
                        "Files saved only on this device may not be downloadable again.",
                        "仅保存在此设备上的文件可能无法再次下载。",
                    )
                })
                .text_role(ui::TextRole::Body),
            )
            .child(
                parts::dialog_actions()
                    .child(
                        ui::action(
                            move || i18n.text(Key::Cancel),
                            move || confirming.set(false),
                        )
                        .disabled(feedback.busy),
                    )
                    .child(
                        ui::button(ui::ButtonTone::Danger, ui::ButtonSize::Regular)
                            .disabled(disabled)
                            .child(text(move || i18n.tr("Clear cache", "清除缓存")))
                            .on_press(clear),
                    ),
            ),
    )
}

fn bytes(value: i64) -> String {
    let value = value.max(0) as f64;
    if value >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", value / (1024.0 * 1024.0 * 1024.0))
    } else if value >= 1024.0 * 1024.0 {
        format!("{:.1} MB", value / (1024.0 * 1024.0))
    } else if value >= 1024.0 {
        format!("{:.1} KB", value / 1024.0)
    } else {
        format!("{value:.0} B")
    }
}
