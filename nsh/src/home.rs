use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    chat, create,
    i18n::{self, Key},
    runtime, settings,
    sidebar::{self, Navigation},
    ui,
};

/// Transient presentation of the account's authoritative transfer snapshots.
#[derive(Clone, Copy)]
pub(crate) struct AttachmentProgress(Signal<Vec<mixin_desktop_api::AttachmentTransferProgress>>);

impl AttachmentProgress {
    pub(crate) fn bytes(self, transcript_id: Option<&str>, message_id: &str) -> Option<(u64, u64)> {
        self.0.with(|values| {
            values
                .iter()
                .find(|value| {
                    value.message_id == message_id
                        && value.transcript_id.as_deref() == transcript_id
                })
                .map(|value| (value.completed, value.total))
        })
    }
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    on_signed_out: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let attachment_progress = signal(Vec::new());
    provide_context(AttachmentProgress(attachment_progress));
    let progress_changes = runtime::observe(account.attachment_progress_changes());
    spawn(async move {
        futures::pin_mut!(progress_changes);
        while let Some(snapshot) = progress_changes.next().await {
            attachment_progress.set(snapshot);
        }
    });
    let settings_page = signal(settings::Page::Profile);
    let settings_detail = signal(false);
    let locate = signal_with(None::<String>, SignalEquals::Never);
    let owner = Owner::current();
    let drafts = signal(HashMap::<String, String>::new());
    let pending = signal(HashSet::<String>::new());
    let selected = signal_with(None::<ConversationListData>, SignalEquals::Never);
    let selected_id =
        computed(move || selected.with(|row| row.as_ref().map(|row| row.conversation_id.clone())));
    let security = crate::security::mount(account.clone());
    provide_context(security);
    let reedit = crate::chat::reedit::Reedit::mount();
    provide_context(reedit);
    effect(move || {
        if security.blocked() {
            reedit.clear();
        }
    });
    provide_context(crate::appearance::mount());
    provide_context(crate::drafts::Drafts::mount(account.clone()));
    let transfer = crate::transfer::mount(account.clone());
    provide_context(transfer);
    let nav = Navigation::new();
    provide_context(nav);
    let circles = sidebar::Circles::new(account.clone(), nav);
    let commands = crate::conversation_list::Commands::new(account.clone(), selected, circles);
    provide_context(commands);
    let chat_navigation = crate::navigation::ChatNavigation::new(selected, locate, nav.settings);
    provide_context(chat_navigation);
    chat_navigation.track(account.clone());
    let links = crate::links::LinkNavigation::new(account.clone());
    provide_context(links);
    let bots = crate::bot::mount(account.clone());
    provide_context(bots);
    links.on_open_app(move |link| bots.open(link));
    let playback = crate::playback::mount(account.clone());
    provide_context(playback);
    let on_account_ended = move || {
        playback.stop();
        crate::bot::end_session(bots, on_signed_out);
    };
    let notification_account = account.clone();
    effect(move || {
        if !bots.ending() {
            // Retraction and click invalidation must not wait for a browser
            // provider's asynchronous data purge to finish or recover.
            crate::notifications::attach(notification_account.clone(), selected, chat_navigation);
        }
    });
    let category = nav.category;
    let query = signal(String::new());
    let composing = signal(false);
    let unread = signal(false);
    let conversation_window = crate::conversation_list::Window::mount(
        account.clone(),
        nav,
        query,
        composing,
        unread,
        selected,
    );
    let rows = conversation_window.rows;
    let loading = conversation_window.loading;
    let failed = conversation_window.failed;
    let online = signal(false);
    let health = signal(String::from("ready"));
    let search = element_ref();
    let narrow = move || window().width.0 - nav.width().0 < 620.0;
    let search_hidden =
        computed(move || nav.settings.get() || (narrow() && selected_id.with(Option::is_some)));
    effect(move || {
        if search_hidden.get() {
            search.blur();
            composing.set(false);
        }
    });
    let palette_open = signal(false);
    on_key("mod+k", move |_| {
        if !security.blocked() {
            palette_open.set(true);
        }
    });
    let create_action = signal(None::<create::Action>);
    let shell = use_context::<crate::shell::Handle>().expect("App provides the native shell");
    crate::shell::attach(
        shell,
        account.clone(),
        selected,
        move || security.blocked() || bots.ending(),
        move || security.enabled(),
        move |command| {
            use crate::shell::Command;
            match command {
                Command::Preferences => nav.settings.set(true),
                Command::NewChat => {
                    if create_action.get_untracked().is_none() {
                        create_action.set(Some(create::Action::Chat));
                    }
                }
                Command::NewGroup => {
                    if create_action.get_untracked().is_none() {
                        create_action.set(Some(create::Action::Group));
                    }
                }
                Command::NewCircle => {
                    if create_action.get_untracked().is_none() {
                        create_action.set(Some(create::Action::Circle));
                    }
                }
                Command::QuickSearch => palette_open.set(true),
                Command::Lock => security.lock(),
                #[cfg(debug_assertions)]
                Command::Transfer => {
                    batch(|| {
                        settings_page.set(settings::Page::Transfer);
                        settings_detail.set(true);
                        nav.settings.set(true);
                    });
                }
                Command::SearchConversation => chat_navigation
                    .panel
                    .set(Some(crate::inspector::Mode::Search)),
                Command::ToggleInfo => chat_navigation.panel.update(|panel| {
                    *panel = if *panel == Some(crate::inspector::Mode::Info) {
                        None
                    } else {
                        Some(crate::inspector::Mode::Info)
                    }
                }),
                Command::Mute => {
                    if let Some(row) = selected.get_untracked() {
                        commands.toggle_mute(row);
                    }
                }
                Command::Pin => {
                    if let Some(row) = selected.get_untracked() {
                        commands.toggle_pin(row);
                    }
                }
                Command::DeleteConversation => {
                    if let Some(row) = selected.get_untracked() {
                        commands.remove(row);
                    }
                }
                Command::PreviousChat | Command::NextChat => {
                    if nav.settings.get_untracked()
                        || !query.with_untracked(|query| query.trim().is_empty())
                    {
                        return;
                    }
                    conversation_window.step(matches!(command, Command::NextChat));
                }
            }
        },
    );

    let client = account.clone();
    spawn(async move {
        let mut status = Box::pin(crate::runtime::observe(client.connection_status()));
        while let Some(connected) = status.next().await {
            online.set(connected);
            log::info!(
                "NSH connection state: {}",
                if connected {
                    "connected"
                } else {
                    "reconnecting"
                }
            );
        }
        online.set(false);
    });
    let health_account = account.clone();
    spawn(async move {
        let mut updates = Box::pin(runtime::observe(health_account.account_health()));
        while let Some(value) = updates.next().await {
            if value == "unauthorized" {
                on_account_ended();
                break;
            }
            health.set(value);
        }
    });
    let retry_connection = account.clone();
    let modal_account = account.clone();
    let search_account = account.clone();
    let palette_account = account.clone();
    effect(move || {
        if !nav.settings.get() {
            settings_detail.set(false);
        } else if !narrow() {
            settings_detail.set(true);
        }
    });
    view()
        .flex_1()
        .child(crate::bot::session(bots))
        .child(when(
            move || !security.blocked() && !bots.ending(),
            move || {
                let account = account.clone();
                let retry_connection = retry_connection.clone();
                let modal_account = modal_account.clone();
                let search_account = search_account.clone();
                let palette_account = palette_account.clone();
                view()
                    .flex_1()
                    .flex_row()
                    .child(sidebar::screen(account.clone(), nav, circles))
                    .child(
                        view()
                            .w(ui::sidebar_width)
                            .flex_shrink_0()
                            .min_h_0()
                            .bg(move || theme().background)
                            .border_r_1()
                            .border_color(move || theme().border)
                            .style_with(move |s| if narrow() { s.flex_1().min_w_0() } else { s })
                            .style_when(
                                move || {
                                    narrow()
                                        && if nav.settings.get() {
                                            settings_detail.get()
                                        } else {
                                            selected.with(Option::is_some)
                                        }
                                },
                                |s| s.hidden(),
                            )
                            .child(when(
                                move || nav.settings.get() && (!narrow() || !settings_detail.get()),
                                move || settings::navigation(
                                    settings_page,
                                    narrow,
                                    move || settings_detail.set(true),
                                    move || nav.settings.set(false),
                                ),
                            ))
                            .child({
                                let account = account.clone();
                                when(move || !nav.settings.get(), move || {
                                    let account = account.clone();
                                    let retry_connection = retry_connection.clone();
                                    let search_account = search_account.clone();
                                    view()
                                    .flex_1()
                                    .min_h_0()
                                    .child(
                                        view()
                                            .h_16()
                                            .flex_row()
                                            .items_center()
                                            .px_4()
                                            .gap_2()
                                            .child(sidebar::trigger(nav))
                                            .child(
                                                view()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap_2()
                                                    .px_3()
                                                    .rounded_full()
                                                    .bg(move || theme().muted)
                                                    .child(ui::icon(ui::Icon::Search).text_color(
                                                        move || theme().muted_foreground,
                                                    ))
                                                    .child(
                                                        ui::field()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .border(px(0.0))
                                                            .px_0()
                                                            .bg(move || theme().muted)
                                                            .element_ref(&search)
                                                            .placeholder(move || {
                                                                if unread.get() {
                                                                    i18n.tr("Search Unread", "搜索未读会话").to_owned()
                                                                } else {
                                                                    format!(
                                                                        "{} ({})",
                                                                        i18n.text(Key::Search),
                                                                        key_label("mod+k")
                                                                    )
                                                                }
                                                            })
                                                            .label(move || i18n.text(Key::Search))
                                                            .value(move || query.get())
                                                            .on_change(move |value: &str| {
                                                                batch(|| {
                                                                    query.set(value.to_owned());
                                                                })
                                                            })
                                                            .on_key_press(move |key: &str| {
                                                                if key == "Escape"
                                                                    && !composing.get_untracked()
                                                                {
                                                                    query.set(String::new());
                                                                }
                                                            })
                                                            .on_composition_change(
                                                                move |event: &Composition| {
                                                                    composing.set(event.composing)
                                                                },
                                                            ),
                                                    )
                                                    .child(when(
                                                        move || !query.with(String::is_empty),
                                                        move || {
                                                            ui::icon_button()
                                                                .size_6()
                                                                .px_0()
                                                                .py_0()
                                                                .disabled(composing)
                                                                .accessibility_label(move || {
                                                                    Some(i18n.tr("Clear", "清除").into())
                                                                })
                                                                .on_press(move || {
                                                                    query.set(String::new());
                                                                    search.focus();
                                                                })
                                                                .child(ui::icon(ui::Icon::Close).size_4())
                                                        },
                                                    )),
                                            )
                                            .child(
                                                ui::command()
                                                    .size_8()
                                                    .px_1()
                                                    .py_1()
                                                    .selected(unread)
                                                    .accessibility_label(move || {
                                                        Some(i18n.text(Key::Unread).into())
                                                    })
                                                    .selected_style(|s| {
                                                        s.text_color(move || theme().primary)
                                                    })
                                                    .on_press(move || {
                                                        batch(|| {
                                                            unread.update(|value| *value = !*value);
                                                        })
                                                    })
                                                    .child(ui::icon(ui::Icon::Filter)),
                                            )
                                            .child(create::trigger(create_action)),
                                    )
                                    .child(when(
                                        move || health.get() != "ready",
                                        move || {
                                            view().px_4().py_2().bg(move || theme().muted).child(
                                                text(move || match health.get().as_str() {
                                                    "time_inaccurate" => i18n.tr(
                                                        "Check your system clock and try again.",
                                                        "系统时间不准确，请校准后重试。",
                                                    ),
                                                    "update_required" => i18n.tr(
                                                        "Update Mixin Messenger to continue.",
                                                        "当前版本需要更新。",
                                                    ),
                                                    _ => i18n.tr(
                                                        "The account is temporarily unavailable.",
                                                        "账号暂时不可用。",
                                                    ),
                                                })
                                                .text_sm()
                                                .text_color(move || theme().destructive),
                                            )
                                        },
                                    ))
                                    .child(when(
                                        move || !online.get(),
                                        move || {
                                            let account = retry_connection.clone();
                                            view()
                                                .px_4()
                                                .py_2()
                                                .gap_2()
                                                .flex_row()
                                                .items_center()
                                                .bg(move || theme().muted)
                                                .child(
                                                    text(move || i18n.text(Key::Connecting))
                                                        .text_xs()
                                                        .flex_1(),
                                                )
                                                .child(
                                                    ui::command()
                                                        .min_h_6()
                                                        .py_0()
                                                        .on_press(move || {
                                                            account.retry_connection()
                                                        })
                                                        .child(
                                                            text(move || i18n.text(Key::Reconnect))
                                                                .text_xs(),
                                                        ),
                                                )
                                        },
                                    ))
                                    .child(when(
                                        move || !query.with(|q| q.trim().is_empty()),
                                        move || {
                                            crate::search::screen(
                                                search_account.clone(),
                                                query,
                                                composing,
                                                category,
                                                selected,
                                                locate,
                                                unread,
                                            )
                                        },
                                    ))
                                    .child(when(
                                        move || loading.get() && rows.with(Vec::is_empty),
                                        move || {
                                            view().flex_1().p_6().child(
                                                text(move || i18n.text(Key::LoadingChats))
                                                    .text_sm()
                                                    .text_color(move || theme().muted_foreground),
                                            )
                                        },
                                    ))
                                    .child(when(failed, move || {
                                        view()
                                            .flex_1()
                                            .p_5()
                                            .gap_3()
                                            .child(
                                                text(move || i18n.text(Key::ChatsFailed))
                                                    .text_sm()
                                                    .text_color(move || theme().destructive),
                                            )
                                            .child(ui::action(
                                                move || i18n.text(Key::Retry),
                                                move || conversation_window.retry(),
                                            ))
                                    }))
                                    .child(when(
                                        move || {
                                            !loading.get()
                                                && !failed.get()
                                                && rows.with(Vec::is_empty)
                                                && query.with(|q| q.trim().is_empty())
                                        },
                                        move || {
                                            let filtered = move || {
                                                !query.with(|q| q.trim().is_empty()) || unread.get()
                                            };
                                            view()
                                                .flex_1()
                                                .p_6()
                                                .gap_3()
                                                .items_center()
                                                .justify_center()
                                                .child(
                                                    ui::icon(ui::Icon::Chat).size_8().text_color(
                                                        move || theme().muted_foreground,
                                                    ),
                                                )
                                                .child(
                                                    text(move || {
                                                        if filtered() {
                                                            return i18n.tr("No unread conversations", "暂无未读会话");
                                                        }
                                                        match (category.get(), nav.circle.with(Option::is_some)) {
                                                            ("circle", _) | (_, true) => i18n.tr("No conversations in this circle", "此圈子暂无会话"),
                                                            ("contacts", _) => i18n.tr("No contact conversations", "暂无联系人会话"),
                                                            ("groups", _) => i18n.tr("No group conversations", "暂无群组会话"),
                                                            ("bots", _) => i18n.tr("No bot conversations", "暂无机器人会话"),
                                                            ("strangers", _) => i18n.tr("No stranger conversations", "暂无陌生人会话"),
                                                            _ => i18n.text(Key::NoChats),
                                                        }
                                                    })
                                                    .text_sm()
                                                    .font_weight(FontWeight::MEDIUM),
                                                )
                                                .child(
                                                    text(move || {
                                                        let in_circle = nav.circle.with(Option::is_some) || category.get() == "circle";
                                                        if filtered() {
                                                            return if in_circle {
                                                                i18n.tr("There are no unread conversations in this circle.", "此圈子中没有未读会话。")
                                                            } else if category.get() == "chats" {
                                                                i18n.tr("You have no unread conversations.", "当前没有未读会话。")
                                                            } else {
                                                                i18n.tr("There are no unread conversations in this category.", "当前分类中没有未读会话。")
                                                            };
                                                        }
                                                        if in_circle {
                                                            return i18n.tr("Conversations added to this circle will appear here.", "加入此圈子的会话会显示在这里。");
                                                        }
                                                        match category.get() {
                                                            "contacts" => i18n.tr("Conversations with your contacts will appear here.", "与联系人的会话会显示在这里。"),
                                                            "groups" => i18n.tr("Group conversations will appear here.", "群组会话会显示在这里。"),
                                                            "bots" => i18n.tr("Conversations with bots will appear here.", "与机器人的会话会显示在这里。"),
                                                            "strangers" => i18n.tr("Conversations with people outside your contacts will appear here.", "与非联系人的会话会显示在这里。"),
                                                            _ => i18n.text(Key::NoChatsBody),
                                                        }
                                                    })
                                                    .text_sm()
                                                    .text_center()
                                                    .text_color(move || theme().muted_foreground),
                                                )
                                                .child(when(filtered, move || {
                                                    ui::action(
                                                        move || i18n.text(Key::ClearFilters),
                                                        move || {
                                                            batch(|| {
                                                                query.set(String::new());
                                                                unread.set(false);
                                                            })
                                                        },
                                                    )
                                                }))
                                        },
                                    ))
                                    .child(
                                        crate::conversation_list::screen(
                                            account.clone(),
                                            rows,
                                            drafts,
                                            selected,
                                            conversation_window,
                                        )
                                        .style_when(
                                            move || {
                                                rows.with(Vec::is_empty)
                                                    || (!query.with(|q| q.trim().is_empty()))
                                            },
                                            |s| s.hidden(),
                                        ),
                                    )
                                    .child(crate::playback::bar(playback, selected))
                                })
                            }),
                    )
                    .child(
                        view()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .style_when(
                                move || {
                                    narrow()
                                        && if nav.settings.get() {
                                            !settings_detail.get()
                                        } else {
                                            selected.with(Option::is_none)
                                        }
                                },
                                |s| s.hidden(),
                            )
                            .child(dynamic(move || {
                                let settings = nav.settings.get();
                                let _conversation_id = selected_id.get();
                                let conversation = selected.get_untracked();
                                let account = account.clone();
                                untracked(|| {
                                    if settings {
                                        view().flex_1().min_w_0().min_h_0().child(when(
                                            move || !narrow() || settings_detail.get(),
                                            move || settings::screen(
                                                account.clone(),
                                                settings_page,
                                                move || {
                                                    if narrow() {
                                                        settings_detail.set(false);
                                                    } else {
                                                        nav.settings.set(false);
                                                    }
                                                },
                                                on_account_ended,
                                                owner,
                                            ),
                                        ))
                                        .into_element()
                                    } else {
                                        match conversation {
                                            Some(conversation) => ui::owned(
                                                conversation.conversation_id.clone(),
                                                move || {
                                                    chat::screen(
                                                        account.clone(),
                                                        conversation.clone(),
                                                        locate,
                                                        drafts,
                                                        pending,
                                                        owner,
                                                        move || selected.set(None),
                                                    )
                                                },
                                            )
                                            .into_element(),
                                            None => view()
                                                .flex_1()
                                                .items_center()
                                                .justify_center()
                                                .bg(move || {
                                                    theme()
                                                        .color("chat-background")
                                                        .unwrap_or(theme().background)
                                                })
                                                .child(
                                                    text(move || i18n.text(Key::PickConversation))
                                                        .text_sm()
                                                        .text_color(move || {
                                                            theme().muted_foreground
                                                        }),
                                                )
                                                .into_element(),
                                        }
                                    }
                                })
                            })),
                    )
                    .child(create::surface(modal_account, create_action, selected))
                    .child(crate::transfer::surface(transfer))
                    .child(crate::conversation_list::command_surface(commands))
                    .child(crate::search::palette(palette_account, palette_open))
                    .child(links.surface())
                    .child(crate::bot::surface(bots))
            },
        ))
        .child(crate::security::surface(security))
        .child(when(
            move || bots.ending(),
            move || crate::bot::ending_surface(bots),
        ))
}
