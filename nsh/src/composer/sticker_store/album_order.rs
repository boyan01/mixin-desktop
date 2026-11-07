use nsh_base::theme;
use nsh_core::{PointerEvents, TouchEvent, TouchPhase, prelude::*};
use std::{sync::Arc, time::Duration};

use super::{State, tr};
use crate::{i18n, motion, ui};

const ROW_HEIGHT: f32 = 36.0;

#[derive(Clone, PartialEq)]
struct Drag {
    order: Arc<[String]>,
    name: String,
    from: usize,
    target: usize,
    start_scroll: f32,
    delta: f32,
    active: bool,
}

impl Drag {
    fn top(&self) -> f32 {
        self.from as f32 * ROW_HEIGHT - self.start_scroll + self.delta
    }

    fn update_target(&mut self, offset: f32) {
        self.target = (self.from as f32 + (self.delta + offset - self.start_scroll) / ROW_HEIGHT)
            .round()
            .clamp(0.0, self.order.len().saturating_sub(1) as f32) as usize;
    }

    fn marker(&self, offset: f32) -> f32 {
        (self.target + usize::from(self.target > self.from)) as f32 * ROW_HEIGHT - offset
    }
}

fn order(state: State) -> Vec<String> {
    state
        .albums
        .with_untracked(|rows| rows.iter().map(|row| row.album_id.clone()).collect())
}

fn save(state: State, ids: Vec<String>) {
    let client = state.client();
    state.update(async move { client.sticker().set_sticker_album_order(ids).await });
}

pub(super) fn list(state: State) -> impl IntoElement {
    let i18n = i18n::current();
    let drag = signal(None::<Drag>);
    let candidate = stored(None::<(f64, String)>);
    let serial = stored(0_u64);
    let offset = signal(0.0_f32);
    let viewport = signal((0.0_f32, 0.0_f32));
    let list_ref = element_ref();
    // This is a gesture draft, never another source of persisted album order.
    effect(move || {
        let _ = (
            state.albums.get(),
            state.busy.get(),
            window().width,
            window().height,
        );
        drag.set(None);
        candidate.replace(None);
    });
    let edge = computed(move || {
        drag.with(|value| match value.as_ref().filter(|value| value.active) {
            Some(value) if value.top() < 12.0 => -1_i8,
            Some(value) if value.top() + ROW_HEIGHT > viewport.get().1 - 12.0 => 1,
            _ => 0,
        })
    });
    effect(move || {
        let direction = edge.get();
        if direction == 0 {
            return;
        }
        let gesture = serial.get();
        // A timer only exists at an active drag edge and ends with this owner.
        spawn(async move {
            loop {
                timer(Duration::from_millis(50)).await;
                if serial.get() != gesture
                    || edge.get_untracked() != direction
                    || drag.with_untracked(|value| value.as_ref().is_none_or(|value| !value.active))
                {
                    break;
                }
                let maximum = (state.albums.with_untracked(Vec::len) as f32 * ROW_HEIGHT
                    - viewport.get_untracked().1)
                    .max(0.0);
                let next =
                    (offset.get_untracked() + f32::from(direction) * 8.0).clamp(0.0, maximum);
                if (next - offset.get_untracked()).abs() <= 0.5 {
                    break;
                }
                list_ref.scroll_to_offset(px(next), false);
            }
        });
    });
    view()
        .h_20()
        .min_h_0()
        .overflow_hidden()
        .on_layout(move |bounds: &Bounds<Points>| {
            let size = (bounds.size.width.0, bounds.size.height.0);
            if viewport.get_untracked() != size {
                drag.set(None);
                viewport.set(size);
            }
        })
        .claim_touches(
            TouchClaim::on_start()
                .capture()
                .ask(move |request: &ClaimRequest| {
                    if !state.busy.get_untracked()
                        && request.event.gesture.active_touches == 1
                        && state.albums.with_untracked(Vec::len) > 1
                        && candidate.with(|value| {
                            value
                                .as_ref()
                                .is_some_and(|(time, _)| *time == request.event.timestamp)
                        })
                    {
                        request.claim();
                    }
                }),
        )
        .keep_touches(true)
        .on_responder_grant(move |_: &ResponderEvent| {
            let Some((_, id)) = candidate.replace(None) else {
                return;
            };
            if state.busy.get_untracked() {
                return;
            }
            let rows = state.albums.get_untracked();
            let Some(from) = rows.iter().position(|row| row.album_id == id) else {
                return;
            };
            serial.replace(serial.get().wrapping_add(1));
            drag.set(Some(Drag {
                name: rows[from].name.clone(),
                order: rows.into_iter().map(|row| row.album_id).collect(),
                from,
                target: from,
                start_scroll: offset.get_untracked(),
                delta: 0.0,
                active: false,
            }));
        })
        .on_responder_start(move |event: &ResponderEvent| {
            if event.gesture.active_touches != 1 {
                drag.set(None);
            }
        })
        .on_responder_move(move |event: &ResponderEvent| {
            let Some(mut value) = drag.get_untracked() else {
                return;
            };
            value.delta = event.gesture.delta.y.0;
            value.active |= value.delta.abs() > 4.0;
            value.update_target(offset.get_untracked());
            drag.set(Some(value));
        })
        .on_responder_release(move |_: &ResponderEvent| {
            let Some(value) = drag.get_untracked() else {
                return;
            };
            drag.set(None);
            if state.busy.get_untracked()
                || !value.active
                || value.from == value.target
                || value.order.as_ref() != order(state).as_slice()
            {
                return;
            }
            let mut ids = value.order.to_vec();
            let id = ids.remove(value.from);
            ids.insert(value.target, id);
            save(state, ids);
        })
        .on_responder_terminate(move |_: &ResponderEvent| drag.set(None))
        .child(when(
            move || drag.with(Option::is_some),
            move || {
                on_key("escape", move |_| drag.set(None));
                view().hidden()
            },
        ))
        .child(
            virtual_list(
                move || state.albums.with(Vec::len),
                move |at| {
                    let id = move || {
                        state
                            .albums
                            .with(|rows| rows.get(at.index()).map(|row| row.album_id.clone()))
                    };
                    view()
                        .h(px(ROW_HEIGHT))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .opacity(move || {
                            if drag.with(|value| {
                                value.as_ref().is_some_and(|value| {
                                    value.active && Some(value.order[value.from].clone()) == id()
                                })
                            }) {
                                0.3_f32
                            } else {
                                1.0_f32
                            }
                        })
                        .transition_opacity()
                        .duration(|| motion::duration(motion::FEEDBACK_MS))
                        .ease_out()
                        .child(
                            view()
                                .size_8()
                                .items_center()
                                .justify_center()
                                .on_touch(move |event: &TouchEvent| {
                                    if event.phase == TouchPhase::Start
                                        && let Some(id) = id()
                                    {
                                        candidate.replace(Some((event.timestamp, id)));
                                    }
                                })
                                .accessibility_elements_hidden(true)
                                .child(
                                    ui::icon(ui::Icon::Menu)
                                        .size_4()
                                        .text_color(move || theme().muted_foreground),
                                ),
                        )
                        .child(
                            text(move || {
                                state.albums.with(|rows| {
                                    rows.get(at.index())
                                        .map(|row| row.name.clone())
                                        .unwrap_or_default()
                                })
                            })
                            .text_sm()
                            .number_of_lines(1)
                            .flex_1(),
                        )
                        .children([-1_isize, 1].into_iter().map(move |direction| {
                            ui::command()
                                .size_8()
                                .px_1()
                                .py_1()
                                .accessibility_label(move || {
                                    Some(
                                        tr(
                                            i18n,
                                            if direction < 0 {
                                                "Move up"
                                            } else {
                                                "Move down"
                                            },
                                            if direction < 0 { "上移" } else { "下移" },
                                        )
                                        .into(),
                                    )
                                })
                                .disabled(move || {
                                    state.busy.get()
                                        || drag.with(Option::is_some)
                                        || (direction < 0 && at.index() == 0)
                                        || (direction > 0
                                            && at.index() + 1 >= state.albums.with(Vec::len))
                                })
                                .child(text(if direction < 0 { "↑" } else { "↓" }))
                                .on_press(move || {
                                    let mut ids = order(state);
                                    let from = at.index();
                                    let to = (from as isize + direction) as usize;
                                    if from >= ids.len() || to >= ids.len() {
                                        return;
                                    }
                                    ids.swap(from, to);
                                    save(state, ids);
                                })
                        }))
                },
            )
            .keys(move || {
                state.albums.with(|rows| {
                    rows.iter()
                        .map(|row| row.album_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .estimated_item_size(px(ROW_HEIGHT))
            .flex_1()
            .min_h_0()
            .element_ref(&list_ref)
            .on_scroll(move |event: &ScrollEvent| {
                offset.set(event.offset.y.0);
                if let Some(mut value) = drag.get_untracked() {
                    value.update_target(event.offset.y.0);
                    drag.set(Some(value));
                }
            }),
        )
        .child(when(
            move || drag.with(|value| value.as_ref().is_some_and(|value| value.active)),
            move || {
                view()
                    .absolute()
                    .inset_0()
                    .pointer_events(PointerEvents::None)
                    .accessibility_elements_hidden(true)
                    .child(
                        view()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h(px(ROW_HEIGHT))
                            .px_2()
                            .justify_center()
                            .rounded_md()
                            .bg(move || theme().popover)
                            .border_1()
                            .border_color(move || theme().border)
                            .translate_y(move || {
                                px(drag.with(|value| value.as_ref().map_or(0.0, Drag::top)))
                            })
                            .child(
                                text(move || {
                                    drag.with(|value| {
                                        value
                                            .as_ref()
                                            .map(|value| value.name.clone())
                                            .unwrap_or_default()
                                    })
                                })
                                .text_sm()
                                .number_of_lines(1),
                            ),
                    )
                    .child(when(
                        move || {
                            drag.with(|value| {
                                value
                                    .as_ref()
                                    .is_some_and(|value| value.from != value.target)
                            })
                        },
                        move || {
                            view()
                                .absolute()
                                .top_0()
                                .left_2()
                                .right_2()
                                .h(px(2.0))
                                .bg(move || theme().primary)
                                .translate_y(move || {
                                    px(drag.with(|value| {
                                        value.as_ref().map_or(0.0, |value| {
                                            (value.marker(offset.get()) - 1.0).max(0.0)
                                        })
                                    }))
                                })
                        },
                    ))
            },
        ))
}
