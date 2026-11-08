//! Development-only scene browser for checking UI changes without an account.
//! Scenes place production components over fixtures; `.agents/skills/nsh-preview`
//! describes how to capture and compare them.

mod components;
mod composer;
mod fixtures;
mod loading;
mod messages;
mod sidebar;
mod stress;

use std::time::Duration;

use nsh_core::prelude::*;

use crate::ui::{self, ButtonSize, ButtonTone};

/// An action prepares its start state at once on press, then runs this long after,
/// so a capture has its start frame and is already taking frames when it moves.
const ACTION_LEAD: Duration = Duration::from_millis(800);
/// `nsh/tool/preview.py` (`TOOLBAR_HEIGHT`) crops this band off captures.
const TOOLBAR_HEIGHT: f32 = 40.0;

struct Scene {
    name: &'static str,
    actions: &'static [&'static str],
    mount: fn(Actions) -> View,
}

const SCENES: &[Scene] = &[
    Scene {
        name: "messages",
        actions: messages::ACTIONS,
        mount: messages::text,
    },
    Scene {
        name: "messages-replies",
        actions: messages::ACTIONS,
        mount: messages::replies,
    },
    Scene {
        name: "messages-photos",
        actions: messages::ACTIONS,
        mount: messages::photos,
    },
    Scene {
        name: "messages-media",
        actions: messages::ACTIONS,
        mount: messages::media,
    },
    Scene {
        name: "messages-files",
        actions: messages::ACTIONS,
        mount: messages::files,
    },
    Scene {
        name: "messages-cards",
        actions: messages::ACTIONS,
        mount: messages::cards,
    },
    Scene {
        name: "messages-apps",
        actions: messages::ACTIONS,
        mount: messages::apps,
    },
    Scene {
        name: "messages-wallet",
        actions: messages::ACTIONS,
        mount: messages::wallet,
    },
    Scene {
        name: "messages-system",
        actions: messages::ACTIONS,
        mount: messages::system,
    },
    Scene {
        name: "messages-stress",
        actions: stress::MESSAGE_ACTIONS,
        mount: stress::messages,
    },
    Scene {
        name: "conversations-stress",
        actions: stress::CONVERSATION_ACTIONS,
        mount: stress::conversations,
    },
    Scene {
        name: "sidebar",
        actions: sidebar::ACTIONS,
        mount: sidebar::scene,
    },
    Scene {
        name: "composer-details",
        actions: composer::DETAIL_ACTIONS,
        mount: composer::details_scene,
    },
    Scene {
        name: "composer-fold",
        actions: composer::DETAIL_ACTIONS,
        mount: composer::fold_scene,
    },
    Scene {
        name: "components",
        actions: components::ACTIONS,
        mount: components::scene,
    },
    Scene {
        name: "loading",
        actions: loading::ACTIONS,
        mount: loading::scene,
    },
    Scene {
        name: "composer",
        actions: composer::ACTIONS,
        mount: composer::scene,
    },
];

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Prepare,
    Run,
}

/// The toolbar's requests to the open scene.
#[derive(Clone, Copy)]
struct Actions(Signal<Option<(&'static str, Step)>>);

impl Actions {
    /// Each time the toolbar plays `name`: `prepare` sets the state the action starts
    /// from, without animation, so it always shows a change; `run` follows after
    /// `ACTION_LEAD`.
    fn on(self, name: &'static str, prepare: impl Fn() + 'static, run: impl Fn() + 'static) {
        effect(move || match self.0.get() {
            Some((played, Step::Prepare)) if played == name => untracked(&prepare),
            Some((played, Step::Run)) if played == name => untracked(&run),
            _ => {}
        });
    }
}

#[component]
pub fn Preview() -> impl IntoElement {
    provide_context(crate::i18n::I18n::new());
    let current = signal(0_usize);
    view()
        .flex_1()
        .bg(ui::surface_pane)
        .text_color(ui::fg_primary)
        .child(dynamic(move || {
            let index = current.get();
            untracked(|| scene(index, current))
        }))
}

fn scene(index: usize, current: Signal<usize>) -> View {
    let scene = &SCENES[index];
    // Each mount has its own requests, so a new scene never replays the last action.
    let requested = signal_with(None, SignalEquals::Never);
    view()
        .flex_1()
        .child(toolbar(index, current, scene.actions, requested))
        .child(
            view()
                .flex_1()
                .min_h_0()
                .child((scene.mount)(Actions(requested))),
        )
}

fn toolbar(
    index: usize,
    current: Signal<usize>,
    actions: &'static [&'static str],
    requested: Signal<Option<(&'static str, Step)>>,
) -> View {
    view()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .h(px(TOOLBAR_HEIGHT))
        .gap(ui::space_sm())
        .px(ui::space_md())
        .border_b_1()
        .border_color(ui::hairline)
        .test_id("preview-toolbar")
        // More scenes than a window's width scroll, and the actions stay in view.
        .child(
            scroll()
                .horizontal()
                .scrollbar(false)
                .flex_1()
                .min_w_0()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(ui::space_sm())
                        .children(SCENES.iter().enumerate().map(move |(position, scene)| {
                            let tone = if position == index {
                                ButtonTone::Primary
                            } else {
                                ButtonTone::Ghost
                            };
                            ui::button(tone, ButtonSize::Small)
                                .accessibility_label(move || {
                                    Some(format!("Scene {}", scene.name).into())
                                })
                                .on_press(move || current.set(position))
                                .child(text(scene.name))
                        })),
                ),
        )
        .children(actions.iter().map(move |&name| {
            ui::button(ButtonTone::Secondary, ButtonSize::Small)
                .accessibility_label(move || Some(format!("Action {name}").into()))
                .on_press(move || {
                    requested.set(Some((name, Step::Prepare)));
                    set_timeout(ACTION_LEAD, move || requested.set(Some((name, Step::Run))));
                })
                .child(text(name))
        }))
}
