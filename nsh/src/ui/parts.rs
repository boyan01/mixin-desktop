//! Page parts shared by Info, Inspector, Settings and the creation surfaces.
//! Unlike the flat component exports, callers keep the `parts::` prefix.

use std::time::Duration;

use futures::{Stream, StreamExt as _};
use nsh_base::{ButtonElement, alert_dialog};
use nsh_core::prelude::*;

use crate::{
    i18n::{self, Key, tr},
    navigation::Navigation,
    ui::{self, Typography as _},
};

/// Search fields wait this long so typing does not start a query per keystroke.
pub(crate) const SEARCH_DELAY: Duration = Duration::from_millis(150);
const CHANGE_DELAY: Duration = Duration::from_millis(300);

pub(crate) fn back_button(on_press: impl Fn() + 'static) -> ButtonElement {
    let i18n = i18n::current();
    ui::glass(ui::round_button())
        .size(ui::glass_bar_height())
        .rounded(ui::glass_radius())
        .accessibility_label(move || Some(i18n.text(Key::Back).into()))
        .on_press(on_press)
        .child(ui::icon(ui::Icon::ArrowLeft))
}

/// Closes the panel the page sits in. It stands at the trailing edge, where Back never does.
pub(crate) fn close_button(on_press: impl Fn() + 'static) -> ButtonElement {
    let i18n = i18n::current();
    ui::glass(ui::round_button())
        .size(ui::glass_bar_height())
        .rounded(ui::glass_radius())
        .accessibility_label(move || Some(i18n.tr("Close", "关闭").into()))
        .on_press(on_press)
        .child(ui::icon(ui::Icon::Close))
}

/// A fixed slot of a page bar, so the title stays centered whatever it holds.
fn bar_slot(button: Option<ButtonElement>) -> View {
    let slot = view().size(ui::glass_bar_height()).flex_shrink_0();
    match button {
        Some(button) => slot.child(button),
        None => slot,
    }
}

/// The bar of a page, in the row shared with the native window controls: `leading`
/// goes back one page, `trailing` closes the panel. A page shows one of them, and
/// the body's own background shows through.
pub(crate) fn page_header(
    title: impl IntoReactive<SharedString>,
    leading: Option<ButtonElement>,
    trailing: Option<ButtonElement>,
) -> View {
    let nav = use_context::<Navigation>().expect("Home provides navigation");
    ui::title_bar()
        .px(ui::space_lg())
        .gap(ui::space_md())
        .border_b_1()
        .border_color(ui::hairline)
        .child(bar_slot(leading))
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
        .child(bar_slot(trailing))
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
    let insets = use_context::<ScrollInsets>();
    let inset = move |edge: fn(ScrollInsets) -> Signal<f32>| {
        move || px(ui::space_md().0 + insets.map_or(0.0, |insets| edge(insets).get()))
    };
    view()
        .w_full()
        .max_w(ui::content_max_width())
        .self_center()
        .p(ui::space_md())
        .pt(inset(|insets| insets.top))
        .pb(inset(|insets| insets.bottom))
        .gap(ui::space_md())
}

/// The room a [`floating_page`]'s header takes at the top, for a page that lays
/// out its own body instead of a [`page_body`]: none outside such a page.
pub(crate) fn top_inset() -> impl Fn() -> Points + Copy + 'static {
    let insets = use_context::<ScrollInsets>();
    move || px(insets.map_or(0.0, |insets| insets.top.get()))
}

/// The scroll padding a [`floating_page`] gives the page bodies inside it.
#[derive(Clone, Copy)]
pub(crate) struct ScrollInsets {
    top: Signal<f32>,
    bottom: Signal<f32>,
}

/// A page whose header floats over its scrolling body: the body scrolls under
/// the header, past a scroll padding of its height, and both ends fade on
/// `color` (`ui::edge_fade`), the background the caller draws. Call before building `body`'s page bodies, or
/// build them in a deferred region, so they see the padding.
pub(crate) fn floating_page(
    color: impl Fn() -> AdaptiveColor + Copy + 'static,
    header: impl IntoElement,
    body: impl IntoElement,
) -> View {
    let insets = ScrollInsets {
        top: signal(0.0),
        bottom: signal(0.0),
    };
    provide_context(insets);
    view()
        .flex_1()
        .min_h_0()
        .relative()
        .child(view().flex_1().min_h_0().child(body))
        .child(
            view()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .pointer_events(PointerEvents::BoxNone)
                .child(ui::edge_fade(false, color)),
        )
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .pointer_events(PointerEvents::BoxNone)
                .on_layout(move |bounds: &Bounds<Points>| {
                    let height = bounds.size.height.0;
                    if insets.top.get_untracked() != height {
                        insets.top.set(height);
                    }
                })
                .child(ui::edge_fade(true, color))
                .child(header),
        )
}

/// Supporting text beside a card, lined up with the rows' labels.
pub(crate) fn note(message: impl Fn() -> &'static str + 'static) -> Text {
    text(message)
        .px(ui::space_lg())
        .text_role(ui::TextRole::Body)
        .text_color(ui::fg_secondary)
}

/// A page of `group`s, such as chat info and its pages: the iOS grouped style.
pub(crate) fn grouped_page() -> View {
    // Sections reach the panel's edges, as the conversation list's rows do.
    page_body().px(Points::ZERO).gap(ui::space_md())
}

/// A section of a grouped page. Its rows are drawn as the conversation list's
/// are: inset from the panel's edges, rounded, apart only by their hover and
/// selected fills; space parts one section from the next. Give every row of one
/// group a leading icon, or none.
pub(crate) fn group() -> View {
    view().px(ui::space_sm())
}

/// A pressable row of a `group`. Add a trailing value, `ui::chevron` when it
/// leads to a page, or `.text_color(ui::danger)` for a destructive command.
pub(crate) fn group_row(
    leading: Option<ui::Icon>,
    label: impl Fn() -> &'static str + 'static,
) -> ButtonElement {
    let row = ui::nav_row()
        .min_h(ui::group_row_height())
        .px(ui::space_md())
        .gap(ui::space_md());
    match leading {
        Some(leading) => row.child(ui::icon(leading).text_color(ui::fg_secondary)),
        None => row,
    }
    .child(
        text(label)
            .text_role(ui::TextRole::Label)
            .flex_1()
            .min_w_0()
            .number_of_lines(1),
    )
}

/// A `group_row` that does not act: a label with its value after it.
pub(crate) fn group_static_row(
    leading: Option<ui::Icon>,
    label: impl Fn() -> &'static str + 'static,
) -> View {
    let row = view()
        .min_h(ui::group_row_height())
        .px(ui::space_md())
        .flex_row()
        .items_center()
        .gap(ui::space_md());
    match leading {
        Some(leading) => row.child(ui::icon(leading).text_color(ui::fg_secondary)),
        None => row,
    }
    .child(
        text(label)
            .text_role(ui::TextRole::Label)
            .flex_1()
            .min_w_0()
            .number_of_lines(1),
    )
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
        .flex_row()
        .gap(ui::space_sm())
        .items_center()
        .child(ui::error_text(message).flex_1().min_w_0())
        .child(ui::link(move || i18n.text(Key::Retry), retry))
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
