mod circles;
mod info;
mod members;
mod shared;
mod user;

pub(crate) use user::{profile, profile_with_opened};

use std::{future::Future, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ClientResult, ConversationDetailItem, ConversationListData,
    ConversationParticipantItem, SharedAppItem,
};
use nsh_base::{alert_dialog, theme};
use nsh_core::prelude::*;

use crate::{i18n, runtime, ui};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Info,
    Edit,
    Members,
    Expiry,
    Circles,
    Shared,
    Invite,
    Profile,
    Apps,
    EditName,
}

#[derive(Clone, Copy)]
struct State {
    account: Stored<Arc<AccountClient>>,
    id: Stored<String>,
    row: Signal<Option<ConversationListData>>,
    detail: Signal<Option<ConversationDetailItem>>,
    biography: Signal<String>,
    shared_apps_preview: Signal<Vec<SharedAppItem>>,
    members: Signal<Vec<ConversationParticipantItem>>,
    page: Signal<Page>,
    refresh: Signal<u64>,
    busy: Signal<bool>,
    loading: Signal<bool>,
    refreshing: Signal<bool>,
    failed: Signal<bool>,
}

impl State {
    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    fn id(self) -> String {
        self.id.with(|id| id.clone())
    }

    fn group(self) -> bool {
        self.row
            .with(|row| row.as_ref().is_some_and(|row| row.category == "GROUP"))
    }

    fn role(self) -> Option<String> {
        let current = self.account.with(|account| account.account_id());
        self.members.with(|members| {
            members
                .iter()
                .find(|member| member.user_id == current)
                .and_then(|member| member.role.clone())
        })
    }

    fn can_manage(self) -> bool {
        matches!(self.role().as_deref(), Some("OWNER" | "ADMIN"))
    }

    fn refresh_from_server(self) {
        if self.refreshing.get_untracked() {
            return;
        }
        self.refreshing.set(true);
        let client = self.client();
        let id = self.id();
        spawn(async move {
            match runtime::run(async move { client.conversation().conversation_detail(id).await })
                .await
            {
                Ok(_) => {
                    self.failed.set(false);
                    self.refresh.update(|value| *value += 1);
                }
                Err(_) => self.failed.set(true),
            }
            self.refreshing.set(false);
        });
    }

    fn submit(
        self,
        command: impl Future<Output = ClientResult<()>> + Send + 'static,
        done: impl FnOnce() + 'static,
    ) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        let task = runtime::commit(command);
        spawn(async move {
            match task.await {
                Ok(()) => {
                    self.refresh.update(|value| *value += 1);
                    done();
                }
                Err(_) => self.failed.set(true),
            }
            self.busy.set(false);
        });
    }
}

fn tr(en: &'static str, zh: &'static str) -> &'static str {
    crate::i18n::tr(en, zh)
}

fn row(
    title: impl Fn() -> &'static str + 'static,
    action: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    ui::command()
        .justify_start()
        .min_h_12()
        .px_4()
        .on_press(action)
        .child(text(title).text_sm())
}

fn section(title: impl Fn() -> &'static str + 'static) -> View {
    view().px_4().pt_5().pb_2().child(
        text(title)
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || theme().muted_foreground),
    )
}

async fn confirm(title: &'static str, description: String, action: &'static str) -> bool {
    matches!(
        alert_dialog().title(title).description(description)
            .cancel("cancel", tr("Cancel", "取消"))
            .destructive("apply", action).show().await,
        Ok(answer) if answer.action.as_deref() == Some("apply")
    )
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    id: String,
    target: Signal<Option<String>>,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let state = State {
        account: stored(account.clone()),
        id: stored(id.clone()),
        row: signal_with(None, SignalEquals::Never),
        detail: signal_with(None, SignalEquals::Never),
        biography: signal(String::new()),
        shared_apps_preview: signal_with(Vec::new(), SignalEquals::Never),
        members: signal_with(Vec::new(), SignalEquals::Never),
        page: signal(Page::Info),
        refresh: signal(0),
        busy: signal(false),
        loading: signal(true),
        refreshing: signal(false),
        failed: signal(false),
    };
    let version = signal(0_u64);
    effect(move || {
        let _refresh = state.refresh.get();
        let ticket = version.get_untracked().wrapping_add(1);
        version.set(ticket);
        let client = state.client();
        let id = state.id();
        state.loading.set(true);
        spawn(async move {
            let result = runtime::run(async move {
                let access = client.conversation();
                let detail = access.local_conversation_detail(id.clone()).await?;
                let (mut rows, members) = futures::try_join!(
                    access.conversation_items_by_ids(vec![id.clone()]),
                    access.conversation_participants(id)
                )?;
                let row = rows.pop();
                let (biography, shared_apps_preview) = match row.as_ref() {
                    Some(row) if row.category != "GROUP" && !row.owner_id.is_empty() => {
                        let access = client.user();
                        let (profile, apps) = futures::try_join!(
                            access.user_profile(Some(row.owner_id.clone()), None),
                            access.local_shared_apps(row.owner_id.clone())
                        )?;
                        (
                            profile.map(|profile| profile.biography).unwrap_or_default(),
                            apps.into_iter().take(4).collect::<Vec<_>>(),
                        )
                    }
                    _ => (String::new(), Vec::new()),
                };
                Ok((detail, row, members, biography, shared_apps_preview))
            })
            .await;
            if version.get_untracked() != ticket {
                return;
            }
            match result {
                Ok((detail, row, members, biography, shared_apps_preview)) => batch(|| {
                    state.detail.set(Some(detail));
                    state.row.set(row);
                    state.members.set(members);
                    state.biography.set(biography);
                    state.shared_apps_preview.set(shared_apps_preview);
                }),
                Err(_) => state.failed.set(true),
            }
            state.loading.set(false);
        });
    });
    state.refresh_from_server();
    spawn(async move {
        let mut changes = Box::pin(crate::runtime::observe(account.conversation_changes()));
        while let Some(change) = changes.next().await {
            if change.reload_all || change.conversation_ids.contains(&id) {
                state.refresh.update(|value| *value += 1);
            }
        }
    });
    view()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .h_16()
                .px_4()
                .flex_row()
                .items_center()
                .gap_3()
                .style_when(
                    move || {
                        matches!(
                            state.page.get(),
                            Page::Profile | Page::Apps | Page::EditName
                        )
                    },
                    |style| style.hidden(),
                )
                .child(
                    ui::icon_button()
                        .size_8()
                        .px_1()
                        .accessibility_label(move || Some(tr("Back", "返回").into()))
                        .on_press(move || {
                            if state.page.get_untracked() == Page::Info {
                                close();
                            } else {
                                state.page.set(Page::Info);
                            }
                        })
                        .child(ui::icon(ui::Icon::ArrowLeft)),
                )
                .child(
                    text(move || match state.page.get() {
                        Page::Info => tr("Chat info", "聊天详情"),
                        Page::Edit => tr("Edit group", "编辑群组"),
                        Page::Members => tr("Group participants", "群组成员"),
                        Page::Expiry => tr("Disappearing messages", "阅后即焚"),
                        Page::Circles => tr("Circles", "圈子"),
                        Page::Shared => tr("Shared media", "共享媒体"),
                        Page::Invite => tr("Invite to group", "邀请加入群组"),
                        Page::Profile => tr("Profile", "个人资料"),
                        Page::Apps => tr("Shared apps", "共享应用"),
                        Page::EditName => tr("Edit name", "编辑名称"),
                    })
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD),
                ),
        )
        .child(when(state.loading, move || {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .text_color(move || theme().muted_foreground)
                .child(ui::busy_indicator(
                    move || state.loading.get(),
                    move || tr("Loading…", "加载中…").into(),
                ))
                .child(text(move || tr("Loading…", "加载中…")).text_sm())
        }))
        .child(when(state.failed, move || {
            view()
                .px_4()
                .py_2()
                .gap_2()
                .child(
                    text(move || tr("Couldn’t update. Try again.", "更新失败，请重试。"))
                        .text_sm()
                        .text_color(move || theme().destructive),
                )
                .child(
                    ui::action(
                        move || tr("Retry", "重试"),
                        move || state.refresh_from_server(),
                    )
                    .disabled(state.refreshing),
                )
        }))
        .child(when(
            move || state.row.with(Option::is_some),
            move || {
                dynamic(move || {
                    let page = state.page.get();
                    untracked(|| match page {
                        Page::Info => info::screen(state).into_element(),
                        Page::Edit => info::edit(state).into_element(),
                        Page::Members => members::screen(state).into_element(),
                        Page::Expiry => info::expiry(state).into_element(),
                        Page::Circles => circles::screen(state).into_element(),
                        Page::Shared => shared::screen(state, target, close).into_element(),
                        Page::Invite => info::invite(state).into_element(),
                        Page::Profile => user::profile(
                            state.client(),
                            state.row.with_untracked(|row| {
                                row.as_ref()
                                    .map(|row| row.owner_id.clone())
                                    .unwrap_or_default()
                            }),
                            move || state.page.set(Page::Info),
                        )
                        .into_element(),
                        Page::Apps | Page::EditName => user::profile_at(
                            state.client(),
                            state.row.with_untracked(|row| {
                                row.as_ref()
                                    .map(|row| row.owner_id.clone())
                                    .unwrap_or_default()
                            }),
                            if page == Page::Apps {
                                user::Destination::Apps
                            } else {
                                user::Destination::EditName
                            },
                            move || {
                                batch(|| {
                                    state.refresh.update(|value| *value += 1);
                                    state.page.set(Page::Info);
                                });
                            },
                            || {},
                        )
                        .into_element(),
                    })
                })
            },
        ))
}
