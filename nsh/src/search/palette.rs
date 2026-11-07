use futures::StreamExt as _;
use nsh_base::{Command, CommandApi, CommandEntry, CommandItem, command};
use nsh_core::keymap::When;

use super::*;

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

fn content(
    account: Arc<AccountClient>,
    open: Signal<bool>,
    open_failed: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let navigation = use_context::<ChatNavigation>().expect("Home provides navigation");
    let account = stored(account);
    let query = signal(String::new());
    let page = signal(0_i64);
    let rows = signal_with(Vec::<Row>::new(), SignalEquals::Never);
    let more = signal(false);
    let loading = signal(true);
    let failed = signal(false);
    let request = signal(0_u64);
    let revision = signal(0_u64);
    let control = component_ref::<Command>();
    let highlighted = signal(None::<String>);
    let close = move || {
        dismiss_keyboard();
        open.set(false);
    };
    let mut changes = Box::pin(runtime::observe(
        account.with(|account| account.conversation_changes()),
    ));
    spawn(async move {
        while changes.next().await.is_some() {
            if open.get_untracked() {
                revision.update(|value| *value = value.wrapping_add(1));
            }
            // Event-driven throttling keeps live updates from continuously
            // canceling the input debounce during a busy conversation.
            timer(std::time::Duration::from_millis(333)).await;
        }
    });
    let move_highlight = move |forward: bool| {
        if !open.get_untracked() || loading.get_untracked() {
            return;
        }
        let mut ids = rows.with_untracked(|rows| rows.iter().map(Row::key).collect::<Vec<_>>());
        if page.get_untracked() > 0 {
            ids.push("previous".into());
        }
        if more.get_untracked() {
            ids.push("next".into());
        }
        if ids.is_empty() {
            return;
        }
        let current = highlighted
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
    on_key_with(
        "tab",
        KeyOptions {
            when: When::Always,
            repeat: true,
        },
        move |_| move_highlight(true),
    );
    #[cfg(target_os = "macos")]
    {
        on_key_with(
            "ctrl+n",
            KeyOptions {
                when: When::Always,
                repeat: true,
            },
            move |_| move_highlight(true),
        );
        on_key_with(
            "ctrl+p",
            KeyOptions {
                when: When::Always,
                repeat: true,
            },
            move |_| move_highlight(false),
        );
    }
    const SIZE: i64 = 50;
    effect(move || {
        let keyword = query.get();
        let requested_page = page.get();
        let requested_keyword = keyword.clone();
        let _revision = revision.get();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        let recent = navigation.recent_ids();
        let client = account.with(|account| account.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            timer(std::time::Duration::from_millis(100)).await;
            if request.get_untracked() != ticket {
                return;
            }
            let result = runtime::run(async move {
                if keyword.trim().is_empty() {
                    let rows = client
                        .conversation()
                        .conversation_items_by_ids(recent.clone())
                        .await?;
                    let mut rows = rows
                        .into_iter()
                        .map(|row| (row.conversation_id.clone(), row))
                        .collect::<std::collections::HashMap<_, _>>();
                    Ok((
                        recent
                            .into_iter()
                            .filter_map(|id| rows.remove(&id))
                            .map(|row| Row::Conversation(Box::new(row)))
                            .collect::<Vec<_>>(),
                        false,
                    ))
                } else {
                    let mut items = client
                        .quick_search(keyword, requested_page * SIZE, SIZE + 1)
                        .await?;
                    let more = items.len() > SIZE as usize;
                    items.truncate(SIZE as usize);
                    let result = items
                        .into_iter()
                        .map(|item| match item {
                            mixin_desktop_api::QuickSearchItem::User(user) => Row::User(*user),
                            mixin_desktop_api::QuickSearchItem::Conversation(conversation) => {
                                Row::Conversation(conversation)
                            }
                        })
                        .collect::<Vec<_>>();
                    Ok((result, more))
                }
            })
            .await;
            if request.get_untracked() != ticket
                || query.get_untracked() != requested_keyword
                || page.get_untracked() != requested_page
            {
                return;
            }
            match result {
                Ok((values, extra)) => {
                    rows.set(values);
                    more.set(extra);
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    nsh_base::dialog().animation(nsh_core::ModalAnimation::None).close_on_escape(false).close_on_backdrop_press(false).on_cancel(close).flex_1()
        .child(view().flex_1().p_6().items_center().justify_center()
        .child(presence.backdrop(view().absolute().inset_0().bg(move || theme().foreground.opacity(0.2)).on_press(close)))
        .child(presence.surface(view().w(px(480.0)).max_w(relative(1.0)).max_h(relative(0.9)).min_h_0().rounded_xl().overflow_hidden().bg(move || theme().popover)
            .child(command().component_ref(&control).flex_shrink(1.0).min_h_0().auto_focus(true).filterable(false).loading(loading)
                .query(move || Some(query.get().into())).on_query_change(move |value: &SharedString| { if query.with_untracked(|query| query.as_str() != value.as_str()) || page.get_untracked() != 0 { batch(|| { loading.set(true); query.set(value.to_string()); page.set(0); }); } })
                .placeholder(move || Some(tr("Search conversations and contacts", "搜索会话和联系人").into()))
                .accessibility_label(move || Some(tr("Quick search", "快速搜索").into()))
                .items(move || {
                    let mut items = rows.with(|rows| rows.iter().filter_map(|row| match row {
                        Row::User(user) => Some(CommandItem::new(row.key(), user.full_name.clone()).keywords([user.identity_number.clone()])),
                        Row::Conversation(conversation) => Some(CommandItem::new(row.key(), conversation.name.clone())), _ => None,
                    }).collect::<Vec<_>>());
                    if page.get() > 0 { items.push(CommandItem::new("previous", tr("Previous page", "上一页"))); }
                    if more.get() { items.push(CommandItem::new("next", tr("Next page", "下一页"))); }
                    items.into_iter().map(|item| item.disabled(loading.get())).collect::<Vec<_>>()
                })
                .search_bar_class("flex items-center px-4 py-2 border-b border-border gap-3 shrink-0")
                .search_class("flex-1 min-w-0 text-sm text-foreground bg-transparent")
                .list_class("min-h-0 max-h-96")
                .item_class("group px-4 py-3 min-h-16 focus:bg-accent hover:bg-muted active:opacity-70")
                .group_class("px-4 py-2 text-xs text-muted-foreground")
                .separator_class("h-px bg-border")
                .search_prefix(ui::icon(ui::Icon::Search))
                        .search_suffix(ui::icon_button().size_8().px_1().py_1().accessibility_label(move || Some(tr("Close", "关闭").into())).on_press(close).child(text("×").text_lg()))
                        .on_highlight_change(move |value: &Option<SharedString>| highlighted.set(value.as_ref().map(ToString::to_string)))
                .empty(view().p_6().child(text(move || tr("No results", "没有搜索结果")).text_sm().text_color(move || theme().muted_foreground)))
                .footer(view().flex_none().px_4().py_3().flex_row().items_center().gap_3().border_t_1().border_color(move || theme().border)
                    .child(text(move || tr("↑↓ Select · Enter Open · Esc Close", "↑↓ 选择 · Enter 打开 · Esc 关闭")).flex_1().min_w_0().text_xs().text_color(move || theme().muted_foreground))
                    .child(ui::command().min_h_6().py_0().on_press(close).child(text(move || tr("Close", "关闭")).text_xs())))
                .render_item(move |item: &CommandEntry| {
                    let id = stored(item.id.to_string());
                    let row = move || rows.with(|rows| rows.iter().find(|row| row.key() == id.with(|id| id.clone())).cloned());
                    let label = item.label.clone();
                    view().flex_row().gap_3().items_center().min_w_0()
                        .child(when(move || row().is_some(), move || ui::avatar_view(move || row().map(|row| match row { Row::User(user) => user.full_name.into(), Row::Conversation(row) => row.name.into(), _ => SharedString::default() }).unwrap_or_default(), move || row().map(|row| match row { Row::User(user) => user.avatar_url, Row::Conversation(row) => row.avatar_url, _ => String::new() }).unwrap_or_default())))
                        .child(text(label).text_sm().flex_1().min_w_0().number_of_lines(1))
                })
                .on_select(move |id: &SharedString| {
                    if !open.get_untracked() || loading.get_untracked() { return; }
                    match id.as_str() {
                        "next" => { loading.set(true); page.update(|page| *page += 1); return; },
                        "previous" => { loading.set(true); page.update(|page| *page = page.saturating_sub(1)); return; },
                        _ => {},
                    }
                    if let Some(row) = rows.with_untracked(|rows| rows.iter().find(|row| row.key() == id.as_str()).cloned()) {
                        match row {
                            Row::Conversation(row) => {
                                close();
                                navigation.select(*row);
                            },
                            Row::User(user) => {
                                close();
                                navigation.open_user(account.with(|account| account.clone()), user.user_id, open_failed);
                            },
                            _ => (),
                        }
                    }
                }).on_cancel(close))
            .child(when(failed, move || view().p_3().gap_2().child(text(move || tr("Couldn’t load quick search", "快速搜索加载失败")).text_sm().text_color(move || theme().destructive))
                .child(ui::action(move || tr("Retry", "重试"), move || revision.update(|revision| *revision += 1))))))))
}
