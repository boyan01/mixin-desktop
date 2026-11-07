//! Conversation row presentation and object-scoped commands.
mod actions;
mod window;
pub(crate) use actions::{State as Commands, surface as command_surface, wrap as row_menu};
pub(crate) use window::Window;

use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::prelude::*;
use std::{collections::HashMap, sync::Arc};

use crate::{
    appearance,
    i18n::{self, I18n, Key},
    navigation::ChatNavigation,
    presentation, runtime,
    ui::{self, Typography as _},
};

type Row = Memo<Option<ConversationListData>>;

/// What every recycled cell reads; per-row state stays in the rows, keyed by id.
#[derive(Clone, Copy)]
struct Cells {
    i18n: I18n,
    account_id: Stored<String>,
    drafts: Signal<HashMap<String, String>>,
    selected: Signal<Option<ConversationListData>>,
    previews: Signal<HashMap<String, (String, String)>>,
    navigation: ChatNavigation,
    commands: Commands,
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    rows: Signal<Vec<ConversationListData>>,
    drafts: Signal<HashMap<String, String>>,
    selected: Signal<Option<ConversationListData>>,
    window: Window,
) -> View {
    let cells = Cells {
        i18n: i18n::current(),
        account_id: stored(account.account_id()),
        drafts,
        selected,
        previews: signal(HashMap::new()),
        navigation: use_context().expect("Home provides chat navigation"),
        commands: use_context().expect("Home provides conversation commands"),
    };
    resolve_mentions(account, rows, cells);
    view().flex_1().min_h_0().child(
        virtual_list(
            move || rows.with(Vec::len),
            move |at| {
                // Only the cell whose row changed rebinds when the window reloads.
                cells.cell(computed(move || {
                    rows.with(|rows| rows.get(at.index()).cloned())
                }))
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
        .estimated_item_size(ui::list_row_height())
        .flex_1()
        .min_h_0()
        .on_viewable_items_change(move |items: &nsh_core::element::ViewableItems| {
            window.viewable(items)
        })
        .on_load_more(move || window.next())
        .on_load_more_at_start(move || window.previous()),
    )
}

/// Resolves only previews that can hold a mention; a cell shows a result while
/// its source text still matches, so a stale result can never be displayed.
fn resolve_mentions(
    account: Arc<AccountClient>,
    rows: Signal<Vec<ConversationListData>>,
    cells: Cells,
) {
    effect(move || {
        let sources = rows.with(|rows| {
            cells.drafts.with(|drafts| {
                rows.iter()
                    .filter(|row| {
                        row.last_message.contains('@') || draft_of(row, drafts).contains('@')
                    })
                    .map(|row| (row.conversation_id.clone(), cells.source(row, drafts)))
                    .filter(|(_, text)| text.contains('@'))
                    .collect::<Vec<_>>()
            })
        });
        if sources.is_empty() {
            cells.previews.set(HashMap::new());
            return;
        }
        let account = account.clone();
        let contents = sources.iter().map(|(_, text)| text.clone()).collect();
        // Rerunning this effect cancels the task, and with it the business call.
        spawn(async move {
            match runtime::run(async move { account.user().replace_mentions(contents).await }).await
            {
                Ok(values) => cells.previews.set(
                    sources
                        .into_iter()
                        .zip(values)
                        .map(|((id, source), value)| (id, (source, value)))
                        .collect(),
                ),
                Err(error) => log::warn!("Could not resolve conversation mentions: {error}"),
            }
        });
    });
}

impl Cells {
    fn cell(self, row: Row) -> impl IntoElement {
        let selected = move || {
            let id = read(row, |row| row.conversation_id.clone());
            self.selected
                .with(|row| row.as_ref().is_some_and(|row| row.conversation_id == id))
        };
        let content = ui::list_row()
            .selected(selected)
            .on_press(move || {
                if let Some(row) = row.get_untracked()
                    && !selected()
                {
                    self.navigation.select(row);
                }
            })
            .child(ui::conversation_avatar(
                move || row.get(),
                ui::AvatarSize::Large,
            ))
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(self.title_line(row))
                    .child(self.preview_line(row)),
            );
        actions::wrap(self.commands, move || row.get_untracked(), content)
    }

    fn title_line(self, row: Row) -> impl IntoElement {
        let i18n = self.i18n;
        let badge = computed(move || {
            read(row, |row| {
                Some((
                    row.category == "GROUP",
                    row.is_verified,
                    row.is_bot,
                    row.membership.clone(),
                ))
            })
        });
        view()
            .flex_row()
            .items_center()
            .gap_1()
            .child(
                text(move || read(row, |row| row.name.clone()))
                    .min_w_0()
                    .flex_shrink(1.0)
                    .number_of_lines(1)
                    .text_role(ui::TextRole::Label),
            )
            .child(dynamic(move || {
                let badge = badge.get();
                untracked(|| match badge {
                    Some((group, verified, bot, membership)) => {
                        appearance::identity_badge(group, verified, bot, membership.as_deref())
                    }
                    None => view().hidden().into_element(),
                })
            }))
            .child(when(
                move || read(row, |row| row.is_scam),
                move || {
                    text(move || i18n.tr("SCAM", "诈骗风险"))
                        .text_role(ui::TextRole::Caption)
                        .text_color(ui::danger)
                },
            ))
            .child(view().flex_1())
            .child(
                text(move || read(row, |row| presentation::time(i18n, row.updated_at_millis)))
                    .flex_shrink_0()
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::fg_secondary),
            )
    }

    fn preview_line(self, row: Row) -> impl IntoElement {
        let i18n = self.i18n;
        let drafted = computed(move || {
            read(row, |row| {
                self.drafts.with(|drafts| !draft_of(row, drafts).is_empty())
            })
        });
        let delivery = computed(move || {
            read(row, |row| {
                self.account_id
                    .with(|id| shows_delivery(row, id))
                    .then(|| row.last_message_status.clone().unwrap_or_default())
            })
            .filter(|_| !drafted.get())
        });
        view()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(when(
                        move || drafted.get(),
                        move || {
                            text(move || format!("{}:", i18n.text(Key::Draft)))
                                .text_role(ui::TextRole::Body)
                                .text_color(ui::danger)
                        },
                    ))
                    .child(dynamic(move || {
                        let status = delivery.get();
                        untracked(|| match status {
                            Some(status) => appearance::delivery_icon(&status),
                            None => view().hidden().into_element(),
                        })
                    }))
                    .child(when(
                        move || read(row, |row| row.expire_in > 0),
                        || ui::icon(ui::Icon::Disappearing).text_color(ui::fg_secondary),
                    ))
                    .child(
                        text(move || read(row, |row| self.preview(row)))
                            .flex_1()
                            .min_w_0()
                            .number_of_lines(1)
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::fg_secondary),
                    ),
            )
            .child(when(
                move || read(row, |row| row.mention_count > 0),
                move || {
                    ui::count_pill(|| "@".into(), || true)
                        .label(move || i18n.tr("Unread mention", "未读提及"))
                },
            ))
            .child(when(
                move || read(row, |row| row.unseen_count > 0),
                move || {
                    ui::count_pill(
                        move || read(row, |row| row.unseen_count.to_string()),
                        move || read(row, |row| !row.is_muted || row.mention_count > 0),
                    )
                },
            ))
            .child(when(
                move || read(row, |row| row.unseen_count <= 0),
                move || {
                    view()
                        .flex_row()
                        .gap_1()
                        .child(when(
                            move || read(row, |row| row.is_muted),
                            || appearance::status_icon(false),
                        ))
                        .child(when(
                            move || read(row, |row| row.is_pinned),
                            || appearance::status_icon(true),
                        ))
                },
            ))
    }

    fn source(self, row: &ConversationListData, drafts: &HashMap<String, String>) -> String {
        let draft = draft_of(row, drafts);
        if draft.is_empty() {
            self.account_id
                .with(|id| presentation::conversation_preview(self.i18n, row, id))
        } else {
            draft.to_owned()
        }
    }

    fn preview(self, row: &ConversationListData) -> String {
        let source = self.drafts.with(|drafts| self.source(row, drafts));
        self.previews.with(|previews| {
            previews
                .get(&row.conversation_id)
                .filter(|(original, _)| *original == source)
                .map(|(_, value)| value.clone())
                .unwrap_or(source)
        })
    }
}

fn read<T: Default>(row: Row, value: impl FnOnce(&ConversationListData) -> T) -> T {
    row.with(|row| row.as_ref().map(value).unwrap_or_default())
}

fn draft_of<'a>(row: &'a ConversationListData, drafts: &'a HashMap<String, String>) -> &'a str {
    if row.status == 3 {
        return "";
    }
    drafts
        .get(&row.conversation_id)
        .map_or(row.draft.as_str(), String::as_str)
}

fn shows_delivery(row: &ConversationListData, account_id: &str) -> bool {
    row.last_message_sender_id.as_deref() == Some(account_id)
        && !row
            .last_message_category
            .as_deref()
            .is_some_and(|category| {
                category.starts_with("SYSTEM_")
                    || category == "MESSAGE_RECALL"
                    || category == "MESSAGE_PIN"
            })
}
