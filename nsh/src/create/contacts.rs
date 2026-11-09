use mixin_desktop_api::{AccountClient, AccountProfile, GroupAvatar, UserProfileItem};
use nsh_core::prelude::*;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use super::{Action, modal, parts, tr};
use crate::{
    i18n::{self, Key},
    navigation::ChatNavigation,
    runtime,
    ui::{self, Typography as _},
};

const PAGE_SIZE: i64 = 60;
const MAX_SELECTED: usize = 1023;

#[derive(Clone, Copy)]
struct Form {
    profile: Stored<Box<dyn Fn() -> AccountProfile>>,
    kind: Action,
    focus: bool,
    query: Signal<String>,
    composing: Signal<bool>,
    page: Signal<i64>,
    rows: Signal<Vec<UserProfileItem>>,
    selected: Signal<BTreeMap<String, UserProfileItem>>,
    more: Signal<bool>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    busy: Signal<bool>,
    naming: Signal<bool>,
    name: Signal<String>,
    created: Signal<Option<String>>,
    revision: Signal<u64>,
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    kind: Action,
    close: impl Fn() + Copy + 'static,
    open_user: impl Fn(String) + 'static,
) -> impl IntoElement {
    let avatar_account = account.clone();
    let form = Form {
        profile: stored(
            Box::new(move || avatar_account.profile()) as Box<dyn Fn() -> AccountProfile>
        ),
        kind,
        focus: true,
        query: signal(String::new()),
        composing: signal(false),
        page: signal(0),
        rows: signal_with(Vec::new(), SignalEquals::Never),
        selected: signal_with(BTreeMap::new(), SignalEquals::Never),
        more: signal(false),
        loading: signal(true),
        failed: signal(false),
        busy: signal(false),
        naming: signal(false),
        name: signal(String::new()),
        created: signal(None),
        revision: signal(0),
    };
    let navigation = use_context::<ChatNavigation>().expect("Home provides navigation");
    let open_user = stored(Box::new(open_user) as Box<dyn Fn(String)>);
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    let cancel = move || {
        if presence.interactive() && !form.busy.get_untracked() && !form.composing.get_untracked() {
            close();
        }
    };
    let query_account = account.clone();
    effect(move || {
        if form.composing.get() || form.naming.get() {
            return;
        }
        let query = form.query.get();
        let page = form.page.get();
        let _revision = form.revision.get();
        let account = query_account.clone();
        form.loading.set(true);
        form.failed.set(false);
        spawn(async move {
            if !query.trim().is_empty() {
                timer(parts::SEARCH_DELAY).await;
            }
            let result = runtime::run(async move {
                account
                    .user()
                    .selectable_users_page(query, page * PAGE_SIZE, PAGE_SIZE + 1)
                    .await
            })
            .await;
            match result {
                Ok(mut values) => {
                    form.more.set(values.len() > PAGE_SIZE as usize);
                    values.truncate(PAGE_SIZE as usize);
                    if page == 0 {
                        form.rows.set(values);
                    } else {
                        form.rows.update(|rows| {
                            let known = rows
                                .iter()
                                .map(|user| user.user_id.clone())
                                .collect::<HashSet<_>>();
                            rows.extend(
                                values
                                    .into_iter()
                                    .filter(|user| !known.contains(&user.user_id)),
                            );
                        });
                    }
                }
                Err(_) => form.failed.set(true),
            }
            form.loading.set(false);
        });
    });
    let create_account = stored(account.clone());
    let create_group = move || {
        if form.busy.get_untracked() || form.name.with_untracked(|name| name.trim().is_empty()) {
            return;
        }
        form.busy.set(true);
        form.failed.set(false);
        let client = create_account.get();
        let name = form.name.get_untracked().trim().to_owned();
        let existing = form.created.get_untracked();
        let users = form
            .selected
            .with_untracked(|users| users.keys().cloned().collect());
        let task = runtime::commit(async move {
            let access = client.conversation();
            let id = match existing {
                Some(id) => id,
                None => access.create_group(name, users).await?,
            };
            let row = access.conversation_items_by_ids(vec![id.clone()]).await;
            Ok((id, row))
        });
        spawn(async move {
            let result = task.await;
            form.busy.set(false);
            match result {
                Ok((id, Ok(mut rows))) if !rows.is_empty() => {
                    form.created.set(Some(id));
                    navigation.select(rows.remove(0));
                    close();
                }
                Ok((id, _)) => {
                    form.created.set(Some(id));
                    form.failed.set(true);
                }
                Err(_) => form.failed.set(true),
            }
        });
    };
    modal(cancel, content(form, cancel, open_user, create_group))
}

fn contact_list(
    form: Form,
    cancel: impl Fn() + Copy + 'static,
    open_user: Stored<Box<dyn Fn(String)>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let Form {
        query,
        rows,
        loading,
        ..
    } = form;
    view()
        .flex_1()
        .min_h_0()
        .child(ui::picker::search(
            ui::search_input()
                .test_id("create-contact-search")
                .label(move || tr("Search contacts", "搜索联系人"))
                .placeholder(move || tr("Search", "搜索").to_owned())
                .auto_focus(form.focus)
                .value(move || query.get())
                .on_key_press(move |key: &str| {
                    if key == "Escape" {
                        cancel();
                    }
                })
                .on_change(move |value: &str| {
                    if query.with_untracked(|query| query == value) {
                        return;
                    }
                    batch(|| {
                        query.set(value.into());
                        form.page.set(0);
                        rows.set(Vec::new());
                        loading.set(true);
                    })
                })
                .on_composition_change(move |event: &Composition| {
                    form.composing.set(event.composing)
                }),
        ))
        .child(when(
            move || form.kind == Action::Group && form.selected.with(|rows| !rows.is_empty()),
            move || selected_people(form),
        ))
        .child(ui::picker::section(|| tr("Contacts", "联系人")))
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            move || parts::status(move || i18n.text(Key::LoadingChats)).px(ui::space_xl()),
        ))
        .child(when(
            move || !loading.get() && !form.failed.get() && rows.with(Vec::is_empty),
            move || ui::placeholder(move || i18n.text(Key::NoContacts)).min_h_24(),
        ))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    let user = move || rows.with(|rows| rows.get(at.index()).cloned());
                    ui::picker::row(
                        move || user().map(|user| user.user_id).unwrap_or_default(),
                        move || user().map(|user| user.full_name).unwrap_or_default(),
                        move || user().map(|user| user.avatar_url).unwrap_or_default(),
                        move || {
                            user().is_some_and(|user| {
                                form.selected
                                    .with(|users| users.contains_key(&user.user_id))
                            })
                        },
                        form.kind == Action::Group,
                    )
                    .disabled(form.busy)
                    .on_press(move || {
                        let Some(user) = user() else {
                            return;
                        };
                        if form.kind == Action::Chat {
                            open_user.with(|open| open(user.user_id));
                            return;
                        }
                        form.selected.update(|users| {
                            if users.remove(&user.user_id).is_none() && users.len() < MAX_SELECTED {
                                users.insert(user.user_id.clone(), user);
                            }
                        });
                    })
                },
            )
            .keys(move || {
                rows.with(|rows| {
                    rows.iter()
                        .map(|user| user.user_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .on_load_more(move || {
                if !loading.get_untracked()
                    && !form.failed.get_untracked()
                    && form.more.get_untracked()
                    && !form.busy.get_untracked()
                    && !form.composing.get_untracked()
                {
                    form.loading.set(true);
                    form.page.update(|page| *page += 1);
                }
            })
            .load_more_threshold(0.5)
            .estimated_item_size(ui::picker::row_height())
            .flex_1()
            .min_h_0()
            .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(form.failed, move || {
            parts::failure(
                || {
                    tr(
                        "Contacts are temporarily unavailable.",
                        "暂时无法加载联系人",
                    )
                },
                move || form.revision.update(|revision| *revision += 1),
            )
            .px(ui::space_xl())
            .py(ui::space_sm())
        }))
}

fn group_form(form: Form, cancel: impl Fn() + Copy + 'static) -> impl IntoElement {
    scroll().flex_1().min_h_0().child(
        view()
            .px(ui::space_xl())
            .pb(ui::space_xl())
            .gap(ui::space_lg())
            .child(
                view()
                    .items_center()
                    .child(group_preview(form.profile, form.selected)),
            )
            .child(
                text(move || {
                    format!(
                        "{}: {}",
                        tr("Participants", "群成员"),
                        form.selected.with(BTreeMap::len) + 1
                    )
                })
                .text_role(ui::TextRole::Body)
                .text_color(ui::fg_secondary),
            )
            .child(
                parts::labeled(|| tr("Group name", "群名称")).child(
                    ui::field()
                        .test_id("group-name")
                        .label(move || tr("Group name", "群名称"))
                        .placeholder(move || tr("Group name", "群名称").to_owned())
                        .auto_focus(form.focus)
                        .max_length(40)
                        .readonly(move || form.busy.get() || form.created.with(Option::is_some))
                        .value(move || form.name.get())
                        .on_key_press(move |key: &str| {
                            if key == "Escape" {
                                cancel();
                            }
                        })
                        .on_change(move |value: &str| form.name.set(value.into()))
                        .on_composition_change(move |event: &Composition| {
                            form.composing.set(event.composing)
                        }),
                ),
            )
            .child(when(
                move || form.created.with(Option::is_some),
                || {
                    text(move || {
                        tr(
                            "The group was created. Retry opening it.",
                            "群组已创建，请重试打开。",
                        )
                    })
                    .text_role(ui::TextRole::Body)
                },
            )),
    )
}

fn group_preview(
    profile: Stored<Box<dyn Fn() -> AccountProfile>>,
    selected: Signal<BTreeMap<String, UserProfileItem>>,
) -> impl IntoElement {
    view()
        .size_16()
        .rounded(ui::radius_full())
        .overflow_hidden()
        .child(dynamic(move || {
            let profile = profile.with(|read| read());
            let mut avatars = vec![GroupAvatar {
                user_id: profile.user_id,
                name: profile.full_name,
                avatar_url: profile.avatar_url,
            }];
            avatars.extend(selected.with(|users| {
                users
                    .values()
                    .take(3)
                    .map(|user| GroupAvatar {
                        user_id: user.user_id.clone(),
                        name: user.full_name.clone(),
                        avatar_url: user.avatar_url.clone(),
                    })
                    .collect::<Vec<_>>()
            }));
            untracked(|| ui::avatar_puzzle(avatars).into_element())
        }))
}

fn content(
    form: Form,
    cancel: impl Fn() + Copy + 'static,
    open_user: Stored<Box<dyn Fn(String)>>,
    create_group: impl Fn() + Copy + 'static,
) -> View {
    let i18n = i18n::current();
    let group = form.kind == Action::Group;
    let unavailable = move || form.busy.get() || form.composing.get();
    ui::picker::dialog()
        .h(move || {
            if form.naming.get() {
                ui::picker_form_height()
            } else {
                ui::picker_height()
            }
        })
        .child(ui::picker::header(
            move || i18n.text(form.kind.title()),
            move || {
                if group && form.selected.with(|users| !users.is_empty()) {
                    i18n.format(
                        "Selected contacts: {count}",
                        "已选 {count} 人",
                        &[("count", &form.selected.with(BTreeMap::len).to_string())],
                    )
                } else {
                    String::new()
                }
            },
            ui::round_button()
                .size_8()
                .disabled(move || {
                    unavailable() || (form.naming.get() && form.created.with(Option::is_some))
                })
                .accessibility_label(move || {
                    Some(
                        if form.naming.get() {
                            tr("Back", "返回")
                        } else {
                            tr("Close", "关闭")
                        }
                        .into(),
                    )
                })
                .on_press(move || {
                    if form.naming.get_untracked() {
                        form.failed.set(false);
                        form.naming.set(false);
                    } else {
                        cancel();
                    }
                })
                .child(
                    when(form.naming, || {
                        ui::icon(ui::Icon::ArrowLeft).size(ui::icon_lg())
                    })
                    .otherwise(|| ui::icon(ui::Icon::Close).size(ui::icon_lg())),
                ),
            ui::picker::commitment()
                .style_when(
                    move || !group || form.selected.with(BTreeMap::is_empty),
                    |style| style.hidden(),
                )
                .disabled(move || {
                    unavailable()
                        || (form.naming.get() && form.name.with(|name| name.trim().is_empty()))
                })
                .on_press(move || {
                    if form.naming.get_untracked() {
                        create_group();
                    } else {
                        form.naming.set(true);
                    }
                })
                .child(text(move || {
                    if form.busy.get() {
                        i18n.text(Key::Creating)
                    } else if form.created.with(Option::is_some) {
                        tr("Open", "打开")
                    } else if form.naming.get() {
                        i18n.text(Key::Create)
                    } else {
                        tr("Next", "下一步")
                    }
                })),
        ))
        .child(when(
            move || !form.naming.get(),
            move || contact_list(form, cancel, open_user),
        ))
        .child(when(form.naming, move || group_form(form, cancel)))
        .child(when(
            move || form.failed.get() && form.naming.get() && form.created.with(Option::is_none),
            move || {
                ui::error_text(|| {
                    tr(
                        "Couldn’t create the group. Try again.",
                        "未能创建群组，可以再次尝试",
                    )
                })
                .px(ui::space_xl())
                .pb(ui::space_lg())
            },
        ))
}

fn selected_people(form: Form) -> impl IntoElement {
    let rows = computed(move || {
        form.selected
            .with(|users| users.keys().cloned().collect::<Vec<_>>())
    });
    virtual_list(
        move || rows.with(Vec::len),
        move |at| {
            let user = move || {
                rows.with(|rows| rows.get(at.index()).cloned())
                    .and_then(|id| form.selected.with(|users| users.get(&id).cloned()))
            };
            ui::picker::selected_person(
                move || user().map(|user| user.user_id).unwrap_or_default(),
                move || user().map(|user| user.full_name).unwrap_or_default(),
                move || user().map(|user| user.avatar_url).unwrap_or_default(),
                move || form.busy.get(),
                move || {
                    if !form.busy.get_untracked()
                        && let Some(user) = user()
                    {
                        form.selected.update(|users| {
                            users.remove(&user.user_id);
                        });
                    }
                },
            )
        },
    )
    .horizontal()
    .keys(move || rows.get())
    .estimated_item_size(ui::space_lg() * 5.0)
    .content_padding_x(ui::space_xl())
    .h_24()
    .flex_none()
}

#[cfg(debug_assertions)]
pub(crate) fn preview(
    profile: AccountProfile,
    users: Vec<UserProfileItem>,
    group: bool,
    naming: Signal<bool>,
    selected: Signal<bool>,
    failed: Signal<bool>,
) -> impl IntoElement {
    let all = stored(users.clone());
    let form = Form {
        profile: stored(Box::new(move || profile.clone()) as Box<dyn Fn() -> AccountProfile>),
        kind: if group { Action::Group } else { Action::Chat },
        focus: false,
        query: signal(String::new()),
        composing: signal(false),
        page: signal(0),
        rows: signal_with(users, SignalEquals::Never),
        selected: signal_with(BTreeMap::new(), SignalEquals::Never),
        more: signal(false),
        loading: signal(false),
        failed,
        busy: signal(false),
        naming,
        name: signal("设计讨论".into()),
        created: signal(None),
        revision: signal(0),
    };
    effect(move || {
        form.selected.set(if selected.get() {
            all.with(|users| {
                users
                    .iter()
                    .take(2)
                    .cloned()
                    .map(|u| (u.user_id.clone(), u))
                    .collect()
            })
        } else {
            BTreeMap::new()
        });
    });
    effect(move || {
        let keyword = form.query.get().to_lowercase();
        form.rows.set(all.with(|users| {
            users
                .iter()
                .filter(|user| {
                    user.full_name.to_lowercase().contains(&keyword)
                        || user.identity_number.contains(&keyword)
                })
                .cloned()
                .collect()
        }));
        form.loading.set(false);
    });
    content(
        form,
        || {},
        stored(Box::new(|_| {}) as Box<dyn Fn(String)>),
        || {},
    )
}
