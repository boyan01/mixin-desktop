use base64::{Engine as _, engine::general_purpose::STANDARD};
use nsh_base::{
    AvatarElement, ButtonElement, Input, MenuItemElement, TooltipElement, avatar, button, input,
    menu_item, theme, tooltip,
};
use nsh_core::{
    ContextMenuEvent,
    element::{
        Image, ImageSrc, StyleRefinement,
        desktop::{MenuItem, popup_menu_at},
    },
    prelude::*,
};

use crate::{
    i18n::{I18n, Key},
    motion,
};

mod owned;
pub(crate) use owned::owned;

#[derive(Clone, Copy)]
pub(crate) enum Icon {
    Chat,
    Search,
    ChevronDown,
    ArrowLeft,
    Send,
    Globe,
    Lock,
    Check,
    Pin,
    Contacts,
    Groups,
    Bot,
    Strangers,
    Circle,
    Collapse,
    Expand,
    Filter,
    Add,
    Sticker,
    Microphone,
    Info,
    Menu,
    Play,
    Download,
    Upload,
    Close,
}

pub(crate) fn icon(icon: Icon) -> Svg {
    let asset = |markup: &str| svg(ImageSrc::svg_markup(markup)).size_5().flex_shrink_0();
    let stroke = |path: &str| {
        asset(&format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'>{path}</svg>"
        ))
    };
    match icon {
        Icon::Chat => asset(include_str!("../assets/chat.svg")),
        Icon::Contacts => asset(include_str!("../assets/slide_contacts.svg")),
        Icon::Groups => asset(include_str!("../assets/group.svg")),
        Icon::Bot => asset(include_str!("../assets/bot.svg")),
        Icon::Strangers => asset(include_str!("../assets/strangers.svg")),
        Icon::Circle => asset(include_str!("../assets/circle.svg")),
        Icon::Collapse => asset(include_str!("../assets/collapse.svg")),
        Icon::Expand => asset(include_str!("../assets/expanded.svg")),
        Icon::Filter => asset(include_str!("../assets/filter_unseen.svg")),
        Icon::Add => asset(include_str!("../assets/ic_add.svg")),
        Icon::Search => asset(include_str!("../assets/ic_search_small.svg")),
        Icon::Sticker => asset(include_str!("../assets/ic_sticker.svg")),
        Icon::Microphone => asset(include_str!("../assets/microphone.svg")),
        Icon::Info => asset(include_str!("../assets/information.svg")),
        Icon::ChevronDown => stroke("<path d='m6 9 6 6 6-6'/>"),
        Icon::ArrowLeft => stroke("<path d='M19 12H5m6-6-6 6 6 6'/>"),
        Icon::Menu => stroke("<path d='M4 6h16M4 12h16M4 18h16'/>"),
        Icon::Play => stroke("<path d='m8 5 11 7-11 7Z'/>"),
        Icon::Download => stroke("<path d='M12 3v12m-5-5 5 5 5-5M5 17v4h14v-4'/>"),
        Icon::Upload => stroke("<path d='M12 16V4m-5 5 5-5 5 5M5 17v4h14v-4'/>"),
        Icon::Close => stroke("<path d='m6 6 12 12M6 18 18 6'/>"),
        Icon::Send => stroke("<path d='m22 2-7 20-4-9-9-4Z'/><path d='M22 2 11 13'/>"),
        Icon::Globe => stroke(
            "<circle cx='12' cy='12' r='9'/><path d='M3 12h18M12 3a17 17 0 0 1 0 18 17 17 0 0 1 0-18Z'/>",
        ),
        Icon::Lock => stroke(
            "<rect x='5' y='10' width='14' height='11' rx='2'/><path d='M8 10V7a4 4 0 0 1 8 0v3'/>",
        ),
        Icon::Check => stroke("<path d='m5 12 4 4L19 6'/>"),
        Icon::Pin => stroke("<path d='m16 3 5 5-4 1-4 4-1 5-3-3-6 6 6-6-3-3 5-1 4-4Z'/>"),
    }
}

pub(crate) fn logo() -> Image {
    static SOURCE: std::sync::LazyLock<SharedString> = std::sync::LazyLock::new(|| {
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(include_bytes!("../assets/logo.png"))
        )
        .into()
    });
    image(SOURCE.clone())
        .size_10()
        .flex_shrink_0()
        .label("Mixin")
}

pub(crate) fn avatar_view(
    name: impl Fn() -> SharedString + 'static,
    source: impl Fn() -> String + 'static,
) -> AvatarElement {
    avatar()
        .name(move || Some(name()))
        .src(move || {
            let url = source();
            (!url.is_empty()).then(|| ImageSrc::from(url))
        })
        .size_10()
        .flex_shrink_0()
        .rounded_full()
        .overflow_hidden()
        .image_class("rounded-full object-cover")
        .fallback_style(|s| {
            s.items_center()
                .justify_center()
                .bg(move || theme().accent)
                .text_color(move || theme().accent_foreground)
                .font_weight(FontWeight::MEDIUM)
                .text_sm()
        })
}

/// Use the same one-to-four participant puzzle as the Flutter conversation list.
pub(crate) fn conversation_avatar(
    row: impl Fn() -> Option<mixin_desktop_api::ConversationListData> + 'static,
    size: Points,
) -> impl IntoElement {
    // A new message changes the row, not its picture; keep the loaded images.
    let identity = computed(move || {
        row().map(|row| {
            let puzzle = row.category == "GROUP" && !row.group_avatars.is_empty();
            let avatars = puzzle.then(|| row.group_avatars.into_iter().take(4).collect::<Vec<_>>());
            (row.name, row.avatar_url, avatars)
        })
    });
    view()
        .size(size)
        .flex_shrink_0()
        .rounded_full()
        .overflow_hidden()
        .child(dynamic(move || {
            let identity = identity.get();
            untracked(|| match identity {
                None => view().into_element(),
                Some((name, url, None)) => {
                    let name = stored(name);
                    let url = stored(url);
                    avatar_view(
                        move || name.with(|s| s.clone().into()),
                        move || url.with(|s| s.clone()),
                    )
                    .size_full()
                    .into_element()
                }
                Some((_, _, Some(avatars))) => avatar_puzzle(avatars).into_element(),
            })
        }))
}

fn avatar_puzzle(avatars: Vec<mixin_desktop_api::GroupAvatar>) -> View {
    let tile = |at: usize| {
        let name = stored(avatars[at].name.clone());
        let url = stored(avatars[at].avatar_url.clone());
        view().flex_1().min_w_0().min_h_0().overflow_hidden().child(
            avatar_view(
                move || name.with(|s| s.clone().into()),
                move || url.with(|s| s.clone()),
            )
            .size_full()
            .rounded(px(0.0))
            .image_class("object-cover"),
        )
    };
    match avatars.len() {
        1 => tile(0),
        2 => view().flex_1().flex_row().child(tile(0)).child(tile(1)),
        3 => view()
            .flex_1()
            .flex_row()
            .child(tile(0))
            .child(view().flex_1().min_w_0().child(tile(1)).child(tile(2))),
        _ => view()
            .flex_1()
            .flex_row()
            .child(view().flex_1().min_w_0().child(tile(0)).child(tile(1)))
            .child(view().flex_1().min_w_0().child(tile(2)).child(tile(3))),
    }
}

pub(crate) fn command() -> ButtonElement {
    button()
        .min_h_9()
        .px_3()
        .py_2()
        .flex_row()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(move || theme().radius)
        .text_sm()
        .transition_colors()
        .duration(|| motion::duration(motion::FEEDBACK_MS))
        .ease_out()
        .text_color(move || theme().foreground)
        .hover(|s| s.bg(move || theme().muted))
        .active(|s| s.opacity(0.7))
        .focus(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
        .selected_style(|s| s.bg(move || theme().accent))
        .disabled_style(|s| s.opacity(0.4))
}

/// A selectable row of a conversation-sized list.
pub(crate) fn list_row() -> ButtonElement {
    command()
        .justify_start()
        .min_h_20()
        .gap_3()
        .mx_2()
        .px_3()
        .py_3()
}

/// The count pill of list rows and the navigation rail. Quiet when not emphasized.
pub(crate) fn count_badge(
    label: impl Fn() -> String + 'static,
    emphasized: impl Fn() -> bool + 'static,
) -> View {
    view()
        .min_w_5()
        .h_5()
        .px_1()
        .flex_shrink_0()
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || {
            if emphasized() {
                theme().primary
            } else {
                theme().muted_foreground
            }
        })
        .child(
            text(label)
                .text_xs()
                .text_color(move || theme().primary_foreground),
        )
}

pub(crate) fn action(
    label: impl Fn() -> &'static str + 'static,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    command()
        .on_press(on_press)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().secondary)
        .child(text(label))
}

pub(crate) fn primary() -> ButtonElement {
    command()
        .transition()
        .active(|s| s.scale(|| if reduce_motion() { 1.0_f32 } else { 0.985_f32 }))
        .px_4()
        .bg(move || theme().primary)
        .text_color(move || theme().primary_foreground)
        .hover(|s| s.bg(move || theme().primary.opacity(0.9)))
}

/// Use for icon-only commands; list rows keep the unscaled `command` feedback.
pub(crate) fn icon_button() -> ButtonElement {
    command()
        .transition()
        .active(|s| s.scale(|| if reduce_motion() { 1.0_f32 } else { 0.985_f32 }))
}

pub(crate) fn field() -> Input {
    input()
        .min_h_10()
        .px_3()
        .py_2()
        .text_sm()
        .text_color(move || theme().foreground)
        .placeholder_color(|| color(theme().muted_foreground))
        .caret_color(|| color(theme().primary))
        .border_1()
        .border_color(move || theme().input)
        .rounded(move || theme().radius)
        .bg(move || theme().background)
        .transition_colors()
        .duration(|| motion::duration(motion::FEEDBACK_MS))
        .ease_out()
        .focus(|s| s.border_color(move || theme().ring))
}

pub(crate) fn popup(style: StyleRefinement) -> StyleRefinement {
    motion::popup(style)
        .min_w_40()
        .p_1()
        .rounded_lg()
        .bg(move || theme().popover)
        .text_color(move || theme().popover_foreground)
        .border_1()
        .border_color(move || theme().border)
        .shadow_lg()
}

/// A menu or context-menu command.
pub(crate) fn menu_row(label: impl Fn() -> &'static str + 'static) -> MenuItemElement {
    menu_item()
        .px_3()
        .py_2()
        .rounded_md()
        .hover(|s| s.bg(move || theme().accent))
        .focus(|s| s.bg(move || theme().accent))
        .disabled_style(|s| s.opacity(0.4))
        .child(text(label).text_sm())
}

/// The short hint shown beside a control whose label is hidden.
pub(crate) fn hint(label: impl Fn() -> String + 'static) -> TooltipElement {
    tooltip()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(move || theme().popover)
        .text_color(move || theme().popover_foreground)
        .border_1()
        .border_color(move || theme().border)
        .child(text(label).text_sm())
}

pub(crate) fn menu() -> nsh_base::MenuElement {
    nsh_base::menu()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
}

/// Opens the OS menu of `items` on a secondary click over `content`. The items
/// are a snapshot made on each opening; none means no menu, which is how a
/// caller disables it.
pub(crate) fn native_context_menu(
    content: impl IntoElement + 'static,
    items: impl Fn() -> Vec<MenuItem> + 'static,
) -> View {
    view()
        .on_context_menu(move |event: &ContextMenuEvent| {
            let items = items();
            if !items.is_empty() {
                drop(popup_menu_at(items, event.page_position));
            }
        })
        .child(content)
}

pub(crate) fn popover() -> nsh_base::PopoverElement {
    nsh_base::popover()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
}

/// The owner retains the surface for exit; callers guard keyboard submissions.
pub(crate) fn modal(
    presence: motion::Presence,
    close: impl Fn() + 'static,
    card: View,
) -> nsh_core::element::Modal {
    // Release background editing after native presentation records its restore target.
    dismiss_keyboard();
    let close = stored(close);
    nsh_base::dialog()
        .animation(nsh_core::ModalAnimation::None)
        .close_on_escape(false)
        .close_on_backdrop_press(false)
        .on_cancel(move || {
            if presence.interactive() {
                close.with(|close| close());
            }
        })
        .flex_1()
        .child(
            view()
                .flex_1()
                .p_6()
                .items_center()
                .justify_center()
                .child(presence.backdrop(scrim().on_press(move || {
                    if presence.interactive() {
                        close.with(|close| close());
                    }
                })))
                .child(presence.surface(card)),
        )
}

/// The dimming layer behind modal surfaces, dark in either appearance.
pub(crate) fn scrim() -> View {
    view().absolute().inset_0().bg(move || {
        theme()
            .color("media-overlay")
            .unwrap_or(theme().foreground)
            .opacity(0.5)
    })
}

/// The surface of a small dialog; size it further at the call site.
pub(crate) fn modal_card() -> View {
    view()
        .w_80()
        .max_w(relative(1.0))
        .max_h(relative(0.9))
        .min_h_0()
        .p_5()
        .gap_4()
        .rounded_xl()
        .bg(move || theme().popover)
        .text_color(move || theme().popover_foreground)
        .border_1()
        .border_color(move || theme().border)
        .shadow_lg()
}

pub(crate) fn modal_title(title: impl Fn() -> &'static str + 'static) -> Text {
    text(title)
        .flex_none()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
}

/// A recoverable failure, stated next to what it affects.
pub(crate) fn error_text(message: impl Fn() -> &'static str + 'static) -> Text {
    text(message)
        .text_sm()
        .text_color(move || theme().destructive)
}

/// A quiet message that fills an empty pane.
pub(crate) fn placeholder(message: impl Fn() -> &'static str + 'static) -> View {
    view()
        .flex_1()
        .min_h_0()
        .p_6()
        .items_center()
        .justify_center()
        .child(
            text(message)
                .text_sm()
                .text_center()
                .text_color(move || theme().muted_foreground),
        )
}

/// Mount only while actual work is pending. Never manufacture a percentage.
pub(crate) fn busy_indicator(
    busy: impl Fn() -> bool + 'static,
    label: impl Fn() -> SharedString + 'static,
) -> impl IntoElement {
    let label = stored(label);
    when(busy, move || {
        svg(ImageSrc::svg_markup("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'><circle cx='12' cy='12' r='9' opacity='.2'/><path d='M12 3a9 9 0 0 1 9 9'/></svg>"))
            .size_4()
            .flex_shrink_0()
            .role(AccessibilityRole::Progressbar)
            .label(move || label.with(|label| label()))
            .style_with(|s| if reduce_motion() { s.animate_none() } else { s.animate_spin() })
    })
}

pub(crate) fn empty(i18n: I18n, title: Key, body: Key) -> View {
    view()
        .flex_1()
        .min_h_0()
        .p_8()
        .gap_3()
        .items_center()
        .justify_center()
        .child(
            view()
                .size_16()
                .rounded_2xl()
                .bg(move || theme().muted)
                .items_center()
                .justify_center()
                .text_color(move || theme().muted_foreground)
                .child(logo().size_12().opacity(0.65)),
        )
        .child(
            text(move || i18n.text(title))
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_center(),
        )
        .child(
            text(move || i18n.text(body))
                .max_w_80()
                .text_sm()
                .text_center()
                .text_color(move || theme().muted_foreground),
        )
}

pub(crate) fn login_width() -> Points {
    px(theme().number("login-width").unwrap_or(520.0))
}

pub(crate) fn sidebar_width() -> Points {
    px(theme().number("sidebar-width").unwrap_or(300.0))
}

pub(crate) fn navigation_width() -> Points {
    px(theme().number("navigation-width").unwrap_or(176.0))
}

pub(crate) fn navigation_compact_width() -> Points {
    px(theme().number("navigation-compact").unwrap_or(64.0))
}

pub(crate) fn bubble_width() -> Points {
    px(theme().number("bubble-width").unwrap_or(480.0))
}

pub(crate) fn wallpaper() -> Image {
    static SOURCE: std::sync::LazyLock<SharedString> = std::sync::LazyLock::new(|| {
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(include_bytes!("../assets/chat_background.png"))
        )
        .into()
    });
    image(SOURCE.clone())
}

pub(crate) fn setting_toggle(
    label: impl Fn() -> &'static str + Copy + 'static,
    checked: impl Fn() -> bool + 'static,
    disabled: impl Fn() -> bool + 'static,
    change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    view().flex_row().items_center().gap_4().py_3()
        .child(text(label).text_sm().flex_1())
        .child(nsh_base::switch().checked(checked).disabled(disabled)
            .accessibility_label(move || Some(label().into()))
            .on_change(move |checked: &bool| change(*checked))
            .app_class("flex-row items-center rounded-full focus:outline-2 focus:outline-ring disabled:opacity-40")
            .track_class("w-10 h-6 rounded-full bg-muted-foreground/40 checked:bg-primary p-0.5")
            .track_style(|s| s.transition_colors().duration(|| motion::duration(motion::FEEDBACK_MS)).ease_out())
            .thumb_class("w-5 h-5 rounded-full bg-primary-foreground checked:translate-x-4")
            .thumb_style(|s| s.transition_transform().duration(|| if reduce_motion() { 0.0 } else { motion::FEEDBACK_MS }).ease_out()))
}
