use futures::StreamExt as _;
use mixin_desktop_api::{GroupConversationItem, SharedAppItem, UserProfileItem};

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

#[derive(Clone, Copy)]
enum Action {
    Add,
    Rename,
    Remove,
    Block,
    Unblock,
    Report,
}

pub(crate) fn profile(
    account: Arc<AccountClient>,
    user_id: String,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    profile_with_opened(account, user_id, close, || {})
}

pub(crate) fn profile_with_opened(
    account: Arc<AccountClient>,
    user_id: String,
    close: impl Fn() + Copy + 'static,
    opened: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    profile_at(account, user_id, Destination::Profile, close, opened)
}

pub(super) fn profile_at(
    account: Arc<AccountClient>,
    user_id: String,
    initial: Destination,
    close: impl Fn() + Copy + 'static,
    opened: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let owner = Owner::current();
    let opened = move || {
        if owner.is_live() {
            opened();
        }
    };
    let account = stored(account);
    let id = signal(user_id);
    let history = signal(Vec::<(String, Destination)>::new());
    let profile = signal_with(None::<UserProfileItem>, SignalEquals::Never);
    let destination = signal(initial);
    let back_button = element_ref();
    let revision = signal(0_u64);
    let request = signal(0_u64);
    let loading = signal(true);
    let busy = signal(false);
    let failed = signal(false);
    let links = use_context::<crate::links::LinkNavigation>().expect("Home provides link routing");
    let creator = signal(None::<String>);
    let navigation =
        use_context::<ChatNavigation>().expect("The signed-in session provides navigation");
    let own_id = account.with(|account| account.account_id());
    let own_id = stored(own_id);
    effect(move || {
        if initial == Destination::Apps && destination.get() == Destination::Apps {
            loading.set(false);
            failed.set(false);
            return;
        }
        let user_id = id.get();
        let _revision = revision.get();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        loading.set(true);
        failed.set(false);
        profile.set(None);
        creator.set(None);
        let client = account.with(|account| account.clone());
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
                if request.get_untracked() != ticket {
                    return;
                }
                profile.set(Some(value));
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
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok((Some(value), owner)) => {
                    profile.set(Some(value));
                    creator.set(owner);
                }
                _ => failed.set(true),
            }
            loading.set(false);
            while let Some(change) = changes.next().await {
                if !change.reload_all {
                    continue;
                }
                let client = updates.clone();
                let current = observed_id.clone();
                let result =
                    runtime::run(
                        async move { client.user().user_profile(Some(current), None).await },
                    )
                    .await;
                if request.get_untracked() != ticket {
                    return;
                }
                if let Ok(Some(value)) = result {
                    profile.set(Some(value));
                }
            }
        });
    });
    let open_profile = move |user: String| {
        batch(|| {
            history.update(|history| {
                history.push((id.get_untracked(), destination.get_untracked()));
            });
            id.set(user);
            destination.set(Destination::Profile);
        });
    };
    let apply = move |action: Action, name: String| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = account.with(|account| account.clone());
        let user_id = id.get_untracked();
        let command_request = request.get_untracked();
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
        spawn(async move {
            let result = command.await;
            busy.set(false);
            if request.get_untracked() != command_request {
                return;
            }
            if result.is_err() {
                failed.set(true);
            } else {
                let current = destination.get_untracked();
                let editing_name =
                    matches!(current, Destination::EditName | Destination::AddContact);
                if current == Destination::EditName
                    && initial == Destination::EditName
                    && history.with_untracked(Vec::is_empty)
                {
                    close();
                    return;
                }
                revision.update(|revision| *revision += 1);
                destination.set(Destination::Profile);
                if editing_name {
                    back_button.focus();
                }
            }
            busy.set(false);
        });
    };
    view().flex_1().min_h_0()
        .child(view().h_14().flex_row().items_center().px_4().gap_3()
            .child(ui::icon_button().element_ref(&back_button).size_8().px_1().disabled(busy)
                        .accessibility_label(move || Some(tr("Back", "返回").into()))
                .on_press(move || {
                    if destination.get_untracked() == initial && history.with_untracked(Vec::is_empty) {
                        close();
                    }
                    else if destination.get_untracked() != Destination::Profile {
                        let editing_name = matches!(destination.get_untracked(), Destination::EditName | Destination::AddContact);
                        destination.set(Destination::Profile);
                        if editing_name { back_button.focus(); }
                    }
                    else if let Some((previous, page)) = history.with_untracked(|history| history.last().cloned()) {
                        batch(|| {
                            history.update(|history| { history.pop(); });
                            id.set(previous);
                            destination.set(page);
                        });
                    } else { close(); }
                }).child(ui::icon(ui::Icon::ArrowLeft)))
            .child(text(move || match destination.get() {
                Destination::Profile => tr("Profile", "个人资料"), Destination::EditName => tr("Edit name", "编辑名称"),
                Destination::AddContact => tr("Add contact", "添加联系人"),
                Destination::Groups => tr("Groups in common", "共同群组"), Destination::Apps => tr("Shared apps", "共享应用"),
            }).text_sm().font_weight(FontWeight::SEMIBOLD)))
        .child(when(move || loading.get() && matches!(destination.get(), Destination::Profile | Destination::EditName | Destination::AddContact), || text(move || tr("Loading…", "加载中…")).px_4().text_sm()))
        .child(when(failed, move || view().px_4().py_2().gap_2()
            .child(text(move || tr("Couldn’t update. Try again.", "更新失败，请重试。")).text_sm().text_color(move || theme().destructive))
            .child(ui::action(move || tr("Retry", "重试"), move || revision.update(|revision| *revision += 1)))))
        .child(dynamic(move || {
            let page = destination.get(); let selected = id.get();
            untracked(|| match page {
                Destination::Groups => groups(account.with(|account| account.clone()), selected, failed).into_element(),
                Destination::Apps => apps(account.with(|account| account.clone()), selected, open_profile).into_element(),
                Destination::EditName | Destination::AddContact => when(
                    move || profile.with(Option::is_some),
                    move || {
                        let name = signal(profile.with_untracked(|profile| profile.as_ref().map(|profile| profile.full_name.clone()).unwrap_or_default()));
                        view().p_4().gap_3()
                            .child(ui::field().label(move || tr("Name", "名称")).max_length(if page == Destination::EditName { 40 } else { 64 }).readonly(busy).auto_focus(true).value(move || name.get()).on_change(move |text: &str| name.set(text.into())))
                            .child(ui::primary().disabled(move || busy.get() || name.with(|name| name.trim().is_empty()) || profile.with(|profile| profile.as_ref().is_none_or(|profile| profile.is_deactivated)))
                                .on_press(move || apply(if page == Destination::EditName { Action::Rename } else { Action::Add }, name.get_untracked().trim().into())).child(text(move || tr("Save", "保存"))))
                    },
                ).into_element(),
                Destination::Profile => scroll().flex_1().min_h_0().child(view().pb_5()
                    .child(view().p_4().gap_2().items_center()
                        .child(ui::avatar_view(move || profile.with(|profile| profile.as_ref().map(|profile| profile.full_name.clone()).unwrap_or_default().into()), move || profile.with(|profile| profile.as_ref().map(|profile| profile.avatar_url.clone()).unwrap_or_default())).size_16())
                        .child(view().flex_row().items_center().justify_center().gap_1().min_w_0()
                            .child(text(move || profile.with(|profile| profile.as_ref().map(|profile| profile.full_name.clone()).unwrap_or_default())).text_lg().font_weight(FontWeight::SEMIBOLD).number_of_lines(2))
                            .child(dynamic(move || {
                                let value = profile.get();
                                untracked(|| match value {
                                    Some(value) => crate::appearance::identity_badge(false, value.is_verified, value.is_bot, value.membership.as_deref()),
                                    None => view().hidden().into_element(),
                                })
                            })))
                        .child(text(move || profile.with(|profile| profile.as_ref().map(|profile| profile.identity_number.clone()).unwrap_or_default())).text_sm().select_text())
                        .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| !profile.is_deactivated)), move || text(move || profile.with(|profile| profile.as_ref().map(|profile| profile.biography.clone()).unwrap_or_default())).text_sm().text_center()))
                        .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.is_deactivated)), move || text(move || tr("This user has deleted their account.", "此用户已删除账号。")).text_sm().text_center().text_color(move || theme().destructive)))
                        .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.is_verified)), || text(move || tr("Verified", "已认证")).text_xs().text_color(move || theme().muted_foreground))))
                    .child(when(move || id.get() != own_id.with(|id| id.clone()), move ||
                        row(move || tr("Message", "发消息"), move || navigation.open_user_then(account.with(|account| account.clone()), id.get_untracked(), failed, opened))
                            .disabled(move || busy.get() || profile.with(Option::is_none))))
                    .child(row(move || tr("Share contact…", "分享联系人…"), move || {
                        use base64::Engine as _;
                        let data = base64::engine::general_purpose::STANDARD.encode(serde_json::json!({"user_id": id.get_untracked()}).to_string());
                        let mut url = url::Url::parse("mixin://send").expect("static Mixin URI");
                        url.query_pairs_mut().append_pair("category", "contact").append_pair("data", &data);
                        links.open(url.into(), None);
                    }).disabled(move || busy.get() || profile.with(Option::is_none)))
                    .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.is_bot)), move || view()
                        .child(row(move || tr("Open app", "打开应用"), move || links.open(format!("mixin://apps/{}?action=open", id.get_untracked()), None)))
                        .child(when(move || creator.with(Option::is_some), move || row(move || tr("Created by", "开发者"), move || {
                            if let Some(owner) = creator.get_untracked() { open_profile(owner); }
                        })))))
                    .child(row(move || tr("Copy link", "复制链接"), move || {
                        if let Some(link) = profile.with(|profile| profile.as_ref().map(|profile| profile.code_url.clone())).filter(|link| !link.is_empty()) { set_clipboard(link); }
                    }).disabled(move || profile.with(|profile| profile.as_ref().is_none_or(|profile| profile.code_url.is_empty()))))
                    .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| !profile.is_bot)), move || row(move || tr("Groups in common", "共同群组"), move || destination.set(Destination::Groups))))
                    .child(row(move || tr("Shared apps", "共享应用"), move || destination.set(Destination::Apps)))
                    .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.user_id != own_id.with(|id| id.clone()))), move ||
                        row(move || tr("Edit name…", "编辑名称…"), move || destination.set(Destination::EditName)).disabled(move || busy.get() || profile.with(|profile| profile.as_ref().is_none_or(|profile| profile.is_deactivated)))))
                    .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.user_id != own_id.with(|id| id.clone()) && profile.relationship != "BLOCKING")), move || view()
                        .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.relationship != "FRIEND")), move ||
                            row(move || tr("Add contact…", "添加联系人…"), move || destination.set(Destination::AddContact)).disabled(move || busy.get() || profile.with(|profile| profile.as_ref().is_none_or(|profile| profile.is_deactivated)))))
                        .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.relationship == "FRIEND")), move || row(move || tr("Remove contact…", "移除联系人…"), move || {
                            spawn(async move { if confirm(tr("Remove contact?", "移除联系人？"), tr("This user will be removed from your contacts.", "此用户将从联系人中移除。").into(), tr("Remove", "移除")).await { apply(Action::Remove, String::new()); } });
                        }).disabled(busy).text_color(move || theme().destructive)))
                        .child(row(move || tr("Block…", "屏蔽…"), move || {
                            spawn(async move { if confirm(tr("Block this user?", "屏蔽此用户？"), tr("Messages from this user will be blocked.", "将不再接收此用户的消息。").into(), tr("Block", "屏蔽")).await { apply(Action::Block, String::new()); } });
                        }).disabled(busy).text_color(move || theme().destructive))))
                    .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.relationship == "BLOCKING")), move || row(move || tr("Unblock…", "取消屏蔽…"), move || {
                        spawn(async move { if confirm(tr("Unblock this user?", "取消屏蔽此用户？"), String::new(), tr("Unblock", "取消屏蔽")).await { apply(Action::Unblock, String::new()); } });
                    }).disabled(busy)))
                    .child(when(move || profile.with(|profile| profile.as_ref().is_some_and(|profile| profile.user_id != own_id.with(|id| id.clone()))), move || row(move || tr("Report and block…", "举报并屏蔽…"), move || {
                        spawn(async move { if confirm(tr("Report and block?", "举报并屏蔽？"), tr("This account will be reported and blocked.", "将举报并屏蔽此账号。").into(), tr("Report", "举报")).await { apply(Action::Report, String::new()); } });
                    }).disabled(busy).text_color(move || theme().destructive))))
                    .into_element(),
            })
        }))
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
            text(move || tr("Loading…", "加载中…")).px_4().text_sm()
        }))
        .child(when(query_failed, move || {
            view()
                .p_4()
                .gap_2()
                .child(
                    text(move || tr("Couldn’t load. Try again.", "加载失败，请重试。"))
                        .text_sm()
                        .text_color(move || theme().destructive),
                )
                .child(ui::action(
                    move || tr("Retry", "重试"),
                    move || revision.update(|value| *value = value.wrapping_add(1)),
                ))
        }))
        .child(
            virtual_list(
                move || values.with(Vec::len),
                move |at| {
                    let value = move || values.with(|rows| rows.get(at.index()).cloned());
                    ui::command()
                        .justify_start()
                        .px_4()
                        .min_h_16()
                        .gap_3()
                        .child(ui::avatar_view(
                            move || value().map(|value| value.name.into()).unwrap_or_default(),
                            move || value().map(|value| value.avatar_url).unwrap_or_default(),
                        ))
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .gap_1()
                                .child(
                                    text(move || {
                                        value().map(|value| value.name).unwrap_or_default()
                                    })
                                    .text_sm()
                                    .number_of_lines(1),
                                )
                                .child(
                                    text(move || {
                                        value()
                                            .map(|value| {
                                                i18n::current().members(value.participant_count)
                                            })
                                            .unwrap_or_default()
                                    })
                                    .text_xs()
                                    .text_color(move || theme().muted_foreground),
                                ),
                        )
                        .on_press(move || {
                            if let Some(group) = value() {
                                let account = account.with(|account| account.clone());
                                spawn(async move {
                                    match runtime::run(async move {
                                        account
                                            .conversation()
                                            .conversation_items_by_ids(vec![group.conversation_id])
                                            .await
                                    })
                                    .await
                                    {
                                        Ok(mut rows) if !rows.is_empty() => {
                                            navigation.select(rows.remove(0))
                                        }
                                        _ => failed.set(true),
                                    }
                                });
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
            .estimated_item_size(px(64.0))
            .flex_1()
            .min_h_0(),
        )
        .child(when(
            move || !loading.get() && !query_failed.get() && values.with(Vec::is_empty),
            || {
                text(move || tr("No groups in common", "没有共同群组"))
                    .p_4()
                    .text_sm()
                    .text_color(move || theme().muted_foreground)
            },
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
            text(move || tr("Loading…", "加载中…")).px_4().text_sm()
        }))
        .child(when(failed, move || {
            ui::action(
                move || tr("Retry", "重试"),
                move || revision.update(|revision| *revision += 1),
            )
        }))
        .child(
            virtual_list(
                move || values.with(Vec::len),
                move |at| {
                    let value = move || values.with(|rows| rows.get(at.index()).cloned());
                    ui::command()
                        .justify_start()
                        .px_4()
                        .min_h_16()
                        .gap_3()
                        .child(ui::avatar_view(
                            move || value().map(|value| value.name.into()).unwrap_or_default(),
                            move || value().map(|value| value.icon_url).unwrap_or_default(),
                        ))
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .gap_1()
                                .child(
                                    text(move || {
                                        value().map(|value| value.name).unwrap_or_default()
                                    })
                                    .text_sm()
                                    .number_of_lines(1),
                                )
                                .child(
                                    text(move || {
                                        value().map(|value| value.description).unwrap_or_default()
                                    })
                                    .text_xs()
                                    .number_of_lines(2)
                                    .text_color(move || theme().muted_foreground),
                                ),
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
            .estimated_item_size(px(64.0))
            .flex_1()
            .min_h_0(),
        )
        .child(when(
            move || !loading.get() && !failed.get() && values.with(Vec::is_empty),
            || {
                text(move || tr("No shared apps", "暂无共享应用"))
                    .p_4()
                    .text_sm()
                    .text_color(move || theme().muted_foreground)
            },
        ))
}
