//! Account-free native window routing fixture using the production shell and input.
use super::Actions;
use crate::{composer::Input, shell, ui};
use nsh_core::{element::desktop::window, prelude::*};
use std::rc::Rc;

pub(super) const ACTIONS: &[&str] = &[];

pub(super) fn scene(_: Actions) -> View {
    let shell = shell::mount();
    let open = signal(false);
    let draft = signal(String::from("Fixture draft"));
    view()
        .flex_1()
        .p_4()
        .gap_4()
        .child(shell::surface(shell))
        .child(
            ui::command()
                .child(text("Open fixture"))
                .on_press(move || open.set(true)),
        )
        .child(text(move || format!("Saved draft: {}", draft.get())))
        .child(
            window()
                .id("close-fixture")
                .title("Mixin Close Fixture — No Account")
                .default_size(Size::new(px(600.0), px(300.0)))
                .open(open)
                .on_open_change(move |event| open.set(event.open))
                .content(move || {
                    let input = Input::detached(draft);
                    view()
                        .flex_1()
                        .bg(ui::surface_pane)
                        .p_4()
                        .gap_4()
                        .child(
                            ui::command()
                                .child(text("Focus outside"))
                                .on_press(move || input.blur()),
                        )
                        .child(input.view(
                            || "Fixture message".into(),
                            computed(|| false),
                            signal(false),
                            Rc::new(|_| {}),
                            Rc::new(|| {}),
                        ))
                }),
        )
}
