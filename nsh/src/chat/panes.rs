//! The detail pane's open conversation.
use std::{rc::Rc, time::Duration};

use mixin_desktop_api::ConversationListData;
use nsh_core::{PointerEvents, prelude::*};

use crate::ui;

/// Says that a chat has its first rows in place.
pub(crate) type Ready = Rc<dyn Fn()>;

/// How long a switch waits for the new chat before showing it unready.
const READY_BOUND: Duration = Duration::from_millis(400);

/// Mounts the selected conversation with `render`, or `empty` when none is.
///
/// A newly selected chat mounts invisibly over the shown one and replaces it in
/// one frame once it calls its `Ready`, so a switch never shows a chat whose rows
/// are still being read. A chat that is not ready within `READY_BOUND` shows
/// anyway, with its own loading state.
pub(crate) fn panes<E: IntoElement + 'static>(
    selected: Signal<Option<ConversationListData>>,
    selected_id: Memo<Option<String>>,
    empty: impl Fn() -> AnyElement + 'static,
    render: impl Fn(ConversationListData, Ready) -> E + Copy + 'static,
) -> impl IntoElement {
    // The shown chat, then the one waiting to replace it, if any.
    let mounted = signal(Vec::<String>::new());
    let shown = signal(None::<String>);
    let show = move |id: &str| {
        if selected_id.with_untracked(|selected| selected.as_deref() == Some(id))
            && mounted.with_untracked(|ids| ids.iter().any(|mounted| mounted == id))
        {
            batch(|| {
                shown.set(Some(id.to_owned()));
                mounted.set(vec![id.to_owned()]);
            });
        }
    };
    effect(move || {
        let id = selected_id.get();
        untracked(|| match id {
            None => batch(|| {
                shown.set(None);
                mounted.set(Vec::new());
            }),
            // Nothing to hold, or the shown chat again: show it as it is.
            Some(id) if shown.with(|shown| shown.is_none() || shown.as_ref() == Some(&id)) => {
                batch(|| {
                    shown.set(Some(id.clone()));
                    mounted.set(vec![id]);
                })
            }
            Some(id) => {
                let mut ids = shown.get().into_iter().collect::<Vec<_>>();
                ids.push(id.clone());
                mounted.set(ids);
                set_timeout(READY_BOUND, move || show(&id));
            }
        })
    });
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .relative()
        .child(when(move || selected_id.with(Option::is_none), empty))
        .child(each(
            move || mounted.get(),
            |id: &String| id.clone(),
            move |id, _| {
                let id = id.get_untracked();
                let visible = {
                    let id = id.clone();
                    computed(move || shown.with(|shown| shown.as_deref() == Some(id.as_str())))
                };
                let conversation = selected
                    .with_untracked(|row| row.clone())
                    .filter(|row| row.conversation_id == id);
                let ready: Ready = {
                    let id = id.clone();
                    Rc::new(move || show(&id))
                };
                view()
                    .absolute()
                    .inset_0()
                    .flex_row()
                    // A waiting chat lays out at full size, so it is placed when shown.
                    .style_when(move || !visible.get(), |style| style.opacity(0.0))
                    .accessibility_elements_hidden(move || !visible.get())
                    .pointer_events(move || {
                        if visible.get() {
                            PointerEvents::Auto
                        } else {
                            PointerEvents::None
                        }
                    })
                    .child(match conversation {
                        Some(conversation) => {
                            ui::owned(id, move || render(conversation.clone(), ready.clone()))
                                .into_element()
                        }
                        None => view().into_element(),
                    })
            },
        ))
}
