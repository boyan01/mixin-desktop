mod circles;
mod info;
mod members;
pub(crate) mod parts;
mod shared;
mod user;

pub(crate) use user::{profile, profile_with_opened};

use parts::{Prompt, confirm};

use std::{future::Future, rc::Rc, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ClientResult, ConversationDetailItem, ConversationListData,
    ConversationParticipantItem, SharedAppItem,
};
use nsh_base::alert_dialog;
use nsh_core::prelude::*;

use crate::{
    i18n::{self, tr},
    runtime,
    ui::{self, Typography as _},
};

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

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Info => tr("Chat info", "聊天详情"),
            Self::Edit => tr("Edit group", "编辑群组"),
            Self::Members => tr("Group participants", "群组成员"),
            Self::Expiry => tr("Disappearing messages", "阅后即焚"),
            Self::Circles => tr("Circles", "圈子"),
            Self::Shared => tr("Shared media", "共享媒体"),
            Self::Invite => tr("Invite to group", "邀请加入群组"),
            Self::Profile => tr("Profile", "个人资料"),
            Self::Apps => tr("Shared apps", "共享应用"),
            Self::EditName => tr("Edit name", "编辑名称"),
        }
    }

    /// The profile flow draws its own header and Back behavior.
    fn profile_flow(self) -> bool {
        matches!(self, Self::Profile | Self::Apps | Self::EditName)
    }
}

#[derive(Clone, Copy)]
struct State {
    owner: Owner,
    account: Stored<Arc<AccountClient>>,
    id: Stored<String>,
    close: Stored<Rc<dyn Fn()>>,
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
    /// Survives Back from Edit group, so an unsaved edit is not lost.
    edit_draft: Stored<Option<(String, String)>>,
}

impl State {
    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    fn id(self) -> String {
        self.id.with(|id| id.clone())
    }

    fn own_id(self) -> String {
        self.account.with(|account| account.account_id())
    }

    fn owner_id(self) -> String {
        self.row.with_untracked(|row| {
            row.as_ref()
                .map(|row| row.owner_id.clone())
                .unwrap_or_default()
        })
    }

    fn close(self) {
        self.close.with(|close| close.clone())();
    }

    fn group(self) -> bool {
        self.row
            .with(|row| row.as_ref().is_some_and(|row| row.category == "GROUP"))
    }

    fn role(self) -> Option<String> {
        let current = self.own_id();
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

    fn reload(self) {
        self.refresh.update(|value| *value += 1);
    }

    fn refresh_from_server(self) {
        if self.refreshing.get_untracked() {
            return;
        }
        self.refreshing.set(true);
        let client = self.client();
        let id = self.id();
        self.owner.run(|| {
            spawn(async move {
                let result =
                    runtime::run(
                        async move { client.conversation().conversation_detail(id).await },
                    )
                    .await;
                self.refreshing.set(false);
                self.failed.set(result.is_err());
                if result.is_ok() {
                    self.reload();
                }
            })
        });
    }

    /// Runs under the screen owner: a press handler's own owner, such as a menu
    /// popup or list cell, can end first and would leave `busy` set for good.
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
        self.owner.run(|| {
            spawn(async move {
                let result = task.await;
                self.busy.set(false);
                match result {
                    Ok(()) => {
                        self.reload();
                        done();
                    }
                    Err(_) => self.failed.set(true),
                }
            })
        });
    }

    fn confirm_then<F>(self, prompt: Prompt, command: impl FnOnce() -> F + 'static)
    where
        F: Future<Output = ClientResult<()>> + Send + 'static,
    {
        self.confirm_then_finish(prompt, command, || {});
    }

    fn confirm_then_finish<F>(
        self,
        prompt: Prompt,
        command: impl FnOnce() -> F + 'static,
        done: impl FnOnce() + 'static,
    ) where
        F: Future<Output = ClientResult<()>> + Send + 'static,
    {
        self.owner.run(|| {
            spawn(async move {
                if confirm(prompt).await {
                    self.submit(command(), done);
                }
            })
        });
    }
}

/// A card row that acts here, such as a command or a confirmation.
fn row(
    title: impl Fn() -> &'static str + 'static,
    action: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    ui::settings_row(None, title).on_press(action)
}

/// A card row that opens another page.
fn page_row(
    title: impl Fn() -> &'static str + 'static,
    action: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    row(title, action).child(ui::chevron())
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    id: String,
    target: Signal<Option<String>>,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let state = State {
        owner: Owner::current(),
        account: stored(account.clone()),
        id: stored(id.clone()),
        close: stored(Rc::new(close) as Rc<dyn Fn()>),
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
        edit_draft: stored(None),
    };
    effect(move || {
        let _refresh = state.refresh.get();
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
            let mut removed = false;
            match result {
                Ok((detail, row, members, biography, shared_apps_preview)) => {
                    removed = row.is_none() && state.row.with_untracked(Option::is_some);
                    batch(|| {
                        state.detail.set(Some(detail));
                        state.row.set(row);
                        state.members.set(members);
                        state.biography.set(biography);
                        state.shared_apps_preview.set(shared_apps_preview);
                    });
                }
                Err(_) => state.failed.set(true),
            }
            state.loading.set(false);
            // Closing disposes this screen, so nothing may follow it.
            if removed {
                state.close();
            }
        });
    });
    state.refresh_from_server();
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
        while let Some(change) = changes.next().await {
            if change.reload_all || change.conversation_ids.contains(&id) {
                state.reload();
            }
        }
    });
    view()
        .flex_1()
        .min_h_0()
        .child(
            parts::page_header(
                move || state.page.get().title(),
                Some(
                    parts::back_button(move || state.page.set(Page::Info)).style_when(
                        move || state.page.get() == Page::Info,
                        |style| style.hidden(),
                    ),
                ),
                Some(parts::close_button(move || state.close()).style_when(
                    move || state.page.get() != Page::Info,
                    |style| style.hidden(),
                )),
            )
            .style_when(
                move || state.page.get().profile_flow(),
                |style| style.hidden(),
            ),
        )
        .child(when(
            move || state.loading.get() && state.row.with(Option::is_none),
            || {
                parts::status(move || tr("Loading…", "加载中…"))
                    .px(ui::space_lg())
                    .py(ui::space_sm())
            },
        ))
        .child(when(state.failed, move || {
            parts::failure(
                move || tr("Couldn’t update. Try again.", "更新失败，请重试。"),
                move || state.refresh_from_server(),
            )
            .px(ui::space_lg())
            .py(ui::space_sm())
        }))
        .child(when(
            move || state.row.with(Option::is_some),
            move || {
                dynamic(move || {
                    let page = state.page.get();
                    untracked(|| page_body(state, page, target))
                })
            },
        ))
}

fn page_body(state: State, page: Page, target: Signal<Option<String>>) -> AnyElement {
    match page {
        Page::Info => info::screen(state).into_element(),
        Page::Edit => info::edit(state).into_element(),
        Page::Members => members::screen(state).into_element(),
        Page::Expiry => info::expiry(state).into_element(),
        Page::Circles => circles::screen(state).into_element(),
        Page::Shared => shared::screen(state, target).into_element(),
        Page::Invite => info::invite(state).into_element(),
        Page::Profile => user::profile_at(
            state.client(),
            state.owner_id(),
            user::Destination::Profile,
            move || state.page.set(Page::Info),
        )
        .into_element(),
        Page::Apps | Page::EditName => user::profile_at(
            state.client(),
            state.owner_id(),
            if page == Page::Apps {
                user::Destination::Apps
            } else {
                user::Destination::EditName
            },
            move || {
                batch(|| {
                    state.reload();
                    state.page.set(Page::Info);
                });
            },
        )
        .into_element(),
    }
}
