use mixin_desktop_api::{CircleItem, ClientResult, ConversationListData};
use nsh_base::alert_dialog;
use nsh_core::{TouchEvent, TouchPhase, element::desktop::MenuItem};
use std::{future::Future, time::Duration};

use super::*;
use crate::{conversation::parts, i18n::tr, runtime};

/// A rail row and the rail's row gap, so circles keep the category rhythm.
fn circle_row_height() -> Points {
    ui::nav_row_height() + ui::space_xs()
}

#[derive(Clone, PartialEq)]
struct CircleDrag {
    circle: CircleItem,
    order: Arc<[String]>,
    from: usize,
    target: usize,
    start_scroll: f32,
    delta: f32,
    started: bool,
}

impl CircleDrag {
    fn active(&self) -> bool {
        self.started
    }

    fn top(&self) -> f32 {
        self.from as f32 * circle_row_height().0 - self.start_scroll + self.delta
    }

    fn update_target(&mut self, offset: f32) {
        let displacement = self.delta + offset - self.start_scroll;
        self.target = (self.from as f32 + displacement / circle_row_height().0)
            .round()
            .clamp(0.0, self.order.len().saturating_sub(1) as f32) as usize;
    }

    fn marker(&self, offset: f32) -> f32 {
        let boundary = self.target + usize::from(self.target > self.from);
        boundary as f32 * circle_row_height().0 - offset
    }
}

#[derive(Clone)]
enum Editing {
    Rename(CircleItem),
    Manage(CircleItem),
    Membership(Box<ConversationListData>),
}

#[derive(Clone, Copy)]
pub(crate) struct Circles {
    owner: Owner,
    account: Stored<Arc<AccountClient>>,
    rows: Signal<Vec<CircleItem>>,
    editing: Signal<Option<Editing>>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    nav: Navigation,
}

impl Circles {
    pub(crate) fn new(account: Arc<AccountClient>, nav: Navigation) -> Self {
        let state = Self {
            owner: Owner::current(),
            account: stored(account.clone()),
            rows: signal(Vec::new()),
            editing: signal_with(None, SignalEquals::Never),
            busy: signal(false),
            failed: signal(false),
            nav,
        };
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(account.circle_changes()));
            while let Some(rows) = changes.next().await {
                if let Some(selected) = nav.circle.get_untracked()
                    && !rows.iter().any(|circle| circle.circle_id == selected)
                {
                    nav.select("chats", None);
                }
                let removed = state.editing.with_untracked(|editing| match editing {
                    Some(Editing::Rename(circle) | Editing::Manage(circle)) => {
                        !rows.iter().any(|row| row.circle_id == circle.circle_id)
                    }
                    _ => false,
                });
                if removed {
                    state.editing.set(None);
                }
                state.rows.set(rows);
            }
        });
        state
    }

    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    fn execute(self, command: impl Future<Output = ClientResult<()>> + Send + 'static) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        let command = runtime::commit(command);
        // Not the caller's owner: deleting a circle disposes the row that asked.
        self.owner.run(|| {
            spawn(async move {
                let result = command.await;
                self.busy.set(false);
                self.failed.set(result.is_err());
                if result.is_ok() {
                    self.editing.set(None);
                }
            });
        });
    }

    fn move_circle(self, id: String, up: bool) {
        let mut ids = self.rows.with_untracked(|rows| {
            rows.iter()
                .map(|circle| circle.circle_id.clone())
                .collect::<Vec<_>>()
        });
        let Some(index) = ids.iter().position(|circle| circle == &id) else {
            return;
        };
        let target = if up {
            index.checked_sub(1)
        } else {
            (index + 1 < ids.len()).then_some(index + 1)
        };
        let Some(target) = target else {
            return;
        };
        ids.swap(index, target);
        let account = self.client();
        self.execute(async move { account.conversation().reorder_circles(ids).await });
    }

    pub(super) fn list(
        self,
        counts: Signal<Vec<ConversationUnseenCount>>,
        labels: super::Labels,
    ) -> impl IntoElement {
        let nav = self.nav;
        let reorder = Reorder {
            circles: self,
            drag: signal(None),
            candidate: stored(None),
            offset: signal(0.0),
            viewport: signal((0.0, 0.0)),
            list: element_ref(),
        };
        let Reorder {
            drag,
            candidate,
            offset,
            viewport,
            list,
            ..
        } = reorder;
        // Any authoritative change invalidates the order captured at pointer-down.
        effect(move || {
            let _ = (
                self.rows.get(),
                self.busy.get(),
                window().width,
                window().height,
                nav.compact(),
            );
            drag.set(None);
        });
        reorder.scroll_at_edges();
        view()
            .flex_1()
            .min_h_0()
            .child(when(move || !self.rows.with(Vec::is_empty), ui::divider))
            .child(
                view()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .on_layout(move |bounds: &Bounds<Points>| {
                        let size = (bounds.size.width.0, bounds.size.height.0);
                        if viewport.get_untracked() != size {
                            drag.set(None);
                            viewport.set(size);
                        }
                    })
                    .claim_touches(TouchClaim::on_start().capture().ask(
                        move |request: &ClaimRequest| {
                            let pressed = candidate.with(|candidate| {
                                candidate
                                    .as_ref()
                                    .is_some_and(|(time, _)| *time == request.event.timestamp)
                            });
                            if !self.busy.get_untracked()
                                && request.event.gesture.active_touches == 1
                                && self.rows.with_untracked(Vec::len) > 1
                                && pressed
                            {
                                request.claim();
                            }
                        },
                    ))
                    .keep_touches(true)
                    .on_responder_grant(move |_: &ResponderEvent| reorder.begin())
                    .on_responder_start(move |event: &ResponderEvent| {
                        if event.gesture.active_touches != 1 {
                            drag.set(None);
                        }
                    })
                    .on_responder_move(move |event: &ResponderEvent| {
                        let Some(mut current) = drag.get_untracked() else {
                            return;
                        };
                        current.delta = event.gesture.delta.y.0;
                        current.started |= current.delta.abs() > 4.0;
                        current.update_target(offset.get_untracked());
                        drag.set(Some(current));
                    })
                    .on_responder_release(move |_: &ResponderEvent| reorder.finish())
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
                            move || self.rows.with(Vec::len),
                            move |at| {
                                reorder.row(
                                    counts,
                                    labels,
                                    computed(move || {
                                        self.rows.with(|rows| rows.get(at.index()).cloned())
                                    }),
                                )
                            },
                        )
                        .keys(move || self.ids())
                        .estimated_item_size(circle_row_height())
                        .flex_1()
                        .min_h_0()
                        .element_ref(&list)
                        .on_scroll(move |event: &ScrollEvent| {
                            offset.set(event.offset.y.0);
                            if let Some(mut current) = drag.get_untracked() {
                                current.update_target(event.offset.y.0);
                                drag.set(Some(current));
                            }
                        }),
                    )
                    .child(when(
                        move || drag.with(|drag| drag.as_ref().is_some_and(CircleDrag::active)),
                        move || reorder.preview(labels),
                    )),
            )
    }

    fn ids(self) -> Vec<String> {
        self.rows.with(|rows| {
            rows.iter()
                .map(|circle| circle.circle_id.clone())
                .collect::<Vec<_>>()
        })
    }

    fn confirm_delete(self, circle: CircleItem) {
        self.owner.run(|| {
            spawn(async move {
                let title = i18n::current().format(
                    "Delete “{name}”?",
                    "删除“{name}”？",
                    &[("name", &circle.name)],
                );
                let answer = alert_dialog()
                    .title(title)
                    .description(tr(
                        "Chats in this circle will remain available in All chats.",
                        "圈子中的聊天仍会保留在“全部聊天”中。",
                    ))
                    .cancel("cancel", tr("Cancel", "取消"))
                    .destructive("delete", tr("Delete", "删除"))
                    .show()
                    .await;
                if matches!(answer, Ok(answer) if answer.action.as_deref() == Some("delete")) {
                    let account = self.client();
                    self.execute(async move {
                        account.conversation().delete_circle(circle.circle_id).await
                    });
                }
            });
        });
    }

    pub(super) fn feedback(self) -> impl IntoElement {
        view().child(when(self.failed, move || {
            view().px_2().py_2().child(
                ui::error_text(move || {
                    tr(
                        "Couldn’t update circles. Try again.",
                        "更新圈子失败，请重试。",
                    )
                })
                .text_role(ui::TextRole::Caption),
            )
        }))
    }

    pub(super) fn surface(self) -> impl IntoElement {
        let shown = signal_with(None::<Editing>, SignalEquals::Never);
        effect(move || {
            if let Some(editing) = self.editing.get() {
                shown.set(Some(editing));
            }
        });
        let presence = crate::motion::Presence::modal(move || self.editing.with(Option::is_some));
        view().child(when(
            move || presence.mounted(),
            move || {
                cleanup(move || {
                    let _ = shown.try_set(None);
                });
                dynamic(move || {
                    let editing = shown.get();
                    untracked(|| match editing {
                        Some(Editing::Rename(circle)) => {
                            self.rename(circle, presence).into_element()
                        }
                        Some(Editing::Manage(circle)) => super::circle_editor::screen(
                            self.client(),
                            circle,
                            presence,
                            move || self.editing.set(None),
                        )
                        .into_element(),
                        Some(Editing::Membership(conversation)) => {
                            self.membership(*conversation, presence).into_element()
                        }
                        None => view().hidden().into_element(),
                    })
                })
            },
        ))
    }

    fn rename(self, circle: CircleItem, presence: crate::motion::Presence) -> impl IntoElement {
        let name = signal(circle.name);
        let id = stored(circle.circle_id);
        ui::modal(
            presence,
            move || {
                if !self.busy.get_untracked() {
                    self.editing.set(None);
                }
            },
            ui::modal_card()
                .child(ui::modal_title(move || tr("Rename circle", "重命名圈子")))
                .child(when(self.failed, || {
                    ui::error_text(move || tr("Couldn’t save. Try again.", "保存失败，请重试。"))
                }))
                .child(
                    ui::field()
                        .label(move || tr("Name", "名称"))
                        .max_length(64)
                        .value(move || name.get())
                        .on_change(move |value: &str| name.set(value.into())),
                )
                .child(
                    view()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(
                            ui::action(
                                move || tr("Cancel", "取消"),
                                move || self.editing.set(None),
                            )
                            .disabled(self.busy),
                        )
                        .child(
                            ui::primary()
                                .disabled(move || {
                                    self.busy.get() || name.with(|name| name.trim().is_empty())
                                })
                                .on_press(move || {
                                    if !presence.interactive() {
                                        return;
                                    }
                                    let account = self.client();
                                    let id = id.with(|id| id.clone());
                                    let name = name.get_untracked().trim().to_owned();
                                    self.execute(async move {
                                        account.conversation().update_circle(id, name).await
                                    });
                                })
                                .child(text(move || tr("Save", "保存"))),
                        ),
                ),
        )
    }

    fn membership(
        self,
        conversation: ConversationListData,
        presence: crate::motion::Presence,
    ) -> impl IntoElement {
        let id = stored(conversation.conversation_id);
        let owner = stored(conversation.owner_id);
        let group = conversation.category == "GROUP";
        let name = stored(conversation.name);
        let current = signal(conversation.circle_ids);
        let account = self.client();
        let query_id = id.with(|id| id.clone());
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
            loop {
                let account = self.client();
                let requested_id = query_id.clone();
                if let Ok(rows) = runtime::run(async move {
                    account
                        .conversation()
                        .conversation_items_by_ids(vec![requested_id])
                        .await
                })
                .await
                    && let Some(row) = rows.into_iter().next()
                {
                    current.set(row.circle_ids);
                }
                loop {
                    match changes.next().await {
                        Some(event)
                            if event.reload_all || event.conversation_ids.contains(&query_id) =>
                        {
                            break;
                        }
                        Some(_) => {}
                        None => return,
                    }
                }
            }
        });
        ui::modal(
            presence,
            move || {
                if !self.busy.get_untracked() {
                    self.editing.set(None);
                }
            },
            ui::modal_card()
                .overflow_hidden()
                .child(ui::modal_title(move || tr("Circles", "圈子")))
                .child(
                    text(move || name.with(|name| name.clone()))
                        .flex_none()
                        .text_role(ui::TextRole::Body)
                        .number_of_lines(1),
                )
                .child(when(self.failed, || {
                    ui::error_text(move || tr("Couldn’t update. Try again.", "更新失败，请重试。"))
                        .flex_none()
                }))
                .child(
                    virtual_list(
                        move || self.rows.with(Vec::len),
                        move |at| {
                            let circle =
                                move || self.rows.with(|rows| rows.get(at.index()).cloned());
                            let checked = move || {
                                circle().is_some_and(|circle| {
                                    current.with(|ids| ids.contains(&circle.circle_id))
                                })
                            };
                            parts::compact_row()
                                .selected(checked)
                                .disabled(self.busy)
                                .child(
                                    text(move || {
                                        circle().map(|circle| circle.name).unwrap_or_default()
                                    })
                                    .text_role(ui::TextRole::Label)
                                    .flex_1(),
                                )
                                .child(when(checked, || ui::icon(ui::Icon::Check)))
                                .on_press(move || {
                                    if let Some(circle) = circle() {
                                        if !presence.interactive() || self.busy.get_untracked() {
                                            return;
                                        }
                                        self.busy.set(true);
                                        self.failed.set(false);
                                        let account = self.client();
                                        let id = id.with(|id| id.clone());
                                        let owner = owner.with(|owner| owner.clone());
                                        let add = !checked();
                                        spawn(async move {
                                            self.failed.set(
                                                runtime::commit(async move {
                                                    account
                                                        .conversation()
                                                        .edit_circle_conversation(
                                                            circle.circle_id,
                                                            id,
                                                            owner,
                                                            group,
                                                            add,
                                                        )
                                                        .await
                                                })
                                                .await
                                                .is_err(),
                                            );
                                            self.busy.set(false);
                                        });
                                    }
                                })
                        },
                    )
                    .keys(move || self.ids())
                    .estimated_item_size(parts::row_height())
                    .h(move || parts::dialog_list_height(self.rows.with(Vec::len)))
                    .flex_shrink(1.0)
                    .min_h_0()
                    .style_when(
                        move || self.rows.with(Vec::is_empty),
                        |style| style.hidden(),
                    ),
                )
                .child(when(
                    move || self.rows.with(Vec::is_empty),
                    || {
                        view().min_h_24().items_center().justify_center().child(
                            text(move || {
                                tr(
                                    "Create a circle from the + menu first.",
                                    "请先通过加号菜单创建圈子。",
                                )
                            })
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::fg_secondary),
                        )
                    },
                ))
                .child(
                    ui::action(move || tr("Done", "完成"), move || self.editing.set(None))
                        .flex_none()
                        .disabled(self.busy),
                ),
        )
    }
}

/// Pointer reordering of the circle rail. Its state lives only for one list.
#[derive(Clone, Copy)]
struct Reorder {
    circles: Circles,
    drag: Signal<Option<CircleDrag>>,
    candidate: Stored<Option<(f64, String)>>,
    offset: Signal<f32>,
    viewport: Signal<(f32, f32)>,
    list: ElementRef,
}

impl Reorder {
    fn begin(self) {
        let Some((_, id)) = self.candidate.replace(None) else {
            return;
        };
        if self.circles.busy.get_untracked() {
            return;
        }
        let rows = self.circles.rows.get_untracked();
        let Some(from) = rows.iter().position(|row| row.circle_id == id) else {
            return;
        };
        self.drag.set(Some(CircleDrag {
            circle: rows[from].clone(),
            order: rows.into_iter().map(|row| row.circle_id).collect(),
            from,
            target: from,
            start_scroll: self.offset.get_untracked(),
            delta: 0.0,
            started: false,
        }));
    }

    fn finish(self) {
        let Some(current) = self.drag.get_untracked() else {
            return;
        };
        self.drag.set(None);
        let circles = self.circles;
        if circles.busy.get_untracked()
            || current.order.as_ref() != untracked(|| circles.ids()).as_slice()
            || !current.active()
            || current.from == current.target
        {
            return;
        }
        let mut order = current.order.to_vec();
        let id = order.remove(current.from);
        order.insert(current.target, id);
        let account = circles.client();
        circles.execute(async move { account.conversation().reorder_circles(order).await });
    }

    fn scroll_at_edges(self) {
        let Self {
            circles,
            drag,
            offset,
            viewport,
            list,
            ..
        } = self;
        let edge = computed(move || {
            drag.with(|drag| match drag.as_ref().filter(|drag| drag.active()) {
                Some(drag) if drag.top() < 24.0 => -1_i8,
                Some(drag) if drag.top() + circle_row_height().0 > viewport.get().1 - 24.0 => 1,
                _ => 0,
            })
        });
        effect(move || {
            let direction = edge.get();
            if direction == 0 {
                return;
            }
            // This task exists only while dragging at an edge; no drawing-frame polling.
            spawn(async move {
                loop {
                    timer(Duration::from_millis(50)).await;
                    if drag.with_untracked(|drag| drag.as_ref().is_none_or(|drag| !drag.active())) {
                        break;
                    }
                    let maximum = (circles.rows.with_untracked(Vec::len) as f32
                        * circle_row_height().0
                        - viewport.get_untracked().1)
                        .max(0.0);
                    let next =
                        (offset.get_untracked() + f32::from(direction) * 12.0).clamp(0.0, maximum);
                    if (next - offset.get_untracked()).abs() <= 0.5 {
                        break;
                    }
                    list.scroll_to_offset(px(next), false);
                }
            });
        });
    }

    fn row(
        self,
        counts: Signal<Vec<ConversationUnseenCount>>,
        labels: super::Labels,
        circle: Memo<Option<CircleItem>>,
    ) -> impl IntoElement {
        let Self {
            circles,
            drag,
            candidate,
            ..
        } = self;
        let nav = circles.nav;
        let id = move || circle.with(|circle| circle.as_ref().map(|c| c.circle_id.clone()));
        let name = move || {
            circle.with(|circle| {
                circle
                    .as_ref()
                    .map(|circle| circle.name.clone())
                    .unwrap_or_default()
            })
        };
        let unseen = computed(move || {
            let id = id();
            counts.with(|counts| {
                counts
                    .iter()
                    .find(|count| count.circle_id == id)
                    .map_or((0, 0), |count| (count.count, count.muted_count))
            })
        });
        let count = move || unseen.get().0;
        let muted = move || unseen.get().1;
        let color = move || ui::circle_color(&id().unwrap_or_default());
        let item = ui::nav_row()
            .accessibility_label(move || Some(name().into()))
            .selected(move || {
                !nav.settings.get() && nav.category.get() == "circle" && nav.circle.get() == id()
            })
            .on_press(move || {
                if let Some(id) = id() {
                    nav.select("circle", Some(id));
                }
            })
            .child(super::navigation_icon(
                ui::icon(ui::Icon::Circle).text_color(color),
                nav,
                count,
                muted,
            ))
            .child(labels.bind_text(text(name)))
            .child(when(
                move || !nav.compact() && count() > 0,
                move || ui::count_pill(move || count().to_string(), || false),
            ));
        let dragged = move || {
            drag.with(|drag| {
                drag.as_ref().is_some_and(|drag| {
                    drag.active() && Some(&drag.circle.circle_id) == id().as_ref()
                })
            })
        };
        let content = view()
            .w_full()
            .h(circle_row_height())
            .justify_center()
            .on_touch(move |event: &TouchEvent| {
                if event.phase == TouchPhase::Start
                    && let Some(id) = id()
                {
                    candidate.replace(Some((event.timestamp, id)));
                }
            })
            .opacity(move || if dragged() { 0.25_f32 } else { 1.0_f32 })
            .tooltip(ui::hint(name).disabled(move || !nav.compact() || drag.with(Option::is_some)))
            .child(item.w_full());
        circles.row_menu(circle, content)
    }

    fn preview(self, labels: super::Labels) -> impl IntoElement {
        let Self { drag, offset, .. } = self;
        let name = move || {
            drag.with(|drag| {
                drag.as_ref()
                    .map(|drag| drag.circle.name.clone())
                    .unwrap_or_default()
            })
        };
        view()
            .absolute()
            .inset_0()
            .pointer_events(nsh_core::PointerEvents::None)
            .accessibility_elements_hidden(true)
            .child(
                view()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(circle_row_height())
                    .px_2()
                    .gap_3()
                    .flex_row()
                    .items_center()
                    .rounded(ui::radius_lg())
                    .bg(ui::surface_popover)
                    .border_1()
                    .border_color(ui::border)
                    .translate_y(move || {
                        px(drag.with(|drag| drag.as_ref().map_or(0.0, CircleDrag::top)))
                    })
                    .child(ui::icon(ui::Icon::Circle).text_color(move || {
                        drag.with(|drag| {
                            ui::circle_color(
                                drag.as_ref()
                                    .map_or("", |drag| drag.circle.circle_id.as_str()),
                            )
                        })
                    }))
                    .child(labels.bind_text(text(name))),
            )
            .child(when(
                move || {
                    drag.with(|drag| drag.as_ref().is_some_and(|drag| drag.from != drag.target))
                },
                move || {
                    view()
                        .absolute()
                        .top_0()
                        .left_2()
                        .right_2()
                        .h_0p5()
                        .bg(ui::accent)
                        .translate_y(move || {
                            px(drag.with(|drag| {
                                drag.as_ref()
                                    .map_or(0.0, |drag| (drag.marker(offset.get()) - 1.0).max(0.0))
                            }))
                        })
                },
            ))
    }
}

impl Circles {
    fn row_menu(
        self,
        circle: Memo<Option<CircleItem>>,
        content: impl IntoElement + 'static,
    ) -> impl IntoElement {
        let nav = self.nav;
        ui::native_context_menu(content, move || {
            let Some(circle) = circle.get_untracked() else {
                return Vec::new();
            };
            let busy = self.busy.get_untracked();
            let (first, last) = self.rows.with_untracked(|rows| {
                let is = |row: &CircleItem| row.circle_id == circle.circle_id;
                (rows.first().is_some_and(is), rows.last().is_some_and(is))
            });
            let edit = |editing: fn(CircleItem) -> Editing| {
                let circle = circle.clone();
                move || {
                    nav.close_drawer();
                    self.editing.set(Some(editing(circle.clone())));
                }
            };
            let reorder = |up: bool| {
                let id = circle.circle_id.clone();
                move || self.move_circle(id.clone(), up)
            };
            vec![
                MenuItem::new(tr("Rename…", "重命名…"))
                    .disabled(busy)
                    .on_select(edit(Editing::Rename)),
                MenuItem::new(tr("Edit conversations…", "编辑聊天…"))
                    .disabled(busy)
                    .on_select(edit(Editing::Manage)),
                MenuItem::new(tr("Move up", "上移"))
                    .disabled(busy || first)
                    .on_select(reorder(true)),
                MenuItem::new(tr("Move down", "下移"))
                    .disabled(busy || last)
                    .on_select(reorder(false)),
                MenuItem::new(tr("Delete circle…", "删除圈子…"))
                    .disabled(busy)
                    .on_select(move || {
                        nav.close_drawer();
                        self.confirm_delete(circle.clone());
                    }),
            ]
        })
    }
}

pub(crate) fn membership_item(state: Circles, row: ConversationListData) -> MenuItem {
    MenuItem::new(tr("Circles…", "圈子…"))
        .disabled(state.busy.get_untracked())
        .on_select(move || {
            state
                .editing
                .set(Some(Editing::Membership(Box::new(row.clone()))));
        })
}
