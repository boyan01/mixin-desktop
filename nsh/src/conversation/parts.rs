//! Page parts shared by Info, Inspector, Settings and the creation surfaces.

use std::time::Duration;

use futures::{Stream, StreamExt as _};
use nsh_base::{ButtonElement, alert_dialog};
use nsh_core::prelude::*;

use crate::{
    i18n::{self, Key, tr},
    sidebar::Navigation,
    ui::{self, Typography as _},
};

/// Search fields wait this long so typing does not start a query per keystroke.
pub(crate) const SEARCH_DELAY: Duration = Duration::from_millis(150);
const CHANGE_DELAY: Duration = Duration::from_millis(300);
const VISIBLE_ROWS: usize = 5;

pub(crate) fn back_button(on_press: impl Fn() + 'static) -> ButtonElement {
    let i18n = i18n::current();
    ui::round_button()
        .size_8()
        .accessibility_label(move || Some(i18n.text(Key::Back).into()))
        .on_press(on_press)
        .child(ui::icon(ui::Icon::ArrowLeft))
}

/// The bar of a page, in the row shared with the native window controls. The
/// spacer balances `back` so the title centers.
pub(crate) fn page_header(title: impl IntoReactive<SharedString>, back: ButtonElement) -> View {
    let nav = use_context::<Navigation>().expect("Home provides navigation");
    ui::title_bar()
        .px(ui::space_lg())
        .gap(ui::space_md())
        .bg(ui::surface_list)
        .border_b_1()
        .border_color(ui::hairline)
        .child(back)
        // The title keeps its own width and the slot centers it, so a resize moves the
        // text, which eases like any frame, instead of re-centering glyphs in a resized one.
        .child(
            view().flex_1().min_w_0().flex_row().justify_center().child(
                text(title)
                    .min_w_0()
                    .number_of_lines(1)
                    .text_role(ui::TextRole::Title),
            ),
        )
        .child(view().size_8().flex_shrink_0())
        // Clears the window controls beside the narrowest rail, so the bar does not shift
        // while the rail eases between its widths.
        .pl(move || {
            let rail = if nav.width().0 > 0.0 {
                ui::navigation_compact_width()
            } else {
                nav.width()
            };
            ui::title_bar_leading(rail)
        })
}

/// The column of a page: one width of cards and sections, centered in a wide pane.
pub(crate) fn page_body() -> View {
    view()
        .w_full()
        .max_w(ui::content_max_width())
        .self_center()
        .p(ui::space_lg())
        .gap(ui::space_lg())
}

/// Supporting text beside a card, lined up with the rows' labels.
pub(crate) fn note(message: impl Fn() -> &'static str + 'static) -> Text {
    text(message)
        .px(ui::space_lg())
        .text_role(ui::TextRole::Body)
        .text_color(ui::fg_secondary)
}

/// A form field and its visible label.
pub(crate) fn labeled(title: impl Fn() -> &'static str + 'static) -> View {
    view()
        .gap(ui::space_sm())
        .child(text(title).text_role(ui::TextRole::Body).text_emphasis())
}

/// Mount only for a page with nothing else to show: an inline row that appears
/// on every background refresh shifts the content below it.
pub(crate) fn status(label: impl Fn() -> &'static str + Copy + 'static) -> View {
    view()
        .flex_row()
        .items_center()
        .gap(ui::space_sm())
        .text_color(ui::fg_secondary)
        .child(ui::busy_indicator(|| true, move || label().into()))
        .child(text(label).text_role(ui::TextRole::Body))
}

pub(crate) fn failure(
    message: impl Fn() -> &'static str + 'static,
    retry: impl Fn() + 'static,
) -> View {
    let i18n = i18n::current();
    view()
        .gap(ui::space_sm())
        .items_start()
        .child(ui::error_text(message))
        .child(ui::action(move || i18n.text(Key::Retry), retry))
}

/// A `Regular` dialog card that holds a page or a long list.
pub(crate) fn tall_dialog_card() -> View {
    ui::dialog_card(ui::DialogWidth::Regular).h(ui::dialog_height_tall())
}

pub(crate) fn dialog_actions() -> View {
    view()
        .flex_none()
        .flex_row()
        .justify_end()
        .gap(ui::space_md())
}

/// One row of a member, contact or result list: a `list_row` at the height of a
/// card row, for what is listed by the screenful.
pub(crate) fn compact_row() -> ButtonElement {
    ui::list_row().min_h(ui::row_height())
}

/// The height of a `compact_row`, margins included.
pub(crate) fn row_height() -> Points {
    ui::row_height() + ui::space_2xs() + ui::space_2xs()
}

/// One selectable list row. An empty `detail` leaves a single line.
pub(crate) fn entity_row(
    id: impl Fn() -> String + 'static,
    name: impl Fn() -> String + Copy + 'static,
    avatar: impl Fn() -> String + 'static,
    detail: impl Fn() -> String + Copy + 'static,
    selected: impl Fn() -> bool + Copy + 'static,
) -> ButtonElement {
    compact_row()
        .selected(selected)
        .child(ui::avatar_view(
            id,
            move || name().into(),
            avatar,
            ui::AvatarSize::Medium,
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap(ui::space_xs())
                .child(text(name).text_role(ui::TextRole::Label).number_of_lines(1))
                .child(
                    text(detail)
                        .text_role(ui::TextRole::Caption)
                        .number_of_lines(2)
                        .text_color(ui::fg_secondary)
                        .style_when(move || detail().is_empty(), |style| style.hidden()),
                ),
        )
        .child(when(selected, || ui::icon(ui::Icon::Check)))
}

/// Applies to a list inside a content-sized dialog: it shows up to five rows.
pub(crate) fn dialog_list_height(rows: usize) -> Points {
    px(rows.min(VISIBLE_ROWS) as f32 * row_height().0)
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
                .gap(ui::space_sm())
                .child(
                    ui::action(
                        move || i18n.tr("Previous page", "上一页"),
                        move || page.update(|page| *page = page.saturating_sub(1)),
                    )
                    .disabled(move || disabled() || page.get() == 0),
                )
                .child(text(move || (page.get() + 1).to_string()).text_role(ui::TextRole::Body))
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
