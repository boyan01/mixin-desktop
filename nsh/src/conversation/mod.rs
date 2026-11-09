mod circles;
mod info;
mod invite;
mod members;
mod shared;
mod user;

pub(crate) use user::{profile, profile_with_opened};

use crate::ui::parts::{self, Prompt, confirm};

use std::{future::Future, rc::Rc, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ClientResult, ConversationDetailItem, ConversationListData,
    ConversationParticipantItem, SharedAppItem, UserProfileItem,
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
    Description,
    Groups,
    Apps,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Info => tr("Chat info", "聊天详情"),
            Self::Edit => tr("Edit name", "编辑名称"),
            Self::Members => tr("Group participants", "群组成员"),
            Self::Expiry => tr("Disappearing messages", "阅后即焚"),
            Self::Circles => tr("Circles", "圈子"),
            Self::Shared => tr("Shared media", "共享媒体"),
            Self::Description => tr("Group description", "群公告"),
            Self::Groups => tr("Groups in common", "共同群组"),
            Self::Apps => tr("Shared apps", "共享应用"),
        }
    }

    /// The profile flow draws its own header and Back behavior.
    fn profile_flow(self) -> bool {
        matches!(self, Self::Apps)
    }
}

#[derive(Clone, Copy)]
struct State {
    owner: Owner,
    account: Stored<Option<Arc<AccountClient>>>,
    own_id: Stored<String>,
    id: Stored<String>,
    close: Stored<Rc<dyn Fn()>>,
    row: Signal<Option<ConversationListData>>,
    detail: Signal<Option<ConversationDetailItem>>,
    profile: Signal<Option<UserProfileItem>>,
    creator: Signal<Option<String>>,
    shared_apps_preview: Signal<Vec<SharedAppItem>>,
    members: Signal<Vec<ConversationParticipantItem>>,
    page: Signal<Page>,
    refresh: Signal<u64>,
    busy: Signal<bool>,
    loading: Signal<bool>,
    refreshing: Signal<bool>,
    failed: Signal<bool>,
    /// Name and description drafts survive Back independently.
    edit_draft: Stored<(Option<String>, Option<String>)>,
}

impl State {
    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| {
            account
                .clone()
                .expect("a detached preview runs no account commands")
        })
    }

    fn id(self) -> String {
        self.id.with(|id| id.clone())
    }

    fn own_id(self) -> String {
        self.own_id.get()
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
        self.row
            .with(|row| row.as_ref().is_some_and(|row| row.status != 3))
            && matches!(self.role().as_deref(), Some("OWNER" | "ADMIN"))
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
        own_id: stored(account.account_id()),
        account: stored(Some(account.clone())),
        id: stored(id.clone()),
        close: stored(Rc::new(close) as Rc<dyn Fn()>),
        row: signal_with(None, SignalEquals::Never),
        detail: signal_with(None, SignalEquals::Never),
        profile: signal_with(None, SignalEquals::Never),
        creator: signal(None),
        shared_apps_preview: signal_with(Vec::new(), SignalEquals::Never),
        members: signal_with(Vec::new(), SignalEquals::Never),
        page: signal(Page::Info),
        refresh: signal(0),
        busy: signal(false),
        loading: signal(true),
        refreshing: signal(false),
        failed: signal(false),
        edit_draft: stored((None, None)),
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
                let (profile, shared_apps_preview, creator) = match row.as_ref() {
                    Some(row) if row.category != "GROUP" && !row.owner_id.is_empty() => {
                        let access = client.user();
                        let (profile, apps) = futures::try_join!(
                            access.user_profile(Some(row.owner_id.clone()), None),
                            access.local_shared_apps(row.owner_id.clone())
                        )?;
                        let creator = if profile.as_ref().is_some_and(|profile| profile.is_bot) {
                            access.bot_creator_id(row.owner_id.clone()).await?
                        } else {
                            None
                        };
                        (
                            profile,
                            apps.into_iter().take(4).collect::<Vec<_>>(),
                            creator,
                        )
                    }
                    _ => (None, Vec::new(), None),
                };
                Ok((detail, row, members, profile, shared_apps_preview, creator))
            })
            .await;
            let mut removed = false;
            match result {
                Ok((detail, row, members, profile, shared_apps_preview, creator)) => {
                    removed = row.is_none() && state.row.with_untracked(Option::is_some);
                    batch(|| {
                        state.detail.set(Some(detail));
                        state.row.set(row);
                        state.members.set(members);
                        state.profile.set(profile);
                        state.creator.set(creator);
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
    frame(
        state,
        when(
            move || state.row.with(Option::is_some),
            move || {
                dynamic(move || {
                    let page = state.page.get();
                    untracked(|| page_body(state, page, target))
                })
            },
        ),
    )
}

fn page_body(state: State, page: Page, target: Signal<Option<String>>) -> AnyElement {
    match page {
        Page::Info => info::screen(state).into_element(),
        Page::Edit => info::edit(state, false).into_element(),
        Page::Members => members::screen(state).into_element(),
        Page::Expiry => info::expiry(state).into_element(),
        Page::Circles => circles::screen(state).into_element(),
        Page::Shared => shared::screen(state, target).into_element(),
        Page::Description => info::edit(state, true).into_element(),
        Page::Groups => user::groups(state.client(), state.owner_id(), state.failed).into_element(),
        Page::Apps => user::profile_at(
            state.client(),
            state.owner_id(),
            user::Destination::Apps,
            move || state.page.set(Page::Info),
        )
        .into_element(),
    }
}

#[cfg(debug_assertions)]
pub(crate) fn preview_page(
    kind: &str,
    own_id: String,
    row: ConversationListData,
    members: Vec<ConversationParticipantItem>,
    users: Vec<mixin_desktop_api::UserProfileItem>,
    failed: Signal<bool>,
    media: Vec<mixin_desktop_api::MessageListView>,
) -> impl IntoElement {
    let profile = users
        .iter()
        .find(|user| user.user_id == row.owner_id)
        .cloned();
    let bot = profile.as_ref().is_some_and(|profile| profile.is_bot);
    let state = State {
        owner: Owner::current(),
        account: stored(None),
        own_id: stored(own_id),
        id: stored(row.conversation_id.clone()),
        close: stored(Rc::new(|| {}) as Rc<dyn Fn()>),
        row: signal_with(Some(row.clone()), SignalEquals::Never),
        detail: signal_with(
            Some(ConversationDetailItem {
                conversation_id: row.conversation_id,
                name: row.name,
                announcement: if kind == "info-long" {
                    "设计评审与发布计划。\nReview updates, implementation notes and release milestones.\n".repeat(6)
                } else {
                    "Design review and release planning".into()
                },
                code_url: "https://mixin.one/codes/5d0c3b8e-91a4-4f27-b6e2-8c1f4a7d2e90".into(),
                created_at_millis: 1_793_610_000_000,
                mute_until_millis: 0,
                expire_in: 0,
            }),
            SignalEquals::Never,
        ),
        profile: signal_with(profile, SignalEquals::Never),
        creator: signal(bot.then(|| "c2e84b19-0a6f-47d3-b5c1-7e39f2d8a540".into())),
        shared_apps_preview: signal_with(
            if bot {
                vec![SharedAppItem {
                    app_id: "c2e84b19-0a6f-47d3-b5c1-7e39f2d8a540".into(),
                    name: "Mixin Tools".into(),
                    icon_url: concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo.png").into(),
                    description: String::new(),
                    home_uri: String::new(),
                }]
            } else {
                Vec::new()
            },
            SignalEquals::Never,
        ),
        members: signal_with(members, SignalEquals::Never),
        page: signal(Page::Info),
        refresh: signal(0),
        busy: signal(false),
        loading: signal(false),
        refreshing: signal(false),
        failed,
        edit_draft: stored((None, None)),
    };
    if kind == "shared" {
        state.page.set(Page::Shared);
        return frame(
            State {
                failed: signal(false),
                ..state
            },
            shared::preview(state, media, failed),
        )
        .into_element();
    }
    if kind == "circles" {
        state.page.set(Page::Circles);
        return frame(state, circles::detached(state)).into_element();
    }
    if kind == "add-members" {
        return members::preview_add(state, users).into_element();
    }
    if kind == "invite" {
        return invite::card(state, || {}, failed).into_element();
    }
    let page = match kind {
        "edit" => Page::Edit,
        "description" => Page::Description,
        "expiry" => Page::Expiry,
        "members" => Page::Members,
        "info" | "info-long" => Page::Info,
        _ => Page::Info,
    };
    state.page.set(page);
    frame(
        state,
        dynamic(move || {
            let page = state.page.get();
            untracked(|| match page {
                Page::Groups => user::groups_view(
                    signal_with(Vec::new(), SignalEquals::Never),
                    signal(false),
                    state.failed,
                    signal(0),
                    |_| {},
                )
                .into_element(),
                Page::Apps => user::preview_apps(
                    state.shared_apps_preview.get_untracked(),
                    state.failed,
                    move || state.page.set(Page::Info),
                )
                .into_element(),
                _ => page_body(state, page, signal(None)),
            })
        }),
    )
    .into_element()
}

#[cfg(debug_assertions)]
pub(crate) use user::preview_name;

fn frame(state: State, content: impl IntoElement) -> View {
    view()
        .flex_1()
        .min_h_0()
        .bg(ui::surface_popover)
        .child(
            parts::page_header(
                move || {
                    if state.page.get() == Page::Info {
                        ""
                    } else {
                        state.page.get().title()
                    }
                },
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
                move || state.page.get() == Page::Info,
                |style| style.border_color(ui::surface_popover),
            )
            .style_when(
                move || state.page.get().profile_flow() || state.page.get() == Page::Members,
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
            user::update_error(move || state.refresh_from_server())
                .px(ui::space_lg())
                .py(ui::space_sm())
        }))
        .child(content)
}

#[cfg(debug_assertions)]
pub(crate) use user::{preview_apps, preview_groups, preview_profile};
