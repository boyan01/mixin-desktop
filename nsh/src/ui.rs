use base64::{Engine as _, engine::general_purpose::STANDARD};
use nsh_base::{AvatarElement, ButtonElement, Input, avatar, button, input, theme};
use nsh_core::{
    element::{Image, ImageSrc, StyleRefinement},
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
    let asset = match icon {
        Icon::Chat => Some(include_str!("../assets/chat.svg")),
        Icon::Contacts => Some(include_str!("../assets/slide_contacts.svg")),
        Icon::Groups => Some(include_str!("../assets/group.svg")),
        Icon::Bot => Some(include_str!("../assets/bot.svg")),
        Icon::Strangers => Some(include_str!("../assets/strangers.svg")),
        Icon::Circle => Some(include_str!("../assets/circle.svg")),
        Icon::Collapse => Some(include_str!("../assets/collapse.svg")),
        Icon::Expand => Some(include_str!("../assets/expanded.svg")),
        Icon::Filter => Some(include_str!("../assets/filter_unseen.svg")),
        Icon::Add => Some(include_str!("../assets/ic_add.svg")),
        Icon::Search => Some(include_str!("../assets/ic_search_small.svg")),
        Icon::Sticker => Some(include_str!("../assets/ic_sticker.svg")),
        Icon::Microphone => Some(include_str!("../assets/microphone.svg")),
        Icon::Info => Some(include_str!("../assets/information.svg")),
        _ => None,
    };
    if let Some(asset) = asset {
        return svg(ImageSrc::svg_markup(asset)).size_5().flex_shrink_0();
    }
    let path = match icon {
        Icon::Chat => {
            "<path d='M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.4 8.4 0 0 1 3.8-.9h.5a8.5 8.5 0 0 1 8 8z'/>"
        }
        Icon::Search => "<circle cx='10.5' cy='10.5' r='6.5'/><path d='m16 16 4 4'/>",
        Icon::ChevronDown => "<path d='m6 9 6 6 6-6'/>",
        Icon::ArrowLeft => "<path d='M19 12H5m6-6-6 6 6 6'/>",
        Icon::Menu => "<path d='M4 6h16M4 12h16M4 18h16'/>",
        Icon::Play => "<path d='m8 5 11 7-11 7Z'/>",
        Icon::Download => "<path d='M12 3v12m-5-5 5 5 5-5M5 17v4h14v-4'/>",
        Icon::Upload => "<path d='M12 16V4m-5 5 5-5 5 5M5 17v4h14v-4'/>",
        Icon::Close => "<path d='m6 6 12 12M6 18 18 6'/>",
        Icon::Send => "<path d='m22 2-7 20-4-9-9-4Z'/><path d='M22 2 11 13'/>",
        Icon::Globe => {
            "<circle cx='12' cy='12' r='9'/><path d='M3 12h18M12 3a17 17 0 0 1 0 18 17 17 0 0 1 0-18Z'/>"
        }
        Icon::Lock => {
            "<rect x='5' y='10' width='14' height='11' rx='2'/><path d='M8 10V7a4 4 0 0 1 8 0v3'/>"
        }
        Icon::Check => "<path d='m5 12 4 4L19 6'/>",
        Icon::Pin => "<path d='m16 3 5 5-4 1-4 4-1 5-3-3-6 6 6-6-3-3 5-1 4-4Z'/>",
        _ => unreachable!("asset-backed icon"),
    };
    svg(ImageSrc::svg_markup(&format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'>{path}</svg>")))
        .size_5().flex_shrink_0()
}

pub(crate) fn logo() -> Image {
    let source = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(include_bytes!("../assets/logo.png"))
    );
    image(source).size_10().flex_shrink_0().label("Mixin")
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
    view()
        .size(size)
        .flex_shrink_0()
        .rounded_full()
        .overflow_hidden()
        .child(dynamic(move || {
            let row = row();
            untracked(|| {
                let Some(row) = row else {
                    return view().into_element();
                };
                if row.category != "GROUP" || row.group_avatars.is_empty() {
                    let name = stored(row.name);
                    let url = stored(row.avatar_url);
                    return avatar_view(
                        move || name.with(|s| s.clone().into()),
                        move || url.with(|s| s.clone()),
                    )
                    .size_full()
                    .into_element();
                }
                let avatars = row.group_avatars.into_iter().take(4).collect::<Vec<_>>();
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
                    1 => tile(0).into_element(),
                    2 => view()
                        .flex_1()
                        .flex_row()
                        .child(tile(0))
                        .child(tile(1))
                        .into_element(),
                    3 => view()
                        .flex_1()
                        .flex_row()
                        .child(tile(0))
                        .child(view().flex_1().min_w_0().child(tile(1)).child(tile(2)))
                        .into_element(),
                    _ => view()
                        .flex_1()
                        .flex_row()
                        .child(view().flex_1().min_w_0().child(tile(0)).child(tile(1)))
                        .child(view().flex_1().min_w_0().child(tile(2)).child(tile(3)))
                        .into_element(),
                }
            })
        }))
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
        .disabled_style(|s| s.opacity(0.4))
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

pub(crate) fn menu() -> nsh_base::MenuElement {
    nsh_base::menu()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
}

pub(crate) fn context_menu() -> nsh_base::ContextMenuElement {
    nsh_base::context_menu()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
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
                .child(
                    presence.backdrop(
                        view()
                            .absolute()
                            .inset_0()
                            .bg(move || theme().foreground.opacity(0.2))
                            .on_press(move || {
                                if presence.interactive() {
                                    close.with(|close| close());
                                }
                            }),
                    ),
                )
                .child(presence.surface(card)),
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

pub(crate) fn centered(title: impl Fn() -> &'static str + 'static) -> View {
    view()
        .flex_1()
        .min_w_0()
        .p_8()
        .gap_4()
        .items_center()
        .justify_center()
        .child(text(title).text_xl().font_weight(FontWeight::SEMIBOLD))
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
    px(theme().number("login-width").unwrap_or(864.0))
}

pub(crate) fn sidebar_width() -> Points {
    px(theme().number("sidebar-width").unwrap_or(336.0))
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
