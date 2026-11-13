use super::message_actions::{Actions, remove};
use crate::{
    i18n::Key,
    ui::{self, Typography},
};
use nsh_core::prelude::*;

mod copy;

/// One message-copy command shared by the editor fallback and non-editing keys.
pub(super) fn keyboard(actions: Actions, conversation: Stored<String>, enabled: Memo<bool>) {
    let window = nsh_core::element::desktop::current_window();
    let enabled = computed(move || {
        enabled.get()
            && window
                .state()
                .is_none_or(|state| state.focused && state.visible)
    });
    let revision = signal(0_u64);
    effect(move || {
        if !enabled.get() {
            revision.update(|value| *value = value.wrapping_add(1));
        }
    });
    actions.text_selection.enable(enabled);
    let copy = move || {
        if !actions.owner.is_live() || !enabled.get_untracked() {
            return;
        }
        if let Some(text) = actions.text_selection.keyboard() {
            set_clipboard(text);
        } else {
            let ticket = revision.get_untracked();
            let selected = actions.selected.get_untracked();
            copy::selected_if(actions, conversation.get(), move || {
                enabled.get_untracked()
                    && revision.get_untracked() == ticket
                    && actions.selected.get_untracked() == selected
            });
        }
    };
    provide_context(CopyFallback(std::rc::Rc::new(copy)));
    effect(move || {
        if enabled.get() {
            on_key_with(
                "mod+c",
                KeyOptions {
                    when: nsh_core::keymap::When::NotEditing,
                    ..Default::default()
                },
                move |_| copy(),
            );
        }
    });
}

#[derive(Clone)]
pub(super) struct CopyFallback(pub std::rc::Rc<dyn Fn()>);

pub(super) fn selection_bar(actions: Actions, conversation_id: Stored<String>) -> impl IntoElement {
    selection(actions, conversation_id, false)
}

pub(super) fn pinned_selection_bar(
    actions: Actions,
    conversation_id: Stored<String>,
) -> impl IntoElement {
    selection(actions, conversation_id, true)
}

fn selection(
    actions: Actions,
    conversation_id: Stored<String>,
    pinned_page: bool,
) -> impl IntoElement {
    let i18n = actions.i18n;
    when(
        move || !actions.selected.with(Vec::is_empty),
        move || {
            on_back_press(move || {
                if actions.busy.get_untracked() || actions.selected.with_untracked(Vec::is_empty) {
                    return false;
                }
                actions.selected.set(Vec::new());
                true
            });
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .bg(ui::surface_list)
                .child(
                    text(move || {
                        let count = actions.selected.with(Vec::len);
                        i18n.format(
                            "{count} selected",
                            "已选择 {count} 条消息",
                            &[("count", &count.to_string())],
                        )
                    })
                    .text_role(ui::TextRole::Body)
                    .flex_none()
                    .number_of_lines(1),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Forward…", "转发…"),
                        move || {
                            let ids = actions.selected.with(|ids| ids.to_vec());
                            actions.combine.set(false);
                            actions.forwarding.set(ids);
                        },
                    )
                    .disabled(move || {
                        actions.busy.get()
                            || actions.selected.with(Vec::len) >= 100
                            || actions.selected.with(|ids| {
                                ids.iter().any(|id| {
                                    !actions
                                        .policies
                                        .with(|p| p.get(id).is_some_and(|p| p.can_forward))
                                })
                            })
                    }),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Copy", "复制"),
                        move || copy::selected(actions, conversation_id.with(|id| id.clone())),
                    )
                    .disabled(actions.busy),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Combine and forward…", "合并转发…"),
                        move || {
                            actions.combine.set(true);
                            actions.forwarding.set(actions.selected.get_untracked());
                        },
                    )
                    .disabled(move || {
                        actions.busy.get()
                            || !(2..100).contains(&actions.selected.with(Vec::len))
                            || actions.selected.with(|ids| {
                                ids.iter().any(|id| {
                                    !actions
                                        .policies
                                        .with(|p| p.get(id).is_some_and(|p| p.can_combine_forward))
                                })
                            })
                    }),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Delete for everyone…", "为所有人删除…"),
                        move || {
                            remove(
                                actions,
                                conversation_id.with(|id| id.clone()),
                                actions.selected.get_untracked(),
                                true,
                            );
                        },
                    )
                    .disabled(move || {
                        actions.busy.get()
                            || actions.selected.with(Vec::len) >= 100
                            || actions.selected.with(|ids| {
                                ids.iter().any(|id| {
                                    !actions
                                        .policies
                                        .with(|p| p.get(id).is_some_and(|p| p.can_recall))
                                })
                            })
                    }),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Delete…", "删除…"),
                        move || {
                            let ids = actions.selected.with(|ids| ids.to_vec());
                            remove(
                                actions,
                                conversation_id.with(|value| value.clone()),
                                ids,
                                false,
                            );
                        },
                    )
                    .disabled(actions.busy)
                    .style_when(move || pinned_page, |style| style.hidden()),
                )
                .child(
                    ui::action(
                        move || i18n.text(Key::Cancel),
                        move || actions.selected.set(Vec::new()),
                    )
                    .disabled(actions.busy),
                )
        },
    )
}
