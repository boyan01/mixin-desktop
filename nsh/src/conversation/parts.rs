//! Page parts shared by Info, Inspector, Settings and the creation surfaces.

use std::time::Duration;

use futures::{Stream, StreamExt as _};
use nsh_base::{ButtonElement, SelectElement, SelectEntry, alert_dialog, select, theme};
use nsh_core::prelude::*;

use crate::{
    i18n::{self, Key, tr},
    ui,
};

/// Search fields wait this long so typing does not start a query per keystroke.
pub(crate) const SEARCH_DELAY: Duration = Duration::from_millis(150);
const CHANGE_DELAY: Duration = Duration::from_millis(300);
const ROW_HEIGHT: f32 = 64.0;
const VISIBLE_ROWS: usize = 5;

pub(crate) fn back_button(on_press: impl Fn() + 'static) -> ButtonElement {
    let i18n = i18n::current();
    ui::icon_button()
        .size_8()
        .px_1()
        .accessibility_label(move || Some(i18n.text(Key::Back).into()))
        .on_press(on_press)
        .child(ui::icon(ui::Icon::ArrowLeft))
}

pub(crate) fn page_header(title: impl IntoReactive<SharedString>, back: ButtonElement) -> View {
    view()
        .flex_none()
        .h_16()
        .px_4()
        .gap_3()
        .flex_row()
        .items_center()
        .child(back)
        .child(
            text(title)
                .flex_1()
                .min_w_0()
                .number_of_lines(1)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD),
        )
}

/// Mount only for a page with nothing else to show: an inline row that appears
/// on every background refresh shifts the content below it.
pub(crate) fn status(label: impl Fn() -> &'static str + Copy + 'static) -> View {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .text_color(move || theme().muted_foreground)
        .child(ui::busy_indicator(|| true, move || label().into()))
        .child(text(label).text_sm())
}

pub(crate) fn failure(
    message: impl Fn() -> &'static str + 'static,
    retry: impl Fn() + 'static,
) -> View {
    let i18n = i18n::current();
    view()
        .gap_2()
        .items_start()
        .child(ui::error_text(message))
        .child(ui::action(move || i18n.text(Key::Retry), retry))
}

/// The wider dialog card for lists and forms.
pub(crate) fn dialog_card() -> View {
    ui::modal_card().w_96().p_6()
}

pub(crate) fn dialog_actions() -> View {
    view().flex_none().flex_row().justify_end().gap_3()
}

/// Callers supply the items, value, change handler and trigger.
pub(crate) fn option_select() -> SelectElement {
    select()
        .on_open_change(|event: &nsh_base::SelectEvent| {
            if matches!(event, nsh_base::SelectEvent::OpenChange(true, _)) {
                dismiss_keyboard();
            }
        })
        .popup_style(ui::popup)
        .item_class(
            "rounded-md focus:bg-accent selected:bg-accent selected:font-medium disabled:opacity-50",
        )
        .render_item(|item: &SelectEntry| {
            view()
                .px_3()
                .py_2()
                .child(text(item.title.clone()).text_sm())
        })
}

pub(crate) fn select_trigger(label: impl IntoReactive<SharedString>) -> ButtonElement {
    ui::command()
        .justify_start()
        .min_h_10()
        .border_1()
        .border_color(move || theme().input)
        .child(text(label).text_sm().flex_1().min_w_0().number_of_lines(1))
        .child(ui::icon(ui::Icon::ChevronDown))
}

/// One selectable list row. An empty `detail` leaves a single line.
pub(crate) fn entity_row(
    name: impl Fn() -> String + Copy + 'static,
    avatar: impl Fn() -> String + 'static,
    detail: impl Fn() -> String + Copy + 'static,
    selected: impl Fn() -> bool + Copy + 'static,
) -> ButtonElement {
    ui::command()
        .justify_start()
        .min_h_16()
        .px_4()
        .gap_3()
        .selected(selected)
        .child(ui::avatar_view(move || name().into(), avatar))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(text(name).text_sm().number_of_lines(1))
                .child(
                    text(detail)
                        .text_xs()
                        .number_of_lines(2)
                        .text_color(move || theme().muted_foreground)
                        .style_when(move || detail().is_empty(), |style| style.hidden()),
                ),
        )
        .child(when(selected, || ui::icon(ui::Icon::Check)))
}

/// Applies to a list inside a content-sized dialog: it shows up to five rows.
pub(crate) fn dialog_list_height(rows: usize) -> Points {
    px(rows.min(VISIBLE_ROWS) as f32 * ROW_HEIGHT)
}

pub(crate) fn row_height() -> Points {
    px(ROW_HEIGHT)
}

pub(crate) fn pager(
    page: Signal<i64>,
    more: impl Fn() -> bool + Copy + 'static,
    disabled: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    when(
        move || page.get() > 0 || more(),
        move || {
            view()
                .flex_none()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    ui::action(
                        move || i18n.tr("Previous page", "上一页"),
                        move || page.update(|page| *page = page.saturating_sub(1)),
                    )
                    .disabled(move || disabled() || page.get() == 0),
                )
                .child(text(move || (page.get() + 1).to_string()).text_sm())
                .child(
                    ui::action(
                        move || i18n.tr("Next page", "下一页"),
                        move || page.update(|page| *page += 1),
                    )
                    .disabled(move || disabled() || !more()),
                )
        },
    )
}

pub(crate) struct Prompt {
    title: &'static str,
    description: String,
    action: &'static str,
}

impl Prompt {
    pub(crate) fn new(
        title: &'static str,
        description: impl Into<String>,
        action: &'static str,
    ) -> Self {
        Self {
            title,
            description: description.into(),
            action,
        }
    }
}

/// Asks before a destructive command. Await it under an owner that outlives the press.
pub(crate) async fn confirm(prompt: Prompt) -> bool {
    matches!(
        alert_dialog()
            .title(prompt.title)
            .description(prompt.description)
            .cancel("cancel", tr("Cancel", "取消"))
            .destructive("apply", prompt.action)
            .show()
            .await,
        Ok(answer) if answer.action.as_deref() == Some("apply")
    )
}

/// Runs `refresh` once per burst of a coarse, account-wide change stream.
pub(crate) async fn throttled<S>(changes: S, refresh: impl Fn())
where
    S: Stream + Unpin,
{
    let mut changes = changes.ready_chunks(32);
    while changes.next().await.is_some() {
        timer(CHANGE_DELAY).await;
        refresh();
    }
}
