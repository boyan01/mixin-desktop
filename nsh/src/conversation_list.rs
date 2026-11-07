//! Conversation row presentation and object-scoped commands.
mod actions;
mod window;
pub(crate) use actions::{State as Commands, surface as command_surface, wrap as row_menu};
pub(crate) use window::Window;

use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::{collections::HashMap, sync::Arc};

use crate::{
    appearance,
    i18n::{self, I18n, Key},
    navigation::ChatNavigation,
    presentation, runtime, ui,
};

pub(crate) fn screen(
    account: Arc<AccountClient>,
    rows: Signal<Vec<ConversationListData>>,
    drafts: Signal<HashMap<String, String>>,
    selected: Signal<Option<ConversationListData>>,
    window: Window,
) -> View {
    let i18n = i18n::current();
    let account_id = stored(account.account_id());
    let navigation = use_context::<ChatNavigation>().expect("Home provides chat navigation");
    let state = use_context::<Commands>().expect("Home provides conversation commands");
    let previews = signal(HashMap::<String, (String, String)>::new());
    let request = signal(0_u64);
    let sources = move || {
        rows.with(|rows| {
            drafts.with(|drafts| {
                account_id.with(|user_id| {
                    rows.iter()
                        .map(|row| {
                            (
                                row.conversation_id.clone(),
                                preview_text(i18n, row, drafts, user_id),
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
        })
    };
    effect(move || {
        let sources_snapshot = sources();
        request.update(|version| *version = version.wrapping_add(1));
        let version = request.get_untracked();
        if !sources_snapshot.iter().any(|(_, text)| text.contains('@')) {
            previews.set(HashMap::new());
            return;
        }
        let account = account.clone();
        let contents = sources_snapshot
            .iter()
            .map(|(_, text)| text.clone())
            .collect();
        // spawn's effect cleanup cancels the old UI task; runtime::run also
        // aborts its business task. Versions and source text reject stale data.
        spawn(async move {
            let result =
                runtime::run(async move { account.user().replace_mentions(contents).await }).await;
            if request.get_untracked() != version || untracked(sources) != sources_snapshot {
                return;
            }
            match result {
                Ok(values) => previews.set(
                    sources_snapshot
                        .into_iter()
                        .zip(values)
                        .map(|((id, source), value)| (id, (source, value)))
                        .collect(),
                ),
                Err(error) => log::warn!("Could not resolve conversation mentions: {error}"),
            }
        });
    });
    view().flex_1().min_h_0().child(
        virtual_list(
            move || rows.with(Vec::len),
            move |at| {
                let row = move || rows.with(|rows| rows.get(at.index()).cloned());
                let draft = move || {
                    row()
                        .map(|row| {
                            if row.status == 3 {
                                String::new()
                            } else {
                                drafts
                                    .with(|drafts| drafts.get(&row.conversation_id).cloned())
                                    .unwrap_or(row.draft)
                            }
                        })
                        .unwrap_or_default()
                };
                let content = ui::command()
                    .justify_start()
                    .px_3()
                    .py_3()
                    .min_h_20()
                    .gap_3()
                    .mx_2()
                    .selected(move || {
                        row().is_some_and(|row| {
                            selected.with(|selected| {
                                selected.as_ref().is_some_and(|selected| {
                                    selected.conversation_id == row.conversation_id
                                })
                            })
                        })
                    })
                    .selected_style(|style| style.bg(move || theme().accent))
                    .on_press(move || {
                        if let Some(row) = row()
                            && !selected.with(|selected| {
                                selected.as_ref().is_some_and(|selected| {
                                    selected.conversation_id == row.conversation_id
                                })
                            })
                        {
                            navigation.select(row);
                        }
                    })
                    .child(ui::conversation_avatar(row, px(48.0)))
                    .child(
                        view()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        text(move || row().map(|row| row.name).unwrap_or_default())
                                            .min_w_0()
                                            .number_of_lines(1)
                                            .font_weight(FontWeight::MEDIUM),
                                    )
                                    .child(dynamic(move || {
                                        let row = row();
                                        untracked(|| match row {
                                            Some(row) => appearance::identity_badge(
                                                row.category == "GROUP",
                                                row.is_verified,
                                                row.is_bot,
                                                row.membership.as_deref(),
                                            ),
                                            None => view().hidden().into_element(),
                                        })
                                    }))
                                    .child(when(
                                        move || row().is_some_and(|row| row.is_scam),
                                        move || {
                                            text(move || i18n.tr("SCAM", "诈骗风险"))
                                                .text_xs()
                                                .text_color(move || theme().destructive)
                                        },
                                    ))
                                    .child(view().flex_1())
                                    .child(
                                        text(move || {
                                            row()
                                                .map(|row| {
                                                    presentation::time(i18n, row.updated_at_millis)
                                                })
                                                .unwrap_or_default()
                                        })
                                        .text_xs()
                                        .text_color(move || theme().muted_foreground),
                                    ),
                            )
                            .child(
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(when(
                                        move || !draft().is_empty(),
                                        move || {
                                            text(move || format!("{}:", i18n.text(Key::Draft)))
                                                .text_sm()
                                                .text_color(move || {
                                                    theme()
                                                        .color("draft")
                                                        .unwrap_or(theme().destructive)
                                                })
                                        },
                                    ))
                                    .child(when(
                                        move || {
                                            draft().is_empty()
                                                && row().is_some_and(|row| {
                                                    row.last_message_sender_id.as_deref()
                                                        == Some(&account_id.with(|id| id.clone()))
                                                        && !row
                                                            .last_message_category
                                                            .as_deref()
                                                            .is_some_and(|category| {
                                                                category.starts_with("SYSTEM_")
                                                                    || category == "MESSAGE_RECALL"
                                                                    || category == "MESSAGE_PIN"
                                                            })
                                                })
                                        },
                                        move || {
                                            dynamic(move || {
                                                let status = row()
                                                    .and_then(|row| row.last_message_status)
                                                    .unwrap_or_default();
                                                untracked(|| appearance::delivery_icon(&status))
                                            })
                                        },
                                    ))
                                    .child(
                                        text(move || {
                                            row()
                                                .map(|row| {
                                                    let source = drafts.with(|drafts| {
                                                        account_id.with(|user_id| {
                                                            preview_text(
                                                                i18n, &row, drafts, user_id,
                                                            )
                                                        })
                                                    });
                                                    previews.with(|previews| {
                                                        previews
                                                            .get(&row.conversation_id)
                                                            .filter(|(original, _)| {
                                                                original == &source
                                                            })
                                                            .map(|(_, value)| value.clone())
                                                            .unwrap_or(source)
                                                    })
                                                })
                                                .unwrap_or_default()
                                        })
                                        .flex_1()
                                        .min_w_0()
                                        .number_of_lines(1)
                                        .text_sm()
                                        .text_color(move || theme().muted_foreground),
                                    )
                                    .child(when(
                                        move || row().is_some_and(|row| row.mention_count > 0),
                                        move || unread_badge("@".into(), false, "Unread mention"),
                                    ))
                                    .child(when(
                                        move || row().is_some_and(|row| row.unseen_count > 0),
                                        move || {
                                            view()
                                                .min_w_6()
                                                .h_5()
                                                .px_1()
                                                .rounded_full()
                                                .items_center()
                                                .justify_center()
                                                .bg(move || {
                                                    if row().is_some_and(|row| row.is_muted) {
                                                        theme().muted_foreground
                                                    } else {
                                                        theme().primary
                                                    }
                                                })
                                                .child(
                                                    text(move || {
                                                        row()
                                                            .map(|row| row.unseen_count.to_string())
                                                            .unwrap_or_default()
                                                    })
                                                    .text_xs()
                                                    .text_color(move || theme().primary_foreground),
                                                )
                                        },
                                    ))
                                    .child(when(
                                        move || row().is_some_and(|row| row.unseen_count <= 0),
                                        move || {
                                            view()
                                                .flex_row()
                                                .gap_1()
                                                .child(when(
                                                    move || row().is_some_and(|row| row.is_muted),
                                                    || appearance::status_icon(false),
                                                ))
                                                .child(when(
                                                    move || row().is_some_and(|row| row.is_pinned),
                                                    || appearance::status_icon(true),
                                                ))
                                        },
                                    )),
                            ),
                    );
                actions::wrap(state, row, content)
            },
        )
        .keys(move || {
            rows.with(|rows| {
                rows.iter()
                    .map(|row| row.conversation_id.clone())
                    .collect::<Vec<_>>()
            })
        })
        .element_ref(&window.list)
        .estimated_item_size(px(80.0))
        .flex_1()
        .min_h_0()
        .on_viewable_items_change(move |items: &nsh_core::element::ViewableItems| {
            window.viewable(items)
        })
        .on_load_more(move || window.next())
        .on_load_more_at_start(move || window.previous()),
    )
}

fn preview_text(
    i18n: I18n,
    row: &ConversationListData,
    drafts: &HashMap<String, String>,
    current_user_id: &str,
) -> String {
    let draft = if row.status == 3 {
        ""
    } else {
        drafts
            .get(&row.conversation_id)
            .map(String::as_str)
            .unwrap_or(&row.draft)
    };
    if draft.is_empty() {
        presentation::conversation_preview(i18n, row, current_user_id)
    } else {
        draft.to_owned()
    }
}

fn unread_badge(value: SharedString, muted: bool, label: &'static str) -> impl IntoElement {
    view()
        .min_w_5()
        .h_5()
        .px_1()
        .rounded_full()
        .items_center()
        .justify_center()
        .label(label)
        .bg(move || {
            if muted {
                theme().muted_foreground
            } else {
                theme().primary
            }
        })
        .child(
            text(value)
                .text_xs()
                .text_color(move || theme().primary_foreground),
        )
}
