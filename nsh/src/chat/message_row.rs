//! One timeline row: day and unread separators, then a notice or a bubble.
use mixin_desktop_api::MessageListView;
use nsh_core::prelude::*;

use super::{
    message_actions::{self, Actions},
    message_appearance::{self, bubble_surface},
    message_content::{self, Placement},
    message_media, message_system,
    timeline::Timeline,
    unread::Unread,
};
use crate::{
    appearance::Appearance,
    presentation,
    ui::{self, Typography},
};

/// What a row shows, derived from the window at its index.
#[derive(Clone, PartialEq)]
struct Line {
    message: MessageListView,
    mine: bool,
    day: Option<String>,
    first_of_sender: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Rows {
    pub actions: Actions,
    pub messages: Signal<Vec<MessageListView>>,
    pub target: Signal<Option<String>>,
    pub timeline: Timeline,
    pub unread: Unread,
    pub history: ElementRef,
    pub own_id: Stored<String>,
    pub sender_names: Memo<bool>,
    pub encrypted: Signal<bool>,
    pub viewable: Signal<Option<(usize, usize)>>,
    pub unobstructed: Memo<bool>,
}

impl Rows {
    fn line(self, index: usize) -> Option<Line> {
        let i18n = self.actions.i18n;
        self.messages.with(|rows| {
            let row = rows.get(index)?;
            let older = rows.get(index + 1);
            let day = presentation::day(i18n, row.created_at_micros);
            let new_day =
                older.is_none_or(|older| presentation::day(i18n, older.created_at_micros) != day);
            Some(Line {
                message: row.clone(),
                mine: self.own_id.with(|id| row.sender_id == *id),
                first_of_sender: new_day
                    || older.is_none_or(|older| {
                        older.sender_id != row.sender_id
                            || older.category.starts_with("SYSTEM_")
                            || older.category == "MESSAGE_PIN"
                    }),
                day: new_day.then_some(day),
            })
        })
    }

    pub fn row(self, at: Position) -> impl IntoElement {
        let Self {
            actions,
            target,
            timeline,
            unread,
            history,
            viewable,
            unobstructed,
            ..
        } = self;
        let i18n = actions.i18n;
        // A splice moves the window before it moves this cell's index, so a
        // binding on both would see another row in between and rebuild. An
        // effect runs after both settled; an equal line then notifies no one.
        let line = signal(untracked(|| self.line(at.index())));
        effect(move || line.set(self.line(at.index())));
        let message = move || line.with(|line| line.as_ref().map(|line| line.message.clone()));
        let id =
            move || line.with(|line| line.as_ref().map(|line| line.message.message_id.clone()));
        let centered = move || {
            line.with(|line| {
                line.as_ref()
                    .is_some_and(|line| message_system::centered(&line.message.category))
            })
        };
        let day = computed(move || line.with(|line| line.as_ref().and_then(|l| l.day.clone())));
        // Delivery progress updates the metadata, not the content beside it.
        let content = computed(move || {
            line.with(|line| {
                line.as_ref().map(|line| {
                    let mut message = line.message.clone();
                    if matches!(message.status.as_str(), "DELIVERED" | "READ") {
                        message.status = "SENT".into();
                    }
                    message
                })
            })
        });
        let status = computed(move || {
            line.with(|line| {
                line.as_ref()
                    .map(|line| line.message.status.clone())
                    .unwrap_or_default()
            })
        });
        crate::sticker::visibility(move || {
            unobstructed.get()
                && viewable
                    .get()
                    .is_some_and(|(first, last)| (first..=last).contains(&at.index()))
        });
        let row_ref = element_ref();
        unread.observe_row(id, move || at.index() == 0, row_ref, history);
        let selected =
            move || id().is_some_and(|id| actions.selected.with(|selected| selected.contains(&id)));
        view()
            .element_ref(&row_ref)
            .px_4()
            .py(ui::space_2xs())
            .gap_2()
            .rounded(ui::radius_md())
            .style_when(selected, |style| style.bg(ui::state_selected))
            .child(unread.divider(i18n, id))
            .child(when(
                move || day.with(Option::is_some),
                move || day_separator(day),
            ))
            .child(when(centered, move || {
                dynamic(move || {
                    let row = content.get();
                    untracked(|| match row {
                        Some(row) => message_system::notice(&row, actions, Some(target)),
                        None => view().hidden().into_element(),
                    })
                })
            }))
            .child(when(
                move || !centered(),
                move || self.message_line(line, message, content, status, timeline),
            ))
    }

    fn message_line(
        self,
        line: Signal<Option<Line>>,
        message: impl Fn() -> Option<MessageListView> + Copy + 'static,
        content: Memo<Option<MessageListView>>,
        status: Memo<String>,
        timeline: Timeline,
    ) -> impl IntoElement {
        let Self {
            actions,
            target,
            encrypted,
            sender_names,
            ..
        } = self;
        let appearance = use_context::<Appearance>().expect("Home provides chat appearance");
        let i18n = actions.i18n;
        let mine = move || line.with(|line| line.as_ref().is_some_and(|line| line.mine));
        let others = move || sender_names.get() && !mine();
        let first_of_sender =
            move || line.with(|line| line.as_ref().is_some_and(|line| line.first_of_sender));
        let inline = move || {
            line.with(|line| {
                line.as_ref()
                    .is_some_and(|line| message_media::inline_metadata(&line.message))
            })
        };
        let highlighted = move || {
            timeline.highlight.with(|target| {
                target.is_some()
                    && line.with(|line| {
                        line.as_ref().map(|line| &line.message.message_id) == target.as_ref()
                    })
            })
        };
        let body = dynamic(move || {
            let row = content.get();
            untracked(|| match row {
                Some(row) => message_content::body(
                    row,
                    actions,
                    Placement {
                        target: Some(target),
                        transcript: None,
                        status: Some(status),
                    },
                ),
                None => view().hidden().into_element(),
            })
        });
        view()
            .flex_row()
            .items_start()
            .gap_2()
            .style_when(mine, |style| style.justify_end())
            .style_when(first_of_sender, |style| style.pt_2())
            .child(when(
                move || !actions.selected.with(Vec::is_empty),
                move || selection_box(message, actions),
            ))
            .child(when(
                move || others() && appearance.show_avatar(),
                move || {
                    view()
                        .size_8()
                        .flex_shrink_0()
                        .child(when(first_of_sender, move || {
                            sender_avatar(message, actions)
                        }))
                },
            ))
            .child(
                view()
                    .max_w(ui::bubble_width())
                    .min_w_0()
                    .flex_shrink(1.0)
                    .gap_1()
                    .child(when(
                        move || others() && first_of_sender(),
                        move || message_appearance::sender(message, actions),
                    ))
                    .child(
                        view()
                            .px_3()
                            .py_2()
                            .rounded(ui::radius_md())
                            .flex_row()
                            .items_end()
                            .gap_2()
                            .style_when(inline, |style| style.p_0())
                            .bg(move || bubble_surface(mine()))
                            .child(when(highlighted, move || {
                                view()
                                    .absolute()
                                    .inset_0()
                                    .rounded(ui::radius_md())
                                    .border_2()
                                    .border_color(ui::focus_ring)
                                    .bind_opacity(&timeline.highlight_opacity)
                                    .pointer_events(PointerEvents::None)
                            }))
                            .child(message_actions::menu(message, actions, body))
                            .child(
                                view()
                                    .flex_row()
                                    .flex_shrink_0()
                                    .items_center()
                                    .gap_1()
                                    .style_when(inline, |style| style.hidden())
                                    .child(when(encrypted, || {
                                        ui::icon(ui::Icon::Lock)
                                            .size_3()
                                            .text_color(ui::fg_secondary)
                                    }))
                                    .child(
                                        text(move || {
                                            line.with(|line| {
                                                line.as_ref()
                                                    .map(|line| {
                                                        presentation::time(
                                                            i18n,
                                                            line.message.created_at_micros / 1000,
                                                        )
                                                    })
                                                    .unwrap_or_default()
                                            })
                                        })
                                        .text_role(ui::TextRole::Micro)
                                        .text_color(ui::fg_secondary),
                                    )
                                    .child(when(mine, move || {
                                        message_appearance::delivery_status(status)
                                    })),
                            ),
                    ),
            )
    }
}

fn day_separator(day: Memo<Option<String>>) -> impl IntoElement {
    view().items_center().py_2().child(
        view()
            .px_3()
            .py_1()
            .rounded(ui::radius_full())
            .bg(ui::day_background)
            .child(
                text(move || day.get().unwrap_or_default())
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::day_foreground),
            ),
    )
}

fn selection_box(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let selected = actions.selected;
    let checked = move || {
        message().is_some_and(|message| selected.with(|ids| ids.contains(&message.message_id)))
    };
    ui::checkbox(
        ui::NO_LABEL,
        checked,
        || false,
        move |_| {
            if let Some(id) = message().map(|message| message.message_id) {
                selected.update(|ids| {
                    if ids.contains(&id) {
                        ids.retain(|value| value != &id);
                    } else {
                        ids.push(id);
                    }
                });
            }
        },
    )
    .size_8()
    .justify_center()
    .accessibility_label(move || Some(i18n.tr("Select message", "选择消息").into()))
}

fn sender_avatar(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
) -> impl IntoElement {
    let i18n = actions.i18n;
    ui::round_button()
        .size_8()
        .accessibility_label(move || Some(i18n.tr("Open profile", "打开个人资料").into()))
        .on_press(move || {
            if let Some(row) = message() {
                actions.show_user(row.sender_id);
            }
        })
        .child(ui::avatar_view(
            move || {
                message()
                    .map(|row| row.sender_name.into())
                    .unwrap_or_default()
            },
            move || {
                message()
                    .map(|row| row.sender_avatar_url)
                    .unwrap_or_default()
            },
            ui::AvatarSize::Compact,
        ))
}
