//! Development-only scene browser for checking UI changes without an account.
//! Scenes place production components over fixtures; `.agents/skills/nsh-preview`
//! describes how to capture and compare them.

mod button_pages;
mod components;
mod composer;
mod fixtures;
mod inspector;
mod loading;
mod mentions;
mod messages;
mod posts;
mod sidebar;
mod sticker_store;
mod stickers;
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
        name: "mentions",
        actions: mentions::ACTIONS,
        mount: mentions::scene,
    },
    Scene {
        name: "snapshot-transfer",
        actions: button_pages::TRANSFER_DETAIL_ACTIONS,
        mount: button_pages::snapshot_transfer,
    },
    Scene {
        name: "snapshot-inscription",
        actions: button_pages::SNAPSHOT_ACTIONS,
        mount: button_pages::snapshot_inscription,
    },
    Scene {
        name: "create-group",
        actions: button_pages::ACTIONS,
        mount: button_pages::group,
    },
    Scene {
        name: "create-chat",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::chat,
    },
    Scene {
        name: "create-lookup",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::lookup,
    },
    Scene {
        name: "contact-add",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::contact_add,
    },
    Scene {
        name: "contact-rename",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::contact_rename,
    },
    Scene {
        name: "create-circle",
        actions: button_pages::CIRCLE_ACTIONS,
        mount: button_pages::circle,
    },
    Scene {
        name: "circle-rename",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::circle_rename,
    },
    Scene {
        name: "circle-conversations",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::circle_editor,
    },
    Scene {
        name: "conversation-circles",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::conversation_circles,
    },
    Scene {
        name: "group-edit",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::edit_group,
    },
    Scene {
        name: "group-members",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::members,
    },
    Scene {
        name: "group-add-members",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::add_members,
    },
    Scene {
        name: "group-invite",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::invite,
    },
    Scene {
        name: "group-expiry",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::expiry,
    },
    Scene {
        name: "mute",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::mute,
    },
    Scene {
        name: "message-forward",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::forward,
    },
    Scene {
        name: "recipients",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::recipients,
    },
    Scene {
        name: "message-selection",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::selection,
    },
    Scene {
        name: "message-reply",
        actions: composer::REPLY_ACTIONS,
        mount: composer::reply_scene,
    },
    Scene {
        name: "message-qr",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::qr,
    },
    Scene {
        name: "transcript",
        actions: button_pages::TRANSCRIPT_ACTIONS,
        mount: button_pages::transcript,
    },
    Scene {
        name: "sticker-save",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::sticker_save,
    },
    Scene {
        name: "write-post",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::write_post,
    },
    Scene {
        name: "share-contact",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::share_contact,
    },
    Scene {
        name: "image-editor",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::image_editor,
    },
    Scene {
        name: "avatar-crop",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::avatar_crop,
    },
    Scene {
        name: "link-invite",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::link_invite,
    },
    Scene {
        name: "link-send",
        actions: button_pages::LINK_ACTIONS,
        mount: button_pages::link_send,
    },
    Scene {
        name: "link-unsupported",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::link_unsupported,
    },
    Scene {
        name: "profile-settings",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::profile_settings,
    },
    Scene {
        name: "proxy-add",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::proxy,
    },
    Scene {
        name: "proxy-settings",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::proxies,
    },
    Scene {
        name: "appearance-settings",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::appearance,
    },
    Scene {
        name: "clear-cache",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::clear_cache,
    },
    Scene {
        name: "sign-out",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::sign_out,
    },
    Scene {
        name: "screen-lock",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::lock,
    },
    Scene {
        name: "passcode-setup",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::passcode,
    },
    Scene {
        name: "transfer-settings",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::transfer_page,
    },
    Scene {
        name: "transfer-approval",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::transfer_approval,
    },
    Scene {
        name: "transfer-progress",
        actions: button_pages::TRANSFER_ACTIONS,
        mount: button_pages::transfer_progress,
    },
    Scene {
        name: "login",
        actions: button_pages::LOGIN_ACTIONS,
        mount: button_pages::login,
    },
    Scene {
        name: "bot-ending",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::bot_ending,
    },
    Scene {
        name: "empty-filtered",
        actions: button_pages::STATIC_ACTIONS,
        mount: button_pages::empty_filtered,
    },
    Scene {
        name: "sticker-store",
        actions: sticker_store::ACTIONS,
        mount: sticker_store::scene,
    },
    Scene {
        name: "stickers",
        actions: stickers::ACTIONS,
        mount: stickers::scene,
    },
    Scene {
        name: "messages",
        actions: button_pages::CHAT_ACTIONS,
        mount: button_pages::messages,
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
        actions: messages::CARD_ACTIONS,
        mount: messages::cards,
    },
    Scene {
        name: "post-reader",
        actions: posts::ACTIONS,
        mount: posts::reader,
    },
    Scene {
        name: "post-blocks",
        actions: posts::ACTIONS,
        mount: posts::blocks,
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
    Scene {
        name: "notifications-settings",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::notifications,
    },
    Scene {
        name: "storage-settings",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::storage,
    },
    Scene {
        name: "shared-media",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::shared_media,
    },
    Scene {
        name: "conversation-info",
        actions: button_pages::INFO_ACTIONS,
        mount: button_pages::info,
    },
    Scene {
        name: "conversation-info-contact",
        actions: button_pages::CONTACT_INFO_ACTIONS,
        mount: button_pages::contact_info,
    },
    Scene {
        name: "contact-profile",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::contact_profile,
    },
    Scene {
        name: "contact-groups",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::contact_groups,
    },
    Scene {
        name: "contact-apps",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::contact_apps,
    },
    Scene {
        name: "pinned-messages",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::pinned_messages,
    },
    Scene {
        name: "message-search",
        actions: button_pages::SEARCH_ACTIONS,
        mount: button_pages::message_search,
    },
    Scene {
        name: "global-search",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::global_search,
    },
    Scene {
        name: "quick-search",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::quick_search,
    },
    Scene {
        name: "chat-switch",
        actions: messages::SWITCH_ACTIONS,
        mount: messages::switch,
    },
    Scene {
        name: "chat-inspector",
        actions: inspector::ACTIONS,
        mount: inspector::scene,
    },
    Scene {
        name: "chat-chrome",
        actions: messages::CHROME_ACTIONS,
        mount: messages::chrome,
    },
    Scene {
        name: "conversations",
        actions: button_pages::LIST_ACTIONS,
        mount: button_pages::conversations,
    },
    Scene {
        name: "transcript-summary",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::summary,
    },
    Scene {
        name: "video-viewer",
        actions: button_pages::VIDEO_ACTIONS,
        mount: button_pages::video_viewer,
    },
    Scene {
        name: "photo-viewer",
        actions: button_pages::PHOTO_ACTIONS,
        mount: button_pages::photo_viewer,
    },
    Scene {
        name: "emoji",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::emoji,
    },
    Scene {
        name: "sticker-detail",
        actions: button_pages::STICKER_DETAIL_ACTIONS,
        mount: button_pages::sticker_detail,
    },
    Scene {
        name: "markdown-attachment",
        actions: button_pages::ERROR_ACTIONS,
        mount: button_pages::markdown_attachment,
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
    let initial = std::env::var("MIXIN_PREVIEW_SCENE")
        .ok()
        .and_then(|name| SCENES.iter().position(|scene| scene.name == name))
        .unwrap_or_else(|| {
            SCENES
                .iter()
                .position(|scene| scene.name == "create-group")
                .unwrap_or(0)
        });
    let current = signal(initial);
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
    let selected = signal(actions.first().copied());
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
        .child(
            ui::button(ButtonTone::Ghost, ButtonSize::Small)
                .accessibility_label(|| Some("Previous scene".into()))
                .on_press(move || current.set((index + SCENES.len() - 1) % SCENES.len()))
                .child(text("‹")),
        )
        .child(
            view().flex_1().min_w_0().child(
                ui::select()
                    .items(|| {
                        SCENES
                            .iter()
                            .map(|scene| nsh_base::SelectItem::new(scene.name, scene.name))
                            .collect::<Vec<_>>()
                    })
                    .value(move || Some(SharedString::from(SCENES[index].name)))
                    .accessibility_label(|| Some("Preview scene".into()))
                    .trigger(ui::select_trigger(move || SCENES[index].name))
                    .on_change(move |value: &Option<SharedString>| {
                        if let Some(index) = value.as_ref().and_then(|name| {
                            SCENES.iter().position(|scene| scene.name == name.as_str())
                        }) {
                            current.set(index);
                        }
                    }),
            ),
        )
        .child(
            ui::button(ButtonTone::Ghost, ButtonSize::Small)
                .accessibility_label(|| Some("Next scene".into()))
                .on_press(move || current.set((index + 1) % SCENES.len()))
                .child(text("›")),
        )
        .child(when(
            move || !actions.is_empty(),
            move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap(ui::space_sm())
                    .w(px(290.0))
                    .max_w(relative(0.6))
                    .min_w_0()
                    .child(
                        view().flex_1().min_w_0().child(
                            ui::select()
                                .items(move || {
                                    actions
                                        .iter()
                                        .map(|name| nsh_base::SelectItem::new(*name, *name))
                                        .collect::<Vec<_>>()
                                })
                                .value(move || selected.get().map(SharedString::from))
                                .accessibility_label(|| Some("Preview action".into()))
                                .trigger(ui::select_trigger(move || {
                                    selected.get().unwrap_or("State")
                                }))
                                .on_change(move |value: &Option<SharedString>| {
                                    selected.set(value.as_ref().and_then(|value| {
                                        actions
                                            .iter()
                                            .find(|name| **name == value.as_str())
                                            .copied()
                                    }))
                                }),
                        ),
                    )
                    .child(
                        ui::button(ButtonTone::Secondary, ButtonSize::Small)
                            .accessibility_label(|| Some("Play action".into()))
                            .on_press(move || {
                                if let Some(name) = selected.get_untracked() {
                                    requested.set(Some((name, Step::Prepare)));
                                    set_timeout(ACTION_LEAD, move || {
                                        requested.set(Some((name, Step::Run)))
                                    });
                                }
                            })
                            .child(text("Play")),
                    )
            },
        ))
}
