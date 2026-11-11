use std::{collections::HashMap, time::Duration};

use futures::StreamExt as _;
use mixin_desktop_api::{ClientResult, QuickSearchItem};
use nsh_base::{Command, CommandApi, CommandEntry, CommandItem, command};
use nsh_core::keymap::When;

use super::*;

const PAGE_SIZE: i64 = 50;
const DEBOUNCE: Duration = Duration::from_millis(100);
const REFRESH_INTERVAL: Duration = Duration::from_millis(333);

pub(crate) fn surface(account: Arc<AccountClient>, open: Signal<bool>) -> impl IntoElement {
    let failed = signal(false);
    effect(move || {
        if failed.get() {
            failed.set(false);
            spawn(async move {
                let _ = nsh_base::alert_dialog()
                    .title(tr("Couldn’t open conversation", "无法打开会话"))
                    .description(tr(
                        "Check your connection and try again.",
                        "请检查网络连接后重试。",
                    ))
                    .action("close", tr("Close", "关闭"))
                    .show()
                    .await;
            });
        }
    });
    let presence = crate::motion::Presence::modal(move || open.get());
    when(
        move || presence.mounted(),
        move || content(account.clone(), open, failed, presence),
    )
}

#[derive(Clone, Copy)]
struct Palette {
    open: Signal<bool>,
    open_failed: Signal<bool>,
    account: Stored<Option<Arc<AccountClient>>>,
    navigation: ChatNavigation,
    query: Signal<String>,
    page: Signal<i64>,
    rows: Signal<Vec<Row>>,
    more: Signal<bool>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    revision: Signal<u64>,
    highlighted: Signal<Option<String>>,
}

fn content(
    account: Arc<AccountClient>,
    open: Signal<bool>,
    open_failed: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let palette = Palette {
        open,
        open_failed,
        account: stored(Some(account)),
        navigation: use_context().expect("Home provides navigation"),
        query: signal(String::new()),
        page: signal(0),
        rows: signal_with(Vec::new(), SignalEquals::Never),
        more: signal(false),
        loading: signal(true),
        failed: signal(false),
        revision: signal(0),
        highlighted: signal(None),
    };
    let control = component_ref::<Command>();
    palette.observe_changes();
    palette.load();
    palette.bind_keys(control);
    let close = move || palette.close();
    nsh_base::dialog()
        .animation(nsh_core::ModalAnimation::None)
        .close_on_escape(false)
        .close_on_backdrop_press(false)
        .on_cancel(close)
        .flex_1()
        .child(
            view()
                .flex_1()
                .p_6()
                .items_center()
                .justify_center()
                .child(presence.backdrop(ui::scrim().on_press(close)))
                .child(presence.surface(palette.card(control, true))),
        )
}

impl Palette {
    fn card(self, control: ComponentRef<Command>, focus: bool) -> View {
        ui::dialog_card(ui::DialogWidth::Regular)
            .p_0()
            .gap_0()
            .overflow_hidden()
            .child(self.command(control, focus))
            .child(self.failure())
    }

    fn close(self) {
        dismiss_keyboard();
        self.open.set(false);
    }

    fn observe_changes(self) {
        let account = self
            .account
            .with(|account| account.clone().expect("live quick search has an account"));
        spawn(async move {
            let mut changes = Box::pin(runtime::throttled(
                account.conversation_changes(),
                REFRESH_INTERVAL,
            ));
            while changes.next().await.is_some() {
                if self.open.get_untracked() {
                    self.revision.update(|value| *value = value.wrapping_add(1));
                }
            }
        });
    }

    fn load(self) {
        let seen_revision = stored(0_u64);
        effect(move || {
            let keyword = self.query.get();
            let page = self.page.get();
            let revision = self.revision.get();
            let recent = self.navigation.recent_ids();
            // Live updates keep the list and its highlight; only input shows loading.
            let background = seen_revision.replace(revision) != revision
                && !self.failed.get_untracked()
                && !self.loading.get_untracked();
            if !background {
                self.loading.set(true);
                self.failed.set(false);
            }
            let client = self
                .account
                .with(|account| account.clone().expect("live quick search has an account"));
            // Rerunning this effect cancels the previous task and its business call.
            spawn(async move {
                if !background {
                    timer(DEBOUNCE).await;
                }
                let result = runtime::run(search(client, keyword, recent, page)).await;
                match result {
                    Ok((values, more)) => batch(|| {
                        self.rows.set(values);
                        self.more.set(more);
                    }),
                    Err(_) => self.failed.set(true),
                }
                self.loading.set(false);
            });
        });
    }

    fn bind_keys(self, control: ComponentRef<Command>) {
        let move_highlight = move |forward: bool| {
            if !self.open.get_untracked() || self.loading.get_untracked() {
                return;
            }
            let ids = self.item_ids();
            if ids.is_empty() {
                return;
            }
            let current = self
                .highlighted
                .with_untracked(|highlighted| {
                    highlighted
                        .as_ref()
                        .and_then(|id| ids.iter().position(|candidate| candidate == id))
                })
                .unwrap_or(0);
            let index = if forward {
                (current + 1).min(ids.len() - 1)
            } else {
                current.saturating_sub(1)
            };
            if let Some(api) = control.api::<CommandApi>() {
                api.highlight(ids[index].clone());
            }
        };
        let always = || KeyOptions {
            when: When::Always,
            repeat: true,
            pass_through: false,
            ..Default::default()
        };
        on_key_with("tab", always(), move |_| move_highlight(true));
        #[cfg(target_os = "macos")]
        {
            on_key_with("ctrl+n", always(), move |_| move_highlight(true));
            on_key_with("ctrl+p", always(), move |_| move_highlight(false));
        }
    }

    fn item_ids(self) -> Vec<String> {
        let mut ids = self
            .rows
            .with_untracked(|rows| rows.iter().map(Row::key).collect::<Vec<_>>());
        if self.page.get_untracked() > 0 {
            ids.push("previous".into());
        }
        if self.more.get_untracked() {
            ids.push("next".into());
        }
        ids
    }

    fn row(self, id: &str) -> Option<Row> {
        self.rows
            .with(|rows| rows.iter().find(|row| row.key() == id).cloned())
    }

    fn items(self) -> Vec<CommandItem> {
        let loading = self.loading.get();
        let mut items = self.rows.with(|rows| {
            rows.iter()
                .filter_map(|row| match row {
                    Row::User(user) => Some(
                        CommandItem::new(row.key(), user.full_name.clone())
                            .keywords([user.identity_number.clone()]),
                    ),
                    Row::Conversation(conversation) => {
                        Some(CommandItem::new(row.key(), conversation.name.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        if self.page.get() > 0 {
            items.push(CommandItem::new("previous", tr("Previous page", "上一页")));
        }
        if self.more.get() {
            items.push(CommandItem::new("next", tr("Next page", "下一页")));
        }
        items
            .into_iter()
            .map(|item| item.disabled(loading))
            .collect()
    }

    fn select(self, id: &str) {
        if !self.open.get_untracked() || self.loading.get_untracked() {
            return;
        }
        match id {
            "next" => {
                self.loading.set(true);
                self.page.update(|page| *page += 1);
            }
            "previous" => {
                self.loading.set(true);
                self.page.update(|page| *page = page.saturating_sub(1));
            }
            _ => match untracked(|| self.row(id)) {
                Some(Row::Conversation(row)) => {
                    self.close();
                    self.navigation.select(*row);
                }
                Some(Row::User(user)) => {
                    self.close();
                    self.navigation.open_user(
                        self.account.with(|account| {
                            account.clone().expect("live quick search has an account")
                        }),
                        user.user_id,
                        self.open_failed,
                    );
                }
                _ => {}
            },
        }
    }

    fn command(self, control: ComponentRef<Command>, focus: bool) -> impl IntoElement {
        let close = move || self.close();
        command()
            .component_ref(&control)
            .flex_shrink(1.0)
            .min_h_0()
            .auto_focus(focus)
            .filterable(false)
            .loading(self.loading)
            .query(move || Some(self.query.get().into()))
            .on_query_change(move |value: &SharedString| {
                if self.query.with_untracked(|query| query != value.as_str())
                    || self.page.get_untracked() != 0
                {
                    batch(|| {
                        self.loading.set(true);
                        self.query.set(value.to_string());
                        self.page.set(0);
                    });
                }
            })
            .placeholder(move || {
                Some(tr("Search conversations and contacts", "搜索会话和联系人").into())
            })
            .accessibility_label(move || Some(tr("Quick search", "快速搜索").into()))
            .items(move || self.items())
            .search_bar_class("flex items-center px-4 py-2 border-b border-border gap-3 shrink-0")
            .search_class("flex-1 min-w-0 text-sm text-foreground bg-transparent")
            .list_class("min-h-0 max-h-96")
            .item_class("group px-4 py-3 min-h-16 focus:bg-state-hover hover:bg-state-hover active:bg-state-pressed")
            .group_class("px-4 py-2 text-xs text-muted-foreground")
            .separator_class("h-px bg-border")
            .search_prefix(ui::icon(ui::Icon::Search))
            .search_suffix(
                ui::round_button()
                    .size_8()
                    .accessibility_label(move || Some(tr("Close", "关闭").into()))
                    .on_press(close)
                    .child(ui::icon(ui::Icon::Close)),
            )
            .on_highlight_change(move |value: &Option<SharedString>| {
                self.highlighted
                    .set(value.as_ref().map(ToString::to_string))
            })
            .empty(ui::placeholder(move || tr("No results", "没有搜索结果")))
            .footer(
                view()
                    .flex_none()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(ui::border)
                    .child(
                        text(move || {
                            tr(
                                "↑↓ Select · Enter Open · Esc Close",
                                "↑↓ 选择 · Enter 打开 · Esc 关闭",
                            )
                        })
                        .text_role(ui::TextRole::Caption)
                        .text_color(ui::fg_secondary),
                    ),
            )
            .render_item(move |item: &CommandEntry| {
                let id = stored(item.id.to_string());
                let row = move || id.with(|id| self.row(id));
                view()
                    .flex_row()
                    .gap_3()
                    .items_center()
                    .min_w_0()
                    .child(when(
                        move || row().is_some(),
                        move || {
                            ui::avatar_view(
                                move || row().map(|row| row.avatar_id()).unwrap_or_default(),
                                move || row().map(|row| row.title()).unwrap_or_default().into(),
                                move || row().map(|row| row.avatar_url()).unwrap_or_default(), ui::AvatarSize::Medium)
                        },
                    ))
                    .child(
                        text(item.label.clone())
                            .text_role(ui::TextRole::Body)
                            .flex_1()
                            .min_w_0()
                            .number_of_lines(1),
                    )
            })
            .on_select(move |id: &SharedString| self.select(id.as_str()))
            .on_cancel(close)
    }

    fn failure(self) -> impl IntoElement {
        when(self.failed, move || {
            load_error(move || self.revision.update(|revision| *revision += 1))
        })
    }
}

async fn search(
    client: Arc<AccountClient>,
    keyword: String,
    recent: Vec<String>,
    page: i64,
) -> ClientResult<(Vec<Row>, bool)> {
    if keyword.trim().is_empty() {
        let mut rows = client
            .conversation()
            .conversation_items_by_ids(recent.clone())
            .await?
            .into_iter()
            .map(|row| (row.conversation_id.clone(), row))
            .collect::<HashMap<_, _>>();
        let rows = recent
            .into_iter()
            .filter_map(|id| rows.remove(&id))
            .map(|row| Row::Conversation(Box::new(row)))
            .collect();
        return Ok((rows, false));
    }
    let mut items = client
        .quick_search(keyword, page * PAGE_SIZE, PAGE_SIZE + 1)
        .await?;
    let more = items.len() > PAGE_SIZE as usize;
    items.truncate(PAGE_SIZE as usize);
    let rows = items
        .into_iter()
        .map(|item| match item {
            QuickSearchItem::User(user) => Row::User(*user),
            QuickSearchItem::Conversation(conversation) => Row::Conversation(conversation),
        })
        .collect();
    Ok((rows, more))
}

pub(crate) fn load_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .p_3()
        .gap_2()
        .child(ui::error_text(move || {
            tr("Couldn’t load quick search", "快速搜索加载失败")
        }))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}

#[cfg(debug_assertions)]
pub(crate) fn preview(users: Vec<UserProfileItem>, failed: Signal<bool>) -> View {
    let palette = Palette {
        open: signal(true),
        open_failed: signal(false),
        account: stored(None),
        navigation: use_context().expect("Preview provides navigation"),
        query: signal("design".into()),
        page: signal(0),
        rows: signal_with(
            users.into_iter().map(Row::User).collect(),
            SignalEquals::Never,
        ),
        more: signal(false),
        loading: signal(false),
        failed,
        revision: signal(0),
        highlighted: signal(None),
    };
    palette.card(component_ref::<Command>(), false)
}
