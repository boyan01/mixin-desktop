//! One timeline row: day and unread separators, then a notice or a bubble.
use mixin_desktop_api::MessageListView;
use nsh_core::prelude::*;

use super::{
    message_actions::{self, Actions},
    message_appearance::{self, Frame, Inset, Stamp, bubble_surface},
    message_content::{self, Placement},
    message_system,
    timeline::Timeline,
    unread::Unread,
};
use crate::{appearance::Appearance, presentation, ui};

/// What a row shows, derived from the window at its index.
#[derive(Clone, PartialEq)]
struct Line {
    message: MessageListView,
    mine: bool,
    day: Option<String>,
    /// The sender's name and avatar start here.
    first_of_sender: bool,
    /// Continues the sender's messages: Flutter's `sameUserPrev`.
    follows_sender: bool,
    /// The newest message of the sender's run, which carries the nip.
    last_of_sender: bool,
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
            let follows_sender = older.is_some_and(|older| {
                older.sender_id == row.sender_id
                    && !older.category.starts_with("SYSTEM_")
                    && older.category != "MESSAGE_PIN"
            });
            let continued = index
                .checked_sub(1)
                .and_then(|newer| rows.get(newer))
                .is_some_and(|newer| {
                    newer.sender_id == row.sender_id
                        && presentation::day(i18n, newer.created_at_micros) == day
                });
            Some(Line {
                message: row.clone(),
                mine: self.own_id.with(|id| row.sender_id == *id),
                first_of_sender: new_day || !follows_sender,
                follows_sender,
                last_of_sender: !continued,
                day: new_day.then_some(day),
            })
        })
    }

    pub fn row(self, at: Position) -> impl IntoElement {
        let Self {
            actions,
            target,
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
        let follows_sender =
            move || line.with(|line| line.as_ref().is_some_and(|line| line.follows_sender));
        view()
            .element_ref(&row_ref)
            .rounded(ui::radius_md())
            .style_when(move || !follows_sender(), |style| style.pt_2())
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
                move || self.message_line(line, message, content, status),
            ))
    }

    fn message_line(
        self,
        line: Signal<Option<Line>>,
        message: impl Fn() -> Option<MessageListView> + Copy + 'static,
        content: Memo<Option<MessageListView>>,
        status: Memo<String>,
    ) -> impl IntoElement {
        let Self {
            actions,
            target,
            timeline,
            sender_names,
            ..
        } = self;
        let appearance = use_context::<Appearance>().expect("Home provides chat appearance");
        let mine = move || line.with(|line| line.as_ref().is_some_and(|line| line.mine));
        let others = move || sender_names.get() && !mine();
        let first_of_sender =
            move || line.with(|line| line.as_ref().is_some_and(|line| line.first_of_sender));
        let last_of_sender =
            move || line.with(|line| line.as_ref().is_some_and(|line| line.last_of_sender));
        let frame =
            computed(move || content.with(|row| row.as_ref().map_or(Frame::TEXT, Frame::of)));
        let surface = move || frame.get().surface;
        let captioned = move || frame.get().inset == Inset::Caption;
        let neutral = computed(move || frame.get().neutral);
        let avatar = move || others() && appearance.show_avatar();
        // Flutter leaves the nip to own bubbles where incoming ones have avatars.
        let nip = move || {
            surface()
                && last_of_sender()
                && (mine() || !(sender_names.get() && appearance.show_avatar()))
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
                        framed: true,
                    },
                ),
                None => view().hidden().into_element(),
            })
        });
        let own_status = move || (mine() && frame.get().delivery).then_some(status);
        let stamp = move |placed: Stamp| {
            dynamic(move || {
                let row = content.get();
                untracked(|| match row {
                    Some(row) if frame.get().stamp == placed => {
                        message_appearance::stamp(&row, own_status(), false).into_element()
                    }
                    _ => view().hidden().into_element(),
                })
            })
        };
        let edge = ui::space_lg() + ui::bubble_nip_width();
        view()
            .flex_row()
            .items_start()
            .child(when(
                move || !actions.selected.with(Vec::is_empty),
                move || selection_box(message, actions),
            ))
            .child(when(avatar, move || {
                view()
                    .ml_2()
                    .size_8()
                    .flex_shrink_0()
                    .child(when(first_of_sender, move || {
                        sender_avatar(message, actions)
                    }))
            }))
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .py(ui::space_2xs())
                    .pl(edge)
                    .pr(ui::message_far_edge())
                    .items_start()
                    .style_when(avatar, |style| style.pl(ui::bubble_nip_width()))
                    .style_when(mine, |style| {
                        style.pl(ui::message_far_edge()).pr(edge).items_end()
                    })
                    .child(when(
                        move || others() && first_of_sender(),
                        move || message_appearance::sender(message, actions),
                    ))
                    .child(
                        view()
                            .min_w_0()
                            .max_w(relative(1.0))
                            .items_end()
                            .child(
                                view()
                                    .relative()
                                    .min_w_0()
                                    .max_w(relative(1.0))
                                    .child(when(nip, move || {
                                        message_appearance::nip(mine, neutral)
                                    }))
                                    .child(
                                        view()
                                            .relative()
                                            .min_w_0()
                                            .rounded(ui::radius_md())
                                            .style_when(surface, move |style| {
                                                style
                                                    .bg(move || {
                                                        bubble_surface(mine() && !neutral.get())
                                                    })
                                                    .shadow_xs()
                                                    .shadow_color(ui::bubble_shadow())
                                            })
                                            .child(
                                                view()
                                                    .min_w_0()
                                                    .rounded(ui::radius_md())
                                                    .overflow_hidden()
                                                    .child(message_actions::menu(
                                                        message, actions, body,
                                                    )),
                                            )
                                            .child(when(highlighted, move || {
                                                view()
                                                    .absolute()
                                                    .inset_0()
                                                    .rounded(ui::radius_md())
                                                    .bg(move || {
                                                        if !surface() {
                                                            ui::bubble_highlight_media()
                                                        } else if mine() {
                                                            ui::bubble_highlight_out()
                                                        } else {
                                                            ui::bubble_highlight_in()
                                                        }
                                                    })
                                                    .bind_opacity(&timeline.highlight_opacity)
                                                    .pointer_events(PointerEvents::None)
                                            }))
                                            .child(when(
                                                move || frame.get().stamp == Stamp::Inline,
                                                move || {
                                                    view()
                                                        .absolute()
                                                        .right(ui::space_sm())
                                                        .bottom(ui::space_sm())
                                                        .style_when(captioned, |style| {
                                                            style
                                                                .right(ui::space_md())
                                                                .bottom(ui::chip_inset())
                                                        })
                                                        .pointer_events(PointerEvents::None)
                                                        .child(stamp(Stamp::Inline))
                                                },
                                            )),
                                    ),
                            )
                            .child(when(
                                move || frame.get().stamp == Stamp::Below,
                                move || {
                                    // Flutter's 10 from the bubble's box, whose end
                                    // holds the nip gutter of an own message.
                                    view()
                                        .mt(ui::space_xs())
                                        .ml(ui::chip_inset())
                                        .mr(ui::chip_inset())
                                        .style_when(mine, |style| {
                                            style.mr(ui::chip_inset() - ui::bubble_nip_width())
                                        })
                                        .child(stamp(Stamp::Below))
                                },
                            )),
                    ),
            )
    }
}

fn day_separator(day: Memo<Option<String>>) -> impl IntoElement {
    let appearance = use_context::<Appearance>().expect("Home provides chat appearance");
    view()
        .items_center()
        .pt(ui::space_lg())
        .pb(ui::chip_inset())
        .child(
            view()
                .px(ui::chip_inset())
                .py(ui::space_2xs())
                .min_w(ui::day_chip_min_width())
                .items_center()
                .rounded(ui::radius_chip())
                .bg(ui::day_background)
                .child(
                    text(move || day.get().unwrap_or_default())
                        .text_size(move || appearance.secondary_size())
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
            move || message().map(|row| row.sender_id).unwrap_or_default(),
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
