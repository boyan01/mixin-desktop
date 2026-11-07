use mixin_desktop_api::{GroupConversationItem, UserProfileItem};
use nsh_core::element::desktop::MenuItem;

use super::*;
use crate::navigation::ChatNavigation;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Destination {
    Profile,
    EditName,
    AddContact,
    Groups,
    Apps,
}

impl Destination {
    fn title(self) -> &'static str {
        match self {
            Self::Profile => tr("Profile", "个人资料"),
            Self::EditName => tr("Edit name", "编辑名称"),
            Self::AddContact => tr("Add contact", "添加联系人"),
            Self::Groups => tr("Groups in common", "共同群组"),
            Self::Apps => tr("Shared apps", "共享应用"),
        }
    }

    fn name_form(self) -> bool {
        matches!(self, Self::EditName | Self::AddContact)
    }
}

/// Flutter's marker for an account with no public identity.
const ANONYMOUS_ID: &str = "0";

#[derive(Clone, Copy)]
enum Action {
    Add,
    Rename,
    Remove,
    Block,
    Unblock,
    Report,
}

#[derive(Clone, Copy)]
struct Page {
    owner: Owner,
    account: Stored<Arc<AccountClient>>,
    own_id: Stored<String>,
    id: Signal<String>,
    profile: Signal<Option<UserProfileItem>>,
    creator: Signal<Option<String>>,
    destination: Signal<Destination>,
    history: Signal<Vec<(String, Destination)>>,
    revision: Signal<u64>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    failed: Signal<bool>,
}

impl Page {
    fn new(account: Arc<AccountClient>, user_id: String, initial: Destination) -> Self {
        Self {
            owner: Owner::current(),
            own_id: stored(account.account_id()),
            account: stored(account),
            id: signal(user_id),
            profile: signal_with(None, SignalEquals::Never),
            creator: signal(None),
            destination: signal(initial),
            history: signal(Vec::new()),
            revision: signal(0),
            loading: signal(true),
            busy: signal(false),
            failed: signal(false),
        }
    }

    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    fn is(self, test: impl Fn(&UserProfileItem) -> bool) -> bool {
        self.profile
            .with(|profile| profile.as_ref().is_some_and(&test))
    }

    fn missing_or(self, test: impl Fn(&UserProfileItem) -> bool) -> bool {
        self.profile
            .with(|profile| profile.as_ref().is_none_or(&test))
    }

    fn text(self, read: impl Fn(&UserProfileItem) -> String) -> String {
        self.profile
            .with(|profile| profile.as_ref().map(read).unwrap_or_default())
    }

    fn other_user(self) -> bool {
        let own_id = self.own_id.with(|id| id.clone());
        self.is(|profile| profile.user_id != own_id)
    }

    fn open_profile(self, user: String) {
        batch(|| {
            self.history.update(|history| {
                history.push((self.id.get_untracked(), self.destination.get_untracked()));
            });
            self.id.set(user);
            self.destination.set(Destination::Profile);
        });
    }

    fn confirm_then(self, prompt: Prompt, apply: impl FnOnce() + 'static) {
        self.owner.run(|| {
            spawn(async move {
                if confirm(prompt).await {
                    apply();
                }
            })
        });
    }
}

/// The profile dialog's content, for a `Compact` card with no insets. Opening
/// the chat ends the dialog.
pub(crate) fn profile(
    account: Arc<AccountClient>,
    user_id: String,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    profile_with_opened(account, user_id, close, close)
}

/// A `profile` whose host does more than `close` once the chat opens.
pub(crate) fn profile_with_opened(
    account: Arc<AccountClient>,
    user_id: String,
    close: impl Fn() + Copy + 'static,
    opened: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let page = Page::new(account, user_id, Destination::Profile);
    let opened = move || {
        if page.owner.is_live() {
            opened();
        }
    };
    effect(move || load(page, Destination::Profile));
    view()
        .flex_shrink(1.0)
        .min_h_0()
        .child(ui::dialog_close(close).disabled(page.busy))
        .child(when(
            move || page.loading.get() && page.profile.with(Option::is_none),
            || {
                parts::status(move || tr("Loading…", "加载中…"))
                    .justify_center()
                    .pt(ui::space_sm())
                    .pb(ui::space_xl())
            },
        ))
        .child(when(page.failed, move || {
            parts::failure(
                move || tr("Couldn’t update. Try again.", "更新失败，请重试。"),
                move || page.revision.update(|revision| *revision += 1),
            )
            .px(ui::space_lg())
            .pb(ui::space_sm())
        }))
        .child(when(
            move || page.profile.with(Option::is_some),
            move || dialog_body(page, opened),
        ))
}

fn dialog_body(page: Page, opened: impl Fn() + Copy + 'static) -> impl IntoElement {
    let links = use_context::<crate::links::LinkNavigation>().expect("Home provides link routing");
    let navigation =
        use_context::<ChatNavigation>().expect("The signed-in session provides navigation");
    let unavailable = move || page.busy.get();
    let known = move || page.is(|profile| profile.identity_number != ANONYMOUS_ID);
    let open = move |panel: Option<crate::inspector::Mode>| {
        navigation.open_user_then(
            page.client(),
            page.id.get_untracked(),
            page.failed,
            move || {
                navigation.panel.set(panel);
                opened();
            },
        )
    };
    let subtitle = move || {
        let id = page.text(|profile| profile.identity_number.clone());
        if id == ANONYMOUS_ID {
            return String::new();
        }
        i18n::current().format("Mixin ID: {arg0}", "Mixin ID：{arg0}", &[("arg0", &id)])
    };
    let card = ui::profile_card(
        ui::AvatarSize::Hero,
        move || page.id.get(),
        move || page.text(|profile| profile.full_name.clone()),
        move || page.text(|profile| profile.avatar_url.clone()),
        subtitle,
        identity_badge(page),
    )
    .pb_14()
    .child(when(
        move || {
            page.is(|profile| {
                profile.identity_number != ANONYMOUS_ID
                    && profile.relationship == "STRANGER"
                    && !profile.is_deactivated
            })
        },
        move || {
            ui::pill(
                move || {
                    if page.is(|profile| profile.is_bot) {
                        tr("+ Add Bot", "+ 添加机器人")
                    } else {
                        tr("+ Add Contact", "+ 添加联系人")
                    }
                },
                move || {
                    let name = untracked(|| page.text(|profile| profile.full_name.clone()));
                    run(page, Action::Add, name, move || refresh(page));
                },
            )
            .mt(ui::space_sm())
            .disabled(unavailable)
        },
    ))
    .child(when(
        move || page.is(|profile| !profile.is_deactivated && !profile.biography.is_empty()),
        move || ui::profile_bio(move || page.text(|profile| profile.biography.clone())),
    ))
    .child(when(
        move || page.is(|profile| profile.is_deactivated),
        move || {
            ui::profile_notice(move || {
                tr("This user has deleted their account.", "此用户已删除账号。")
            })
        },
    ))
    .child(when(known, move || {
        ui::profile_actions()
            .style_when(move || !page.other_user(), |style| style.justify_center())
            .child(
                ui::profile_action(
                    ui::Icon::Share,
                    move || tr("Share contact…", "分享联系人…"),
                    move || links.open(contact_uri(&page.id.get_untracked()), None),
                )
                .disabled(unavailable),
            )
            .child(when(
                move || page.other_user(),
                move || {
                    ui::profile_action(
                        ui::Icon::Conversation,
                        move || tr("Message", "发消息"),
                        move || open(None),
                    )
                    .disabled(unavailable)
                },
            ))
            .child(when(
                move || page.other_user(),
                move || {
                    ui::profile_action(
                        ui::Icon::Profile,
                        move || tr("Profile", "个人资料"),
                        move || open(Some(crate::inspector::Mode::Info)),
                    )
                    .disabled(unavailable)
                },
            ))
    }));
    ui::native_context_menu(scroll().flex_shrink(1.0).min_h_0().child(card), move || {
        dialog_menu(page)
    })
    .flex_shrink(1.0)
    .min_h_0()
}

/// What Flutter keeps on the chat info page: the dialog offers it on a secondary
/// click, so it stays one step from a person with no chat yet.
fn dialog_menu(page: Page) -> Vec<MenuItem> {
    let Some(profile) = page.profile.get_untracked() else {
        return Vec::new();
    };
    let disabled = page.busy.get_untracked();
    let ask = move |title: &'static str, prompt: fn() -> Prompt, action: Action| {
        MenuItem::new(title).disabled(disabled).on_select(move || {
            page.confirm_then(prompt(), move || {
                run(page, action, String::new(), move || refresh(page));
            });
        })
    };
    let link = (!profile.code_url.is_empty()).then(|| {
        let url = profile.code_url.clone();
        MenuItem::new(tr("Copy link", "复制链接")).on_select(move || set_clipboard(url.clone()))
    });
    let mut relationship = Vec::new();
    if profile.identity_number != ANONYMOUS_ID && untracked(|| page.other_user()) {
        if profile.relationship == "BLOCKING" {
            relationship.push(ask(
                tr("Unblock…", "取消屏蔽…"),
                unblock_prompt,
                Action::Unblock,
            ));
        } else {
            if profile.relationship == "FRIEND" {
                relationship.push(ask(
                    tr("Remove contact…", "移除联系人…"),
                    remove_prompt,
                    Action::Remove,
                ));
            }
            relationship.push(ask(tr("Block…", "屏蔽…"), block_prompt, Action::Block));
        }
        relationship.push(ask(
            tr("Report and block…", "举报并屏蔽…"),
            report_prompt,
            Action::Report,
        ));
    }
    ui::menu_groups([link.into_iter().collect(), relationship])
}

/// Loads the user again after a command changed them.
fn refresh(page: Page) {
    page.revision.update(|revision| *revision += 1);
}

/// Runs `action` for the user on screen, then `done` if it applied and they still are.
fn run(page: Page, action: Action, name: String, done: impl FnOnce() + 'static) {
    if page.busy.get_untracked() {
        return;
    }
    page.busy.set(true);
    page.failed.set(false);
    let client = page.client();
    let user_id = page.id.get_untracked();
    let target = user_id.clone();
    let command = runtime::commit(async move {
        let access = client.user();
        match action {
            Action::Add => access.add_contact(user_id, name).await,
            Action::Rename => access.rename_contact(user_id, name).await,
            Action::Remove => access.remove_contact(user_id).await,
            Action::Block => access.block_user(user_id).await,
            Action::Unblock => access.unblock_user(user_id).await,
            Action::Report => access.report_user(user_id).await,
        }
    });
    // The name form owns this press, but ends before the write when its page changes.
    page.owner.run(|| {
        spawn(async move {
            let result = command.await;
            page.busy.set(false);
            if page.id.get_untracked() != target {
                return;
            }
            if result.is_err() {
                page.failed.set(true);
                return;
            }
            done();
        })
    });
}

/// The whole profile, in the window's right-hand pane.
pub(super) fn profile_at(
    account: Arc<AccountClient>,
    user_id: String,
    initial: Destination,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let page = Page::new(account, user_id, initial);
    let back_button = element_ref();
    effect(move || load(page, initial));
    let apply = move |action: Action, name: String| {
        run(page, action, name, move || {
            let current = page.destination.get_untracked();
            if current == Destination::EditName
                && initial == Destination::EditName
                && page.history.with_untracked(Vec::is_empty)
            {
                close();
                return;
            }
            refresh(page);
            page.destination.set(Destination::Profile);
            if current.name_form() {
                back_button.focus();
            }
        });
    };
    let back = move || {
        let destination = page.destination.get_untracked();
        if destination == initial && page.history.with_untracked(Vec::is_empty) {
            close();
        } else if destination != Destination::Profile {
            page.destination.set(Destination::Profile);
            if destination.name_form() {
                back_button.focus();
            }
        } else if let Some((previous, destination)) = page
            .history
            .with_untracked(|history| history.last().cloned())
        {
            batch(|| {
                page.history.update(|history| {
                    history.pop();
                });
                page.id.set(previous);
                page.destination.set(destination);
            });
        } else {
            close();
        }
    };
    let title = move || page.destination.get().title();
    let back = parts::back_button(back)
        .element_ref(&back_button)
        .disabled(page.busy);
    view()
        .flex_1()
        .min_h_0()
        .child(parts::page_header(title, back))
        .child(when(
            move || {
                page.loading.get()
                    && page.profile.with(Option::is_none)
                    && !matches!(
                        page.destination.get(),
                        Destination::Groups | Destination::Apps
                    )
            },
            || {
                parts::status(move || tr("Loading…", "加载中…"))
                    .px(ui::space_lg())
                    .py(ui::space_sm())
            },
        ))
        .child(when(page.failed, move || {
            parts::failure(
                move || tr("Couldn’t update. Try again.", "更新失败，请重试。"),
                move || page.revision.update(|revision| *revision += 1),
            )
            .px(ui::space_lg())
            .py(ui::space_sm())
        }))
        .child(dynamic(move || {
            let destination = page.destination.get();
            let selected = page.id.get();
            untracked(|| match destination {
                Destination::Groups => groups(page.client(), selected, page.failed).into_element(),
                Destination::Apps => {
                    apps(page.client(), selected, move |id| page.open_profile(id)).into_element()
                }
                Destination::EditName | Destination::AddContact => when(
                    move || page.profile.with(Option::is_some),
                    move || name_form(page, destination, apply),
                )
                .into_element(),
                Destination::Profile => details(page, apply).into_element(),
            })
        }))
}

/// Runs inside the page effect, so a rerun cancels this load and its subscription.
fn load(page: Page, initial: Destination) {
    if initial == Destination::Apps && page.destination.get() == Destination::Apps {
        page.loading.set(false);
        page.failed.set(false);
        return;
    }
    let user_id = page.id.get();
    let _revision = page.revision.get();
    page.loading.set(true);
    page.failed.set(false);
    // Keep the current user on screen through a refresh; clearing it collapses the page.
    if page
        .profile
        .with_untracked(|profile| profile.as_ref().is_none_or(|p| p.user_id != user_id))
    {
        batch(|| {
            page.profile.set(None);
            page.creator.set(None);
        });
    }
    let client = page.client();
    spawn(async move {
        let updates = client.clone();
        let observed_id = user_id.clone();
        let mut changes = Box::pin(runtime::observe(updates.conversation_changes()));
        let local_client = client.clone();
        let local_id = user_id.clone();
        if let Ok(Some(value)) =
            runtime::run(
                async move { local_client.user().user_profile(Some(local_id), None).await },
            )
            .await
        {
            page.profile.set(Some(value));
        }
        let result = runtime::run(async move {
            let value = client.user().refresh_user_profile(user_id.clone()).await?;
            let creator = if value.as_ref().is_some_and(|user| user.is_bot) {
                client.user().bot_creator_id(user_id).await?
            } else {
                None
            };
            Ok((value, creator))
        })
        .await;
        match result {
            Ok((Some(value), owner)) => {
                page.profile.set(Some(value));
                page.creator.set(owner);
            }
            _ => page.failed.set(true),
        }
        page.loading.set(false);
        while let Some(change) = changes.next().await {
            if !change.reload_all {
                continue;
            }
            let client = updates.clone();
            let current = observed_id.clone();
            let result =
                runtime::run(async move { client.user().user_profile(Some(current), None).await })
                    .await;
            if let Ok(Some(value)) = result {
                page.profile.set(Some(value));
            }
        }
    });
}

fn name_form(
    page: Page,
    destination: Destination,
    apply: impl Fn(Action, String) + Copy + 'static,
) -> impl IntoElement {
    let name = signal(page.text(|profile| profile.full_name.clone()));
    let editing = destination == Destination::EditName;
    parts::page_body()
        .child(
            ui::field()
                .label(move || tr("Name", "名称"))
                .max_length(if editing { 40 } else { 64 })
                .readonly(page.busy)
                .auto_focus(true)
                .value(move || name.get())
                .on_change(move |text: &str| name.set(text.into())),
        )
        .child(
            ui::primary()
                .disabled(move || {
                    page.busy.get()
                        || name.with(|name| name.trim().is_empty())
                        || page.missing_or(|profile| profile.is_deactivated)
                })
                .on_press(move || {
                    apply(
                        if editing { Action::Rename } else { Action::Add },
                        name.get_untracked().trim().into(),
                    )
                })
                .child(text(move || tr("Save", "保存"))),
        )
}

fn details(page: Page, apply: impl Fn(Action, String) + Copy + 'static) -> impl IntoElement {
    scroll().flex_1().min_h_0().child(
        parts::page_body()
            .child(summary(page))
            .child(primary_rows(page))
            .child(when(
                move || page.other_user(),
                move || relationship_rows(page, apply),
            )),
    )
}

fn summary(page: Page) -> impl IntoElement {
    view()
        .gap(ui::space_sm())
        .items_center()
        .child(
            ui::avatar_view(
                move || page.text(|profile| profile.user_id.clone()),
                move || page.text(|profile| profile.full_name.clone()).into(),
                move || page.text(|profile| profile.avatar_url.clone()),
                ui::AvatarSize::Medium,
            )
            .size_16(),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(ui::space_xs())
                .min_w_0()
                .child(
                    text(move || page.text(|profile| profile.full_name.clone()))
                        .text_role(ui::TextRole::Headline)
                        .number_of_lines(2),
                )
                .child(identity_badge(page)),
        )
        .child(
            text(move || page.text(|profile| profile.identity_number.clone()))
                .text_role(ui::TextRole::Body)
                .select_text(),
        )
        .child(when(
            move || page.is(|profile| !profile.is_deactivated && !profile.biography.is_empty()),
            move || {
                text(move || page.text(|profile| profile.biography.clone()))
                    .text_role(ui::TextRole::Body)
                    .text_center()
            },
        ))
        .child(when(
            move || page.is(|profile| profile.is_deactivated),
            || {
                ui::error_text(move || {
                    tr("This user has deleted their account.", "此用户已删除账号。")
                })
                .text_center()
            },
        ))
        .child(when(
            move || page.is(|profile| profile.is_verified),
            || {
                text(move || tr("Verified", "已认证"))
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::fg_secondary)
            },
        ))
}

fn identity_badge(page: Page) -> impl IntoElement {
    dynamic(move || {
        let value = page.profile.get();
        untracked(|| match value {
            Some(value) => crate::appearance::identity_badge(
                false,
                value.is_verified,
                value.is_bot && value.identity_number != ANONYMOUS_ID,
                value.membership.as_deref(),
            ),
            None => view().hidden().into_element(),
        })
    })
}

fn primary_rows(page: Page) -> impl IntoElement {
    let links = use_context::<crate::links::LinkNavigation>().expect("Home provides link routing");
    let navigation =
        use_context::<ChatNavigation>().expect("The signed-in session provides navigation");
    let unavailable = move || page.busy.get() || page.profile.with(Option::is_none);
    ui::card()
        .child(when(
            move || page.id.get() != page.own_id.with(|id| id.clone()),
            move || {
                row(
                    move || tr("Message", "发消息"),
                    move || {
                        navigation.open_user(page.client(), page.id.get_untracked(), page.failed)
                    },
                )
                .disabled(unavailable)
            },
        ))
        .child(
            row(
                move || tr("Share contact…", "分享联系人…"),
                move || links.open(contact_uri(&page.id.get_untracked()), None),
            )
            .disabled(unavailable),
        )
        .child(when(
            move || page.is(|profile| profile.is_bot),
            move || {
                view()
                    .child(row(
                        move || tr("Open app", "打开应用"),
                        move || {
                            links.open(
                                format!("mixin://apps/{}?action=open", page.id.get_untracked()),
                                None,
                            )
                        },
                    ))
                    .child(when(
                        move || page.creator.with(Option::is_some),
                        move || {
                            page_row(
                                move || tr("Created by", "开发者"),
                                move || {
                                    if let Some(owner) = page.creator.get_untracked() {
                                        page.open_profile(owner);
                                    }
                                },
                            )
                        },
                    ))
            },
        ))
        .child(
            row(
                move || tr("Copy link", "复制链接"),
                move || set_clipboard(untracked(|| page.text(|p| p.code_url.clone()))),
            )
            .disabled(move || page.missing_or(|profile| profile.code_url.is_empty())),
        )
        .child(when(
            move || page.is(|profile| !profile.is_bot),
            move || {
                page_row(
                    move || tr("Groups in common", "共同群组"),
                    move || page.destination.set(Destination::Groups),
                )
            },
        ))
        .child(page_row(
            move || tr("Shared apps", "共享应用"),
            move || page.destination.set(Destination::Apps),
        ))
}

fn contact_uri(user_id: &str) -> String {
    use base64::Engine as _;
    let data = base64::engine::general_purpose::STANDARD
        .encode(serde_json::json!({"user_id": user_id}).to_string());
    let mut url = url::Url::parse("mixin://send").expect("static Mixin URI");
    url.query_pairs_mut()
        .append_pair("category", "contact")
        .append_pair("data", &data);
    url.into()
}

fn relationship_rows(
    page: Page,
    apply: impl Fn(Action, String) + Copy + 'static,
) -> impl IntoElement {
    let blocked = move || page.is(|profile| profile.relationship == "BLOCKING");
    let friend = move || page.is(|profile| profile.relationship == "FRIEND");
    let deactivated = move || page.busy.get() || page.missing_or(|p| p.is_deactivated);
    let danger = move |title: fn() -> &'static str, prompt: fn() -> Prompt, action: Action| {
        row(title, move || {
            page.confirm_then(prompt(), move || apply(action, String::new()))
        })
        .disabled(page.busy)
        .text_color(ui::danger)
    };
    ui::card()
        .child(
            row(
                move || tr("Edit name…", "编辑名称…"),
                move || page.destination.set(Destination::EditName),
            )
            .disabled(deactivated),
        )
        .child(when(
            move || !blocked() && !friend(),
            move || {
                row(
                    move || tr("Add contact…", "添加联系人…"),
                    move || page.destination.set(Destination::AddContact),
                )
                .disabled(deactivated)
            },
        ))
        .child(when(
            move || !blocked() && friend(),
            move || {
                danger(
                    || tr("Remove contact…", "移除联系人…"),
                    remove_prompt,
                    Action::Remove,
                )
            },
        ))
        .child(when(
            move || !blocked(),
            move || danger(|| tr("Block…", "屏蔽…"), block_prompt, Action::Block),
        ))
        .child(when(blocked, move || {
            row(
                move || tr("Unblock…", "取消屏蔽…"),
                move || {
                    page.confirm_then(unblock_prompt(), move || {
                        apply(Action::Unblock, String::new())
                    })
                },
            )
            .disabled(page.busy)
        }))
        .child(danger(
            || tr("Report and block…", "举报并屏蔽…"),
            report_prompt,
            Action::Report,
        ))
}

fn remove_prompt() -> Prompt {
    Prompt::new(
        tr("Remove contact?", "移除联系人？"),
        tr(
            "This user will be removed from your contacts.",
            "此用户将从联系人中移除。",
        ),
        tr("Remove", "移除"),
    )
}

fn block_prompt() -> Prompt {
    Prompt::new(
        tr("Block this user?", "屏蔽此用户？"),
        tr(
            "Messages from this user will be blocked.",
            "将不再接收此用户的消息。",
        ),
        tr("Block", "屏蔽"),
    )
}

fn unblock_prompt() -> Prompt {
    Prompt::new(
        tr("Unblock this user?", "取消屏蔽此用户？"),
        "",
        tr("Unblock", "取消屏蔽"),
    )
}

fn report_prompt() -> Prompt {
    Prompt::new(
        tr("Report and block?", "举报并屏蔽？"),
        tr(
            "This account will be reported and blocked.",
            "将举报并屏蔽此账号。",
        ),
        tr("Report", "举报"),
    )
}

fn groups(account: Arc<AccountClient>, user_id: String, failed: Signal<bool>) -> impl IntoElement {
    let values = signal_with(Vec::<GroupConversationItem>::new(), SignalEquals::Never);
    let loading = signal(true);
    let revision = signal(0_u64);
    let query_failed = signal(false);
    let query = account.clone();
    effect(move || {
        let _revision = revision.get();
        let account = query.clone();
        let id = user_id.clone();
        loading.set(true);
        query_failed.set(false);
        spawn(async move {
            match runtime::run(async move { account.conversation().groups_in_common(id).await })
                .await
            {
                Ok(rows) => values.set(rows),
                Err(_) => query_failed.set(true),
            }
            loading.set(false);
        });
    });
    let navigation =
        use_context::<ChatNavigation>().expect("The signed-in session provides navigation");
    let account = stored(account);
    view()
        .flex_1()
        .min_h_0()
        .child(when(loading, || {
            parts::status(move || tr("Loading…", "加载中…"))
                .px(ui::space_lg())
                .py(ui::space_sm())
        }))
        .child(when(query_failed, move || {
            parts::failure(
                move || tr("Couldn’t load. Try again.", "加载失败，请重试。"),
                move || revision.update(|value| *value = value.wrapping_add(1)),
            )
            .p(ui::space_lg())
        }))
        .child(
            virtual_list(
                move || values.with(Vec::len),
                move |at| {
                    let value = move || values.with(|rows| rows.get(at.index()).cloned());
                    parts::entity_row(
                        move || {
                            value()
                                .map(|value| value.conversation_id)
                                .unwrap_or_default()
                        },
                        move || value().map(|value| value.name).unwrap_or_default(),
                        move || value().map(|value| value.avatar_url).unwrap_or_default(),
                        move || {
                            value()
                                .map(|value| i18n::current().members(value.participant_count))
                                .unwrap_or_default()
                        },
                        || false,
                    )
                    .on_press(move || {
                        if let Some(group) = value() {
                            navigation.open_conversation(
                                account.with(|account| account.clone()),
                                group.conversation_id,
                                None,
                                failed,
                            );
                        }
                    })
                },
            )
            .keys(move || {
                values.with(|rows| {
                    rows.iter()
                        .map(|group| group.conversation_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .estimated_item_size(parts::row_height())
            .flex_1()
            .min_h_0(),
        )
        .child(when(
            move || !loading.get() && !query_failed.get() && values.with(Vec::is_empty),
            || ui::placeholder(move || tr("No groups in common", "没有共同群组")),
        ))
}

fn apps(
    account: Arc<AccountClient>,
    user_id: String,
    open: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let values = signal_with(Vec::<SharedAppItem>::new(), SignalEquals::Never);
    let loading = signal(true);
    let failed = signal(false);
    let revision = signal(0_u64);
    effect(move || {
        let _revision = revision.get();
        let client = account.clone();
        let id = user_id.clone();
        loading.set(true);
        failed.set(false);
        spawn(async move {
            match runtime::run(async move { client.user().shared_apps(id).await }).await {
                Ok(rows) => values.set(rows),
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    view()
        .flex_1()
        .min_h_0()
        .child(when(loading, || {
            parts::status(move || tr("Loading…", "加载中…"))
                .px(ui::space_lg())
                .py(ui::space_sm())
        }))
        .child(when(failed, move || {
            parts::failure(
                move || tr("Couldn’t load. Try again.", "加载失败，请重试。"),
                move || revision.update(|revision| *revision += 1),
            )
            .p(ui::space_lg())
        }))
        .child(
            virtual_list(
                move || values.with(Vec::len),
                move |at| {
                    let value = move || values.with(|rows| rows.get(at.index()).cloned());
                    parts::entity_row(
                        move || value().map(|value| value.app_id).unwrap_or_default(),
                        move || value().map(|value| value.name).unwrap_or_default(),
                        move || value().map(|value| value.icon_url).unwrap_or_default(),
                        move || value().map(|value| value.description).unwrap_or_default(),
                        || false,
                    )
                    .on_press(move || {
                        if let Some(app) = value() {
                            open(app.app_id);
                        }
                    })
                },
            )
            .keys(move || {
                values.with(|rows| {
                    rows.iter()
                        .map(|app| app.app_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .estimated_item_size(parts::row_height())
            .flex_1()
            .min_h_0(),
        )
        .child(when(
            move || !loading.get() && !failed.get() && values.with(Vec::is_empty),
            || ui::placeholder(move || tr("No shared apps", "暂无共享应用")),
        ))
}
