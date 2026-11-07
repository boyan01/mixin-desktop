use mixin_desktop_api::{AccountClient, UserProfileItem};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

use super::{Action, modal, panel, tr};
use crate::{
    i18n::{self, Key},
    navigation::ChatNavigation,
    runtime, ui,
};

pub(super) fn screen(
    account: Arc<AccountClient>,
    kind: Action,
    close: impl Fn() + Copy + 'static,
    open_user: impl Fn(String) + 'static,
) -> impl IntoElement {
    let account = stored(account);
    let query = signal(String::new());
    let composing = signal(false);
    let page = signal(0_i64);
    let rows = signal_with(Vec::<UserProfileItem>::new(), SignalEquals::Never);
    let selected = signal_with(
        BTreeMap::<String, UserProfileItem>::new(),
        SignalEquals::Never,
    );
    let more = signal(false);
    let loading = signal(true);
    let failed = signal(false);
    let busy = signal(false);
    let naming = signal(false);
    let name = signal(String::new());
    let created = signal(None::<String>);
    let revision = signal(0_u64);
    let request = signal(0_u64);
    let navigation = use_context::<ChatNavigation>().expect("Home provides navigation");
    let open_user = stored(Box::new(open_user) as Box<dyn Fn(String)>);
    let i18n = i18n::current();
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            close();
        }
    };
    let cancel_key = move |key: &str| {
        if key == "Escape" {
            cancel();
        }
    };
    const SIZE: i64 = 60;
    const ROW_HEIGHT: f32 = 64.0;
    const VISIBLE_ROWS: usize = 5;
    effect(move || {
        if composing.get() || naming.get() {
            return;
        }
        let query = query.get();
        let page = page.get();
        let _revision = revision.get();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        let account = account.with(|account| account.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            if !query.trim().is_empty() {
                timer(std::time::Duration::from_millis(150)).await;
            }
            let result = runtime::run(async move {
                account
                    .user()
                    .selectable_users_page(query, page * SIZE, SIZE + 1)
                    .await
            })
            .await;
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok(mut values) => {
                    more.set(values.len() > SIZE as usize);
                    values.truncate(SIZE as usize);
                    rows.set(values);
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    let create_group = move || {
        if busy.get_untracked() || name.with_untracked(|name| name.trim().is_empty()) {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = account.with(|account| account.clone());
        let name = name.get_untracked().trim().to_owned();
        let existing = created.get_untracked();
        let users = selected.with_untracked(|users| users.keys().cloned().collect());
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
            busy.set(false);
            match result {
                Ok((id, Ok(mut rows))) if !rows.is_empty() => {
                    created.set(Some(id));
                    navigation.select(rows.remove(0));
                    close();
                }
                Ok((id, _)) => {
                    created.set(Some(id));
                    failed.set(true);
                }
                Err(_) => failed.set(true),
            }
        });
    };
    modal(cancel).child(
        panel()
            .h(auto())
            .min_h_0()
            .overflow_hidden()
            .child(
                text(move || i18n.text(kind.title()))
                    .flex_none()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(when(
                move || !naming.get(),
                move || {
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
                                .on_key_press(cancel_key)
                                .on_change(move |value: &str| {
                                    batch(|| {
                                        query.set(value.into());
                                        page.set(0);
                                    })
                                })
                                .on_composition_change(move |event: &Composition| {
                                    composing.set(event.composing)
                                }),
                        )
                        .child(when(loading, move || {
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(ui::busy_indicator(
                                    move || loading.get(),
                                    move || i18n.text(Key::LoadingChats).into(),
                                ))
                                .child(text(move || i18n.text(Key::LoadingChats)).text_sm())
                        }))
                        .child(when(
                            move || !loading.get() && !failed.get() && rows.with(Vec::is_empty),
                            move || {
                                view().min_h_24().items_center().justify_center().child(
                                    text(move || i18n.text(Key::NoContacts))
                                        .text_sm()
                                        .text_color(move || theme().muted_foreground),
                                )
                            },
                        ))
                        .child(
                            virtual_list(
                                move || rows.with(Vec::len),
                                move |at| {
                                    let user =
                                        move || rows.with(|rows| rows.get(at.index()).cloned());
                                    ui::command()
                                        .min_h_16()
                                        .justify_start()
                                        .gap_3()
                                        .disabled(busy)
                                        .selected(move || {
                                            user().is_some_and(|user| {
                                                selected
                                                    .with(|users| users.contains_key(&user.user_id))
                                            })
                                        })
                                        .selected_style(|style| style.bg(move || theme().accent))
                                        .on_press(move || {
                                            if let Some(user) = user() {
                                                if kind == Action::Chat {
                                                    open_user.with(|open| open(user.user_id));
                                                } else {
                                                    selected.update(|users| {
                                                        if users.remove(&user.user_id).is_none()
                                                            && users.len() < 1023
                                                        {
                                                            users
                                                                .insert(user.user_id.clone(), user);
                                                        }
                                                    });
                                                }
                                            }
                                        })
                                        .child(ui::avatar_view(
                                            move || {
                                                user()
                                                    .map(|user| user.full_name.into())
                                                    .unwrap_or_default()
                                            },
                                            move || {
                                                user()
                                                    .map(|user| user.avatar_url)
                                                    .unwrap_or_default()
                                            },
                                        ))
                                        .child(
                                            view()
                                                .flex_1()
                                                .min_w_0()
                                                .gap_1()
                                                .child(
                                                    text(move || {
                                                        user()
                                                            .map(|user| user.full_name)
                                                            .unwrap_or_default()
                                                    })
                                                    .text_sm()
                                                    .number_of_lines(1),
                                                )
                                                .child(
                                                    text(move || {
                                                        user()
                                                            .map(|user| user.identity_number)
                                                            .unwrap_or_default()
                                                    })
                                                    .text_xs()
                                                    .text_color(move || theme().muted_foreground),
                                                ),
                                        )
                                        .child(when(
                                            move || {
                                                user().is_some_and(|user| {
                                                    selected.with(|users| {
                                                        users.contains_key(&user.user_id)
                                                    })
                                                })
                                            },
                                            || ui::icon(ui::Icon::Check),
                                        ))
                                },
                            )
                            .keys(move || {
                                rows.with(|rows| {
                                    rows.iter()
                                        .map(|user| user.user_id.clone())
                                        .collect::<Vec<_>>()
                                })
                            })
                            .estimated_item_size(px(ROW_HEIGHT))
                            .h(move || {
                                px(rows.with(|rows| rows.len().min(VISIBLE_ROWS)) as f32
                                    * ROW_HEIGHT)
                            })
                            .flex_shrink(1.0)
                            .min_h_0()
                            .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
                        )
                        .child(when(
                            move || page.get() > 0 || more.get(),
                            move || {
                                view()
                                    .flex_none()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        ui::action(
                                            move || tr("Previous page", "上一页"),
                                            move || {
                                                page.update(|page| *page = page.saturating_sub(1))
                                            },
                                        )
                                        .disabled(move || loading.get() || page.get() == 0),
                                    )
                                    .child(text(move || (page.get() + 1).to_string()).text_sm())
                                    .child(
                                        ui::action(
                                            move || tr("Next page", "下一页"),
                                            move || page.update(|page| *page += 1),
                                        )
                                        .disabled(move || loading.get() || !more.get()),
                                    )
                            },
                        ))
                },
            ))
            .child(when(naming, move || {
                scroll().flex_shrink(1.0).min_h_0().child(
                    view()
                        .gap_4()
                        .child(group_preview(account, selected))
                        .child(
                            text(move || {
                                format!(
                                    "{}: {}",
                                    tr("Participants", "群成员"),
                                    selected.with(BTreeMap::len) + 1
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
                                .readonly(move || busy.get() || created.with(Option::is_some))
                                .value(move || name.get())
                                .on_key_press(cancel_key)
                                .on_change(move |value: &str| name.set(value.into()))
                                .on_composition_change(move |event: &Composition| {
                                    composing.set(event.composing)
                                }),
                        )
                        .child(when(
                            move || created.with(Option::is_some),
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
            }))
            .child(when(failed, move || {
                view()
                    .gap_2()
                    .child(
                        text(move || i18n.text(Key::CommandFailed))
                            .text_sm()
                            .text_color(move || theme().destructive),
                    )
                    .child(when(
                        move || !naming.get(),
                        move || {
                            ui::action(
                                move || i18n.text(Key::Retry),
                                move || revision.update(|revision| *revision += 1),
                            )
                        },
                    ))
            }))
            .child(
                view()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(view().flex_1().child(when(
                        move || kind == Action::Group && !naming.get(),
                        move || {
                            text(move || format!("{} / 1023", selected.with(BTreeMap::len)))
                                .text_xs()
                                .text_color(move || theme().muted_foreground)
                        },
                    )))
                    .child(ui::action(move || i18n.text(Key::Cancel), cancel).disabled(busy))
                    .child(when(
                        move || kind == Action::Group,
                        move || {
                            view()
                                .flex_row()
                                .gap_2()
                                .child(when(naming, move || {
                                    ui::action(
                                        move || tr("Back", "返回"),
                                        move || naming.set(false),
                                    )
                                    .disabled(move || busy.get() || created.with(Option::is_some))
                                }))
                                .child(
                                    ui::primary()
                                        .disabled(move || {
                                            busy.get()
                                                || selected.with(BTreeMap::is_empty)
                                                || (naming.get()
                                                    && name.with(|name| name.trim().is_empty()))
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
                                        })),
                                )
                        },
                    )),
            ),
    )
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
