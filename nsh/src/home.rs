use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::prelude::*;

use crate::{
    chat, conversation_list, create,
    i18n::{self, I18n, Key},
    playback::Playback,
    runtime, settings,
    shell::Command,
    sidebar::{self, Navigation},
    ui::{self, Typography as _},
};

const NARROW_CONTENT_WIDTH: f32 = 620.0;

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

/// The signed-in session's view state. Business state stays in the account.
#[derive(Clone, Copy)]
struct Home {
    owner: Owner,
    i18n: I18n,
    account: Stored<Arc<AccountClient>>,
    nav: Navigation,
    selected: Signal<Option<ConversationListData>>,
    selected_id: Memo<Option<String>>,
    locate: Signal<Option<String>>,
    drafts: Signal<HashMap<String, String>>,
    pending: Signal<HashSet<String>>,
    window: conversation_list::Window,
    query: Signal<String>,
    composing: Signal<bool>,
    unread: Signal<bool>,
    search: ElementRef,
    online: Signal<bool>,
    health: Signal<String>,
    create_action: Signal<Option<create::Action>>,
    settings_page: Signal<settings::Page>,
    settings_detail: Signal<bool>,
    /// Outlives the settings screen, which a narrow window unmounts on Back.
    profile_draft: Signal<Option<(String, String)>>,
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    on_signed_out: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let attachment_progress = signal(Vec::new());
    provide_context(AttachmentProgress(attachment_progress));
    let progress_changes = runtime::observe(account.attachment_progress_changes());
    spawn(async move {
        futures::pin_mut!(progress_changes);
        while let Some(snapshot) = progress_changes.next().await {
            attachment_progress.set(snapshot);
        }
    });
    let locate = signal_with(None::<String>, SignalEquals::Never);
    let selected = signal_with(None::<ConversationListData>, SignalEquals::Never);
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
    let toasts = ui::Toasts::mount();
    provide_context(toasts);
    let nav = Navigation::new();
    provide_context(nav);
    let circles = sidebar::Circles::new(account.clone(), nav);
    let commands = conversation_list::Commands::new(account.clone(), selected, circles);
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
    let query = signal(String::new());
    let composing = signal(false);
    let unread = signal(false);
    let home = Home {
        owner: Owner::current(),
        i18n: i18n::current(),
        account: stored(account.clone()),
        nav,
        selected,
        selected_id: chat_navigation.selected_id,
        locate,
        drafts: signal(HashMap::new()),
        pending: signal(HashSet::new()),
        window: conversation_list::Window::mount(account.clone(), nav, query, composing, unread),
        query,
        composing,
        unread,
        search: element_ref(),
        // The status stream reports the truth at once; do not flash a banner first.
        online: signal(true),
        health: signal(String::from("ready")),
        create_action: signal(None),
        settings_page: signal(settings::Page::Profile),
        settings_detail: signal(false),
        profile_draft: signal(None),
    };
    let search_hidden = computed(move || {
        nav.settings.get() || (home.narrow() && home.selected_id.with(Option::is_some))
    });
    effect(move || {
        if search_hidden.get() {
            home.search.blur();
            composing.set(false);
        }
    });
    let palette_open = signal(false);
    on_key("mod+k", move |_| {
        if !security.blocked() {
            palette_open.set(true);
        }
    });
    let shell = use_context::<crate::shell::Handle>().expect("App provides the native shell");
    crate::shell::attach(
        shell,
        account.clone(),
        selected,
        move || security.blocked() || bots.ending(),
        move || security.enabled(),
        move |command| match command {
            Command::QuickSearch => palette_open.set(true),
            Command::Lock => security.lock(),
            Command::Mute | Command::Pin | Command::DeleteConversation => {
                if let Some(row) = selected.get_untracked() {
                    match command {
                        Command::Mute => commands.toggle_mute(row),
                        Command::Pin => commands.toggle_pin(row),
                        _ => commands.remove(row),
                    }
                }
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
            command => home.dispatch(command),
        },
    );
    home.observe_account(account, on_account_ended);
    effect(move || {
        if !nav.settings.get() {
            home.settings_detail.set(false);
            home.profile_draft.set(None);
        } else if !home.narrow() {
            home.settings_detail.set(true);
        }
    });
    view()
        .flex_1()
        .child(crate::bot::session(bots))
        .child(when(
            move || !security.blocked() && !bots.ending(),
            move || {
                view()
                    .flex_1()
                    .flex_row()
                    .child(sidebar::screen(home.client(), nav, circles))
                    .child(home.list_column(playback))
                    .child(home.detail_pane(on_account_ended))
                    .child(create::surface(home.client(), home.create_action, selected))
                    .child(crate::transfer::surface(transfer))
                    .child(conversation_list::command_surface(commands))
                    .child(crate::search::palette(home.client(), palette_open))
                    .child(links.surface())
                    .child(crate::bot::surface(bots))
            },
        ))
        .child(toasts.surface())
        .child(crate::security::surface(security))
        .child(when(
            move || bots.ending(),
            move || crate::bot::ending_surface(bots),
        ))
}

impl Home {
    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    fn narrow(self) -> bool {
        window().width.0 - self.nav.width().0 < NARROW_CONTENT_WIDTH
    }

    fn searching(self) -> bool {
        !self.query.with(|query| query.trim().is_empty())
    }

    fn dispatch(self, command: Command) {
        let nav = self.nav;
        let create = move |action| {
            if self.create_action.get_untracked().is_none() {
                self.create_action.set(Some(action));
            }
        };
        match command {
            Command::Preferences => nav.settings.set(true),
            Command::NewChat => create(create::Action::Chat),
            Command::NewGroup => create(create::Action::Group),
            Command::NewCircle => create(create::Action::Circle),
            #[cfg(debug_assertions)]
            Command::Transfer => batch(|| {
                self.settings_page.set(settings::Page::Transfer);
                self.settings_detail.set(true);
                nav.settings.set(true);
            }),
            Command::PreviousChat | Command::NextChat
                if !nav.settings.get_untracked() && !untracked(|| self.searching()) =>
            {
                self.window.step(matches!(command, Command::NextChat));
            }
            _ => {}
        }
    }

    fn observe_account(self, account: Arc<AccountClient>, on_account_ended: impl Fn() + 'static) {
        let client = account.clone();
        spawn(async move {
            let mut status = Box::pin(runtime::observe(client.connection_status()));
            while let Some(connected) = status.next().await {
                self.online.set(connected);
                log::info!(
                    "NSH connection state: {}",
                    if connected {
                        "connected"
                    } else {
                        "reconnecting"
                    }
                );
            }
            self.online.set(false);
        });
        spawn(async move {
            let mut updates = Box::pin(runtime::observe(account.account_health()));
            while let Some(value) = updates.next().await {
                if value == "unauthorized" {
                    on_account_ended();
                    break;
                }
                self.health.set(value);
            }
        });
    }

    fn list_column(self, playback: Playback) -> impl IntoElement {
        let Self {
            nav,
            settings_page,
            settings_detail,
            selected,
            ..
        } = self;
        view()
            .w(ui::sidebar_width)
            .flex_shrink_0()
            .min_h_0()
            .bg(ui::surface_list)
            .style_with(move |s| {
                if self.narrow() {
                    s.flex_1().min_w_0()
                } else {
                    s
                }
            })
            .style_when(
                move || {
                    self.narrow()
                        && if nav.settings.get() {
                            settings_detail.get()
                        } else {
                            selected.with(Option::is_some)
                        }
                },
                |s| s.hidden(),
            )
            .child(when(
                move || nav.settings.get() && (!self.narrow() || !settings_detail.get()),
                move || {
                    settings::navigation(
                        self.client(),
                        settings_page,
                        move || self.narrow(),
                        move || settings_detail.set(true),
                        move || nav.settings.set(false),
                    )
                },
            ))
            // Stays mounted under Settings: returning keeps the list's scroll position.
            .child(
                self.conversations(playback)
                    .style_when(move || nav.settings.get(), |s| s.hidden()),
            )
    }

    fn conversations(self, playback: Playback) -> View {
        let Self {
            i18n,
            window,
            selected,
            ..
        } = self;
        let rows = window.rows;
        view()
            .flex_1()
            .min_h_0()
            .child(self.list_header())
            .child(self.health_banner())
            .child(self.connection_banner())
            .child(when(
                move || self.searching(),
                move || {
                    crate::search::screen(
                        self.client(),
                        self.query,
                        self.composing,
                        selected,
                        self.unread,
                    )
                },
            ))
            .child(when(
                move || window.loading.get() && rows.with(Vec::is_empty),
                move || ui::placeholder(move || i18n.text(Key::LoadingChats)),
            ))
            .child(when(window.failed, move || {
                view()
                    .p_4()
                    .gap_3()
                    .style_when(move || rows.with(Vec::is_empty), |s| s.flex_1())
                    .child(ui::error_text(move || i18n.text(Key::ChatsFailed)))
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || window.retry(),
                    ))
            }))
            .child(when(
                move || {
                    !window.loading.get()
                        && !window.failed.get()
                        && rows.with(Vec::is_empty)
                        && !self.searching()
                },
                move || self.list_empty(),
            ))
            .child(
                conversation_list::screen(self.client(), rows, self.drafts, selected, window)
                    .style_when(
                        move || rows.with(Vec::is_empty) || self.searching(),
                        |s| s.hidden(),
                    ),
            )
            .child(crate::playback::bar(playback, selected))
    }

    fn list_header(self) -> impl IntoElement {
        let Self {
            i18n, nav, unread, ..
        } = self;
        ui::title_bar()
            .pl(move || ui::title_bar_leading(nav.width()))
            .pr_4()
            .gap_2()
            .child(sidebar::trigger(nav))
            .child(self.search_field())
            .child(
                ui::round_button()
                    .size_8()
                    .selected(unread)
                    .accessibility_label(move || Some(i18n.text(Key::Unread).into()))
                    .on_press(move || unread.update(|value| *value = !*value))
                    .child(ui::icon(ui::Icon::Filter).text_color(move || {
                        if unread.get() {
                            ui::accent()
                        } else {
                            ui::fg_primary()
                        }
                    })),
            )
            .child(create::trigger(self.create_action))
    }

    fn search_field(self) -> impl IntoElement {
        let Self {
            i18n,
            query,
            composing,
            unread,
            search,
            ..
        } = self;
        ui::search_pill()
            .child(
                ui::search_input()
                    .element_ref(&search)
                    .placeholder(move || {
                        if unread.get() {
                            i18n.tr("Search Unread", "搜索未读会话").to_owned()
                        } else {
                            format!("{} ({})", i18n.text(Key::Search), key_label("mod+k"))
                        }
                    })
                    .label(move || i18n.text(Key::Search))
                    .value(move || query.get())
                    .on_change(move |value: &str| query.set(value.to_owned()))
                    .on_key_press(move |key: &str| {
                        if key == "Escape" && !composing.get_untracked() {
                            query.set(String::new());
                        }
                    })
                    .on_composition_change(move |event: &Composition| {
                        composing.set(event.composing)
                    }),
            )
            .child(when(
                move || !query.with(String::is_empty),
                move || {
                    ui::round_button()
                        .size_6()
                        .disabled(composing)
                        .accessibility_label(move || Some(i18n.tr("Clear", "清除").into()))
                        .on_press(move || {
                            query.set(String::new());
                            search.focus();
                        })
                        .child(ui::icon(ui::Icon::Close).size_4())
                },
            ))
    }

    fn health_banner(self) -> impl IntoElement {
        let Self { i18n, health, .. } = self;
        when(
            move || health.get() != "ready",
            move || {
                view().px_4().py_2().bg(ui::surface_card).child(
                    ui::error_text(move || match health.get().as_str() {
                        "time_inaccurate" => i18n.tr(
                            "Check your system clock and try again.",
                            "系统时间不准确，请校准后重试。",
                        ),
                        "update_required" => {
                            i18n.tr("Update Mixin Messenger to continue.", "当前版本需要更新。")
                        }
                        _ => i18n.tr(
                            "The account is temporarily unavailable.",
                            "账号暂时不可用。",
                        ),
                    })
                    .text_role(ui::TextRole::Caption),
                )
            },
        )
    }

    fn connection_banner(self) -> impl IntoElement {
        let Self { i18n, online, .. } = self;
        // The banner fades in, and the list below eases down to make room for it.
        let presence = crate::motion::Presence::fade(move || !online.get());
        let slot = element_ref();
        effect(move || {
            online.get();
            if !reduce_motion() {
                slot.animate_layout(crate::motion::PANEL_WIDTH);
            }
        });
        view().element_ref(&slot).child(when(
            move || presence.mounted(),
            move || {
                presence.surface(
                    view()
                        .px_4()
                        .py_2()
                        .gap_2()
                        .flex_row()
                        .items_center()
                        .bg(ui::surface_card)
                        .child(
                            text(move || i18n.text(Key::Connecting))
                                .text_role(ui::TextRole::Caption)
                                .flex_1(),
                        )
                        .child(
                            ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                                .on_press(move || self.client().retry_connection())
                                .child(
                                    text(move || i18n.text(Key::Reconnect))
                                        .text_role(ui::TextRole::Caption),
                                ),
                        ),
                )
            },
        ))
    }

    fn list_empty(self) -> impl IntoElement {
        let Self {
            i18n, nav, unread, ..
        } = self;
        let scope = move || {
            let circle = nav.circle.with(Option::is_some) || nav.category.get() == "circle";
            (nav.category.get(), circle)
        };
        ui::empty(
            move || {
                let (category, circle) = scope();
                empty_title(i18n, category, circle, unread.get())
            },
            move || {
                let (category, circle) = scope();
                empty_body(i18n, category, circle, unread.get())
            },
        )
        .child(when(unread, move || {
            ui::action(
                move || i18n.text(Key::ClearFilters),
                move || unread.set(false),
            )
        }))
    }

    fn detail_pane(self, on_account_ended: impl Fn() + Copy + 'static) -> impl IntoElement {
        let Self {
            owner,
            i18n,
            nav,
            selected,
            selected_id,
            locate,
            drafts,
            pending,
            settings_page,
            settings_detail,
            profile_draft,
            ..
        } = self;
        view()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .style_when(
                move || {
                    self.narrow()
                        && if nav.settings.get() {
                            !settings_detail.get()
                        } else {
                            selected.with(Option::is_none)
                        }
                },
                |s| s.hidden(),
            )
            // Stays mounted under Settings: returning keeps the timeline position and draft.
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .style_when(move || nav.settings.get(), |s| s.hidden())
                    .child(dynamic(move || {
                        let _conversation_id = selected_id.get();
                        let conversation = selected.get_untracked();
                        untracked(|| match conversation {
                            Some(conversation) => {
                                ui::owned(conversation.conversation_id.clone(), move || {
                                    chat::screen(
                                        self.client(),
                                        conversation.clone(),
                                        locate,
                                        drafts,
                                        pending,
                                        owner,
                                        move || selected.set(None),
                                    )
                                })
                                .into_element()
                            }
                            None => view()
                                .flex_1()
                                .min_h_0()
                                .bg(ui::surface_chat)
                                .child(ui::title_bar())
                                .child(ui::placeholder(move || i18n.text(Key::PickConversation)))
                                .into_element(),
                        })
                    })),
            )
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .style_when(move || !nav.settings.get(), |s| s.hidden())
                    .child(when(
                        move || nav.settings.get() && (!self.narrow() || settings_detail.get()),
                        move || {
                            settings::screen(
                                self.client(),
                                settings_page,
                                profile_draft,
                                move || self.narrow(),
                                move || settings_detail.set(false),
                                on_account_ended,
                                owner,
                            )
                        },
                    )),
            )
    }
}

fn empty_title(i18n: I18n, category: &str, circle: bool, unread: bool) -> &'static str {
    if unread {
        return i18n.tr("No unread conversations", "暂无未读会话");
    }
    if circle {
        return i18n.tr("No conversations in this circle", "此圈子暂无会话");
    }
    match category {
        "contacts" => i18n.tr("No contact conversations", "暂无联系人会话"),
        "groups" => i18n.tr("No group conversations", "暂无群组会话"),
        "bots" => i18n.tr("No bot conversations", "暂无机器人会话"),
        "strangers" => i18n.tr("No stranger conversations", "暂无陌生人会话"),
        _ => i18n.text(Key::NoChats),
    }
}

fn empty_body(i18n: I18n, category: &str, circle: bool, unread: bool) -> &'static str {
    match (unread, circle, category) {
        (true, true, _) => i18n.tr(
            "There are no unread conversations in this circle.",
            "此圈子中没有未读会话。",
        ),
        (true, false, "chats") => {
            i18n.tr("You have no unread conversations.", "当前没有未读会话。")
        }
        (true, false, _) => i18n.tr(
            "There are no unread conversations in this category.",
            "当前分类中没有未读会话。",
        ),
        (false, true, _) => i18n.tr(
            "Conversations added to this circle will appear here.",
            "加入此圈子的会话会显示在这里。",
        ),
        (false, false, "contacts") => i18n.tr(
            "Conversations with your contacts will appear here.",
            "与联系人的会话会显示在这里。",
        ),
        (false, false, "groups") => i18n.tr(
            "Group conversations will appear here.",
            "群组会话会显示在这里。",
        ),
        (false, false, "bots") => i18n.tr(
            "Conversations with bots will appear here.",
            "与机器人的会话会显示在这里。",
        ),
        (false, false, "strangers") => i18n.tr(
            "Conversations with people outside your contacts will appear here.",
            "与非联系人的会话会显示在这里。",
        ),
        (false, false, _) => i18n.text(Key::NoChatsBody),
    }
}
