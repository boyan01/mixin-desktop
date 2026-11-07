use mixin_desktop_api::{AccountClient, UserProfileItem};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

use super::{Action, modal, panel, parts, tr};
use crate::{
    i18n::{self, Key},
    navigation::ChatNavigation,
    runtime, ui,
};

const PAGE_SIZE: i64 = 60;
const MAX_SELECTED: usize = 1023;

#[derive(Clone, Copy)]
struct Form {
    account: Stored<Arc<AccountClient>>,
    kind: Action,
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
    let form = Form {
        account: stored(account),
        kind,
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
    let i18n = i18n::current();
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    let cancel = move || {
        if presence.interactive() && !form.busy.get_untracked() && !form.composing.get_untracked() {
            close();
        }
    };
    effect(move || {
        if form.composing.get() || form.naming.get() {
            return;
        }
        let query = form.query.get();
        let page = form.page.get();
        let _revision = form.revision.get();
        let account = form.account.with(|account| account.clone());
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
                    form.rows.set(values);
                }
                Err(_) => form.failed.set(true),
            }
            form.loading.set(false);
        });
    });
    let create_group = move || {
        if form.busy.get_untracked() || form.name.with_untracked(|name| name.trim().is_empty()) {
            return;
        }
        form.busy.set(true);
        form.failed.set(false);
        let client = form.account.with(|account| account.clone());
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
    modal(cancel).child(
        panel()
            .child(ui::modal_title(move || i18n.text(kind.title())))
            .child(when(
                move || !form.naming.get(),
                move || contact_list(form, cancel, open_user),
            ))
            .child(when(form.naming, move || group_form(form, cancel)))
            .child(when(
                move || form.failed.get() && !form.naming.get(),
                move || {
                    parts::failure(
                        move || i18n.text(Key::CommandFailed),
                        move || form.revision.update(|revision| *revision += 1),
                    )
                },
            ))
            .child(when(
                move || form.failed.get() && form.naming.get(),
                move || ui::error_text(move || i18n.text(Key::CommandFailed)),
            ))
            .child(footer(form, cancel, create_group)),
    )
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
        .flex_shrink(1.0)
        .min_h_0()
        .gap_3()
        .child(
            ui::field()
                .label(move || tr("Search contacts", "搜索联系人"))
                .placeholder(move || tr("Search contacts", "搜索联系人").to_owned())
                .auto_focus(true)
                .value(move || query.get())
                .on_key_press(move |key: &str| {
                    if key == "Escape" {
                        cancel();
                    }
                })
                .on_change(move |value: &str| {
                    batch(|| {
                        query.set(value.into());
                        form.page.set(0);
                    })
                })
                .on_composition_change(move |event: &Composition| {
                    form.composing.set(event.composing)
                }),
        )
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            move || parts::status(move || i18n.text(Key::LoadingChats)),
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
                    parts::entity_row(
                        move || user().map(|user| user.full_name).unwrap_or_default(),
                        move || user().map(|user| user.avatar_url).unwrap_or_default(),
                        move || user().map(|user| user.identity_number).unwrap_or_default(),
                        move || {
                            user().is_some_and(|user| {
                                form.selected
                                    .with(|users| users.contains_key(&user.user_id))
                            })
                        },
                    )
                    .px_3()
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
            .estimated_item_size(parts::row_height())
            .h(move || parts::dialog_list_height(rows.with(Vec::len)))
            .flex_shrink(1.0)
            .min_h_0()
            .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(parts::pager(
            form.page,
            move || form.more.get(),
            move || loading.get(),
        ))
}

fn group_form(form: Form, cancel: impl Fn() + Copy + 'static) -> impl IntoElement {
    scroll().flex_shrink(1.0).min_h_0().child(
        view()
            .gap_4()
            .child(group_preview(form.account, form.selected))
            .child(
                text(move || {
                    format!(
                        "{}: {}",
                        tr("Participants", "群成员"),
                        form.selected.with(BTreeMap::len) + 1
                    )
                })
                .text_sm()
                .text_color(move || theme().muted_foreground),
            )
            .child(
                ui::field()
                    .label(move || tr("Group name", "群名称"))
                    .placeholder(move || tr("Group name", "群名称").to_owned())
                    .auto_focus(true)
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
                    .text_sm()
                },
            )),
    )
}

fn footer(
    form: Form,
    cancel: impl Fn() + Copy + 'static,
    create_group: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let Form {
        busy,
        naming,
        created,
        selected,
        ..
    } = form;
    let group = form.kind == Action::Group;
    view()
        .flex_none()
        .flex_row()
        .items_center()
        .gap_2()
        .child(view().flex_1().child(when(
            move || group && !naming.get(),
            move || {
                text(move || format!("{} / {MAX_SELECTED}", selected.with(BTreeMap::len)))
                    .text_xs()
                    .text_color(move || theme().muted_foreground)
            },
        )))
        .child(ui::action(move || i18n.text(Key::Cancel), cancel).disabled(busy))
        .child(when(
            move || group && naming.get(),
            move || {
                ui::action(move || tr("Back", "返回"), move || naming.set(false))
                    .disabled(move || busy.get() || created.with(Option::is_some))
            },
        ))
        .child(when(
            move || group,
            move || {
                ui::primary()
                    .disabled(move || {
                        busy.get()
                            || selected.with(BTreeMap::is_empty)
                            || (naming.get() && form.name.with(|name| name.trim().is_empty()))
                    })
                    .on_press(move || {
                        if naming.get_untracked() {
                            create_group();
                        } else {
                            naming.set(true);
                        }
                    })
                    .child(text(move || {
                        if busy.get() {
                            i18n.text(Key::Creating)
                        } else if created.with(Option::is_some) {
                            tr("Open", "打开")
                        } else if naming.get() {
                            i18n.text(Key::Create)
                        } else {
                            tr("Next", "下一步")
                        }
                    }))
            },
        ))
}

fn group_preview(
    account: Stored<Arc<AccountClient>>,
    selected: Signal<BTreeMap<String, UserProfileItem>>,
) -> impl IntoElement {
    view()
        .size(px(60.0))
        .rounded_full()
        .overflow_hidden()
        .child(dynamic(move || {
            let profile = account.with(|account| account.profile());
            let mut avatars = vec![(profile.full_name, profile.avatar_url)];
            avatars.extend(selected.with(|users| {
                users
                    .values()
                    .take(3)
                    .map(|user| (user.full_name.clone(), user.avatar_url.clone()))
                    .collect::<Vec<_>>()
            }));
            untracked(|| {
                let tile = |index: usize| {
                    let name = stored(avatars[index].0.clone());
                    let source = stored(avatars[index].1.clone());
                    view().flex_1().min_h_0().min_w_0().overflow_hidden().child(
                        ui::avatar_view(
                            move || name.with(|name| name.clone().into()),
                            move || source.with(|source| source.clone()),
                        )
                        .size_full()
                        .rounded(px(0.0))
                        .image_class("object-cover"),
                    )
                };
                match avatars.len() {
                    1 => tile(0).into_element(),
                    2 => view()
                        .flex_1()
                        .flex_row()
                        .child(tile(0))
                        .child(tile(1))
                        .into_element(),
                    3 => view()
                        .flex_1()
                        .flex_row()
                        .child(tile(0))
                        .child(view().flex_1().child(tile(1)).child(tile(2)))
                        .into_element(),
                    _ => view()
                        .flex_1()
                        .flex_row()
                        .child(view().flex_1().child(tile(0)).child(tile(1)))
                        .child(view().flex_1().child(tile(2)).child(tile(3)))
                        .into_element(),
                }
            })
        }))
}
