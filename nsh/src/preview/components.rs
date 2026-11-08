//! The shared `ui` components in their states, light or dark per `--scheme`.
//! `Disable` and `Enable` switch every control between enabled and disabled.
//! Spinners live in the `loading` scene: a scene with actions stays still once settled.

use nsh_core::prelude::*;

use super::{Actions, fixtures};
use crate::ui::{self, ButtonSize, ButtonTone, Typography as _, parts};

pub(super) const ACTIONS: &[&str] = &["Disable", "Enable"];

pub(super) fn scene(actions: Actions) -> View {
    let disabled = signal(false);
    for (name, value) in [("Disable", true), ("Enable", false)] {
        actions.on(
            name,
            move || disabled.set(!value),
            move || disabled.set(value),
        );
    }
    let off = move || disabled.get();
    view().flex_1().min_h_0().child(
        scroll().flex_1().min_h_0().child(
            view()
                .p(ui::space_lg())
                .gap(ui::space_lg())
                .flex_row()
                .flex_wrap()
                .items_start()
                .child(column("Buttons", buttons(off)))
                .child(column("Rows", rows(off)))
                .child(column("Form", form(off)))
                .child(column("Feedback", feedback()))
                .child(column("Avatars", avatars()))
                .child(column("Dialog", dialog(off))),
        ),
    )
}

fn column(title: &'static str, content: View) -> View {
    view()
        .w(px(260.0))
        .gap(ui::space_sm())
        .child(ui::section_label(move || title))
        .child(content)
}

fn buttons(off: impl Fn() -> bool + Copy + 'static) -> View {
    let tones = [
        (ButtonTone::Primary, "Primary"),
        (ButtonTone::Secondary, "Secondary"),
        (ButtonTone::Ghost, "Ghost"),
        (ButtonTone::Danger, "Danger"),
    ];
    view()
        .gap(ui::space_sm())
        .children(tones.into_iter().map(move |(tone, label)| {
            view()
                .flex_row()
                .gap(ui::space_sm())
                .child(
                    ui::button(tone, ButtonSize::Regular)
                        .disabled(off)
                        .child(text(label)),
                )
                .child(
                    ui::button(tone, ButtonSize::Small)
                        .disabled(off)
                        .child(text(label)),
                )
        }))
        .child(
            view()
                .flex_row()
                .gap(ui::space_sm())
                .items_center()
                .child(ui::pill("Add contact", || {}).disabled(off))
                .child(ui::link("Learn more", || {}).disabled(off))
                .child(
                    ui::round_button()
                        .size_8()
                        .disabled(off)
                        .accessibility_label(|| Some("Search".into()))
                        .child(ui::icon(ui::Icon::Search)),
                ),
        )
}

fn rows(off: impl Fn() -> bool + Copy + 'static) -> View {
    let selected = signal("chats");
    let toggled = signal(true);
    view()
        .gap(ui::space_sm())
        .child(
            view().children(
                [("chats", "All chats"), ("groups", "Groups")]
                    .into_iter()
                    .map(move |(value, label)| {
                        ui::nav_row()
                            .disabled(off)
                            .selected(move || selected.get() == value)
                            .on_press(move || selected.set(value))
                            .child(ui::icon(ui::Icon::Chat))
                            .child(text(label).text_role(ui::TextRole::Body).flex_1())
                            .child(ui::count_pill(|| "8".into(), || false))
                    }),
            ),
        )
        .child(
            ui::card()
                .child(ui::settings_row(Some(ui::Icon::Lock), || "Privacy").child(ui::chevron()))
                .child(ui::settings_row(None, || "No leading icon").child(ui::chevron()))
                .child(ui::toggle_row(
                    || "Message preview",
                    move || toggled.get(),
                    off,
                    move |value| toggled.set(value),
                ))
                .child(ui::static_row(|| "Static row")),
        )
}

fn form(off: impl Fn() -> bool + Copy + 'static) -> View {
    let name = signal(String::from("Alex Chen"));
    let checked = signal(true);
    let tab = signal(0_usize);
    view()
        .gap(ui::space_sm())
        .child(
            ui::field()
                .label(|| "Name")
                .max_length(40)
                .value(move || name.get())
                .on_change(move |value: &str| name.set(value.into())),
        )
        .child(ui::search_pill())
        .child(ui::checkbox(
            Some(|| "ZIP archive"),
            move || checked.get(),
            off,
            move |value| checked.set(value),
        ))
        .child(
            view().flex_row().gap(ui::space_xs()).children(
                ["Media", "Files", "Links"]
                    .into_iter()
                    .enumerate()
                    .map(move |(index, label)| {
                        ui::tab(move || label, move || tab.get() == index)
                            .on_press(move || tab.set(index))
                    }),
            ),
        )
}

fn feedback() -> View {
    view()
        .gap(ui::space_sm())
        .child(
            view()
                .flex_row()
                .gap(ui::space_sm())
                .child(ui::count_pill(|| "3".into(), || false))
                .child(ui::count_pill(|| "128".into(), || true)),
        )
        .child(ui::error_text(|| "Couldn’t save. Try again."))
        .child(parts::note(
            || "A note under a section explains its effect.",
        ))
        .child(ui::divider())
        .child(
            view()
                .h(px(220.0))
                .child(ui::empty("No chats", "Messages you send appear here.")),
        )
}

fn avatars() -> View {
    let sizes = [
        ui::AvatarSize::Small,
        ui::AvatarSize::Compact,
        ui::AvatarSize::Medium,
        ui::AvatarSize::Large,
    ];
    view()
        .gap(ui::space_sm())
        .children(fixtures::PEOPLE.iter().map(move |&(id, name)| {
            view()
                .flex_row()
                .items_center()
                .gap(ui::space_sm())
                .children(sizes.into_iter().map(move |size| {
                    ui::avatar_view(move || id.into(), move || name.into(), String::new, size)
                }))
                .child(text(name).text_role(ui::TextRole::Body))
        }))
}

fn dialog(off: impl Fn() -> bool + Copy + 'static) -> View {
    ui::dialog_surface(view(), ui::DialogWidth::Compact)
        .w_full()
        .p(ui::space_lg())
        .gap(ui::space_md())
        .child(ui::dialog_title("Delete chat"))
        .child(ui::profile_card(
            ui::AvatarSize::Hero,
            || fixtures::USER_ID.into(),
            || fixtures::NAME.into(),
            String::new,
            || format!("Mixin ID: {}", fixtures::IDENTITY_NUMBER),
            view(),
        ))
        .child(
            parts::dialog_actions()
                .child(ui::action(|| "Cancel", || {}).disabled(off))
                .child(
                    ui::button(ButtonTone::Danger, ButtonSize::Regular)
                        .disabled(off)
                        .child(text("Delete")),
                ),
        )
}
