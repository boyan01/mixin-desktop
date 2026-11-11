//! Account-free review scenes for the production pages and dialogs using framed buttons.
use nsh_core::prelude::*;

use super::{Actions, fixtures, messages};
use crate::{ui, ui::parts};

pub(super) const ACTIONS: &[&str] = &["Select", "Next", "Error", "Create error", "Reset"];
pub(super) const ERROR_ACTIONS: &[&str] = &["Error", "Reset"];
pub(super) const CIRCLE_ACTIONS: &[&str] = &["Next", "Error", "Reset"];
pub(super) const PHOTO_ACTIONS: &[&str] = &["Zoom", "Edge", "Rotate", "Error", "Reset"];
pub(super) const STATIC_ACTIONS: &[&str] = &[];

fn state(actions: Actions) -> (Signal<bool>, Signal<bool>, Signal<bool>) {
    let selected = signal(false);
    let next = signal(false);
    let failed = signal(false);
    for name in ACTIONS {
        actions.on(
            name,
            move || {
                selected.set(false);
                next.set(false);
                failed.set(*name == "Reset");
            },
            move || match *name {
                "Select" => selected.set(true),
                "Next" => {
                    selected.set(true);
                    next.set(true);
                }
                "Error" => failed.set(true),
                "Create error" => {
                    selected.set(true);
                    next.set(true);
                    failed.set(true);
                }
                _ => failed.set(false),
            },
        );
    }
    (selected, next, failed)
}

fn host(content: impl IntoElement) -> View {
    view()
        .flex_1()
        .min_h_0()
        .bg(ui::surface_chat)
        .child(content)
}

fn dialog_host(content: impl IntoElement) -> View {
    view()
        .flex_1()
        .min_h_0()
        .relative()
        .p(ui::space_xl())
        .items_center()
        .justify_center()
        .bg(ui::surface_chat)
        .child(ui::scrim())
        .child(content)
}

fn prepare() {
    messages::provide_services();
    provide_context(crate::motion::Presence::modal(|| true));
}

pub(super) fn group(actions: Actions) -> View {
    prepare();
    let (selected, naming, failed) = state(actions);
    dialog_host(crate::create::preview_contacts(
        fixtures::profile(),
        fixtures::picker_users(),
        true,
        naming,
        selected,
        failed,
    ))
}
pub(super) fn chat(actions: Actions) -> View {
    prepare();
    let (selected, naming, failed) = state(actions);
    dialog_host(crate::create::preview_contacts(
        fixtures::profile(),
        fixtures::picker_users(),
        false,
        naming,
        selected,
        failed,
    ))
}
pub(super) fn lookup(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::create::preview_lookup(failed))
}
pub(super) fn circle(actions: Actions) -> View {
    prepare();
    let (_, selecting, failed) = state(actions);
    dialog_host(crate::create::preview_circle(
        fixtures::users(),
        selecting,
        failed,
    ))
}
pub(super) fn share_contact(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::composer::preview_contact(fixtures::users(), failed))
}
pub(super) fn write_post(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::composer::preview_post(failed))
}
pub(super) fn proxy(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::settings::preview_proxy(failed))
}
pub(super) fn clear_cache(_: Actions) -> View {
    prepare();
    dialog_host(crate::settings::preview_clear())
}
pub(super) fn sign_out(_: Actions) -> View {
    prepare();
    dialog_host(crate::settings::preview_sign_out())
}
pub(super) fn login(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, LOGIN_ACTIONS);
    let failed = signal(false);
    effect(move || failed.set(mode.get() == "Error"));
    host(
        when(
            move || mode.get() == "Startup error",
            crate::preview_launch_error,
        )
        .otherwise(move || crate::login::preview(failed)),
    )
}
pub(super) fn image_editor(_: Actions) -> View {
    prepare();
    host(crate::composer::preview_image_editor(
        fixtures::photo_attachment(),
    ))
}
pub(super) fn avatar_crop(_: Actions) -> View {
    prepare();
    dialog_host(crate::avatar::preview_crop(fixtures::photo_attachment()))
}

fn conversation(kind: &str, actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    let mut row = fixtures::conversations::stress_rows().remove(0);
    row.category = "GROUP".into();
    row.name = "设计评审".into();
    let content = crate::conversation::preview_page(
        kind,
        fixtures::USER_ID.into(),
        row,
        fixtures::participants(),
        fixtures::users(),
        failed,
        fixtures::messages::photo_rows(),
    );
    if kind == "invite" {
        dialog_host(content)
    } else {
        host(content)
    }
}
pub(super) fn edit_group(actions: Actions) -> View {
    conversation("edit", actions)
}
pub(super) fn expiry(actions: Actions) -> View {
    conversation("expiry", actions)
}
pub(super) fn members(actions: Actions) -> View {
    conversation("members", actions)
}
pub(super) fn invite(actions: Actions) -> View {
    conversation("invite", actions)
}

fn chat_controls(kind: &str, actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    let content = crate::chat::preview_buttons(
        kind,
        fixtures::USER_ID.into(),
        fixtures::messages::text_rows(),
        failed,
    );
    if matches!(kind, "qr" | "markdown-error" | "transcript" | "summary") {
        dialog_host(content)
    } else {
        host(content)
    }
}
pub(super) fn selection(actions: Actions) -> View {
    chat_controls("selection", actions)
}
pub(super) fn qr(actions: Actions) -> View {
    chat_controls("qr", actions)
}

pub(super) fn recipients(_: Actions) -> View {
    prepare();
    dialog_host(
        ui::dialog_card(ui::DialogWidth::Regular)
            .child(ui::dialog_title("Recipients"))
            .child(crate::recipients::detached(
                fixtures::users(),
                |_| false,
                |_| {},
                true,
            ))
            .child(parts::dialog_actions().child(ui::action(|| "Cancel", || {}))),
    )
}
pub(super) fn transfer_page(_: Actions) -> View {
    prepare();
    host(crate::transfer::preview("page", signal("Normal")))
}
pub(super) fn transfer_approval(_: Actions) -> View {
    prepare();
    dialog_host(crate::transfer::preview("approval", signal("Normal")))
}
pub(super) fn transfer_progress(actions: Actions) -> View {
    prepare();
    dialog_host(crate::transfer::preview(
        "progress",
        mode(actions, TRANSFER_ACTIONS),
    ))
}
pub(super) fn lock(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::security::preview("lock", failed))
}
pub(super) fn passcode(_: Actions) -> View {
    prepare();
    dialog_host(crate::security::preview("setup", signal(false)))
}

pub(super) fn circle_rename(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::sidebar::preview_rename(failed))
}
pub(super) fn circle_editor(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::sidebar::preview_circle_editor(
        fixtures::conversations::stress_rows()
            .into_iter()
            .take(6)
            .collect(),
        failed,
    ))
}
pub(super) fn add_members(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    let mut row = fixtures::conversations::stress_rows().remove(0);
    row.category = "GROUP".into();
    dialog_host(crate::conversation::preview_page(
        "add-members",
        fixtures::USER_ID.into(),
        row,
        fixtures::participants(),
        fixtures::users(),
        failed,
        fixtures::messages::photo_rows(),
    ))
}
pub(super) fn forward(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::chat::preview_forward(
        fixtures::picker_users(),
        failed,
    ))
}

fn link(kind: &str, actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    let value = mixin_desktop_api::CodeResult {
        kind: "conversation".into(),
        user_id: None,
        conversation_id: Some("preview-group".into()),
        conversation_name: Some("设计评审".into()),
        participant_count: 128,
        participant_avatars: fixtures::users()
            .into_iter()
            .map(|user| mixin_desktop_api::GroupAvatar {
                user_id: user.user_id,
                name: user.full_name,
                avatar_url: user.avatar_url,
            })
            .collect(),
        already_member: false,
        asset_id: None,
        asset_symbol: None,
        asset_icon_url: None,
        chain_icon_url: None,
        amount: None,
        senders: Vec::new(),
        receivers: Vec::new(),
        threshold: 0,
        state: None,
        action: None,
    };
    dialog_host(crate::links::preview_panel(kind, value, failed))
}
pub(super) fn link_invite(actions: Actions) -> View {
    link("invite", actions)
}
pub(super) fn link_unsupported(actions: Actions) -> View {
    link("unsupported", actions)
}

pub(super) fn sticker_save(_: Actions) -> View {
    prepare();
    dialog_host(crate::chat::preview_buttons(
        "add-sticker",
        fixtures::USER_ID.into(),
        fixtures::messages::photo_rows()
            .into_iter()
            .filter(|row| row.media_url.as_ref().is_some_and(|url| !url.is_empty()))
            .collect(),
        signal(false),
    ))
}
pub(super) fn bot_ending(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::bot::preview_ending(failed))
}

pub(super) fn mute(_: Actions) -> View {
    prepare();
    dialog_host(crate::conversation_list::preview_mute())
}
pub(super) fn conversation_circles(actions: Actions) -> View {
    conversation("circles", actions)
}
pub(super) fn link_send(actions: Actions) -> View {
    prepare();
    dialog_host(
        ui::dialog_card(ui::DialogWidth::Regular).child(crate::links::preview_send(
            fixtures::users(),
            mode(actions, LINK_ACTIONS),
            fixtures::sticker_image(0),
            fixtures::users().remove(0),
        )),
    )
}
pub(super) fn empty_filtered(_: Actions) -> View {
    prepare();
    host(crate::home::preview_empty_filtered())
}

pub(super) fn transcript(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, TRANSCRIPT_ACTIONS);
    let failed = signal(false);
    effect(move || failed.set(mode.get() == "Error"));
    dialog_host(dynamic(move || {
        let rows = if mode.get() == "Media" {
            let mut rows = fixtures::messages::photo_rows();
            rows.retain(|row| row.media_status == "DONE");
            rows.extend(fixtures::messages::media_rows().into_iter().filter(|row| {
                row.category.ends_with("_STICKER")
                    && row.sticker_asset_type.as_deref() != Some("JSON")
            }));
            rows.sort_by_key(|row| std::cmp::Reverse(row.created_at_micros));
            for (index, row) in rows.iter_mut().enumerate() {
                row.message_id = format!("00000000-0000-4000-8000-{:012}", 5_000 + index);
            }
            rows
        } else {
            fixtures::messages::text_rows()
        };
        untracked(|| {
            crate::chat::preview_buttons("transcript", fixtures::USER_ID.into(), rows, failed)
        })
    }))
}
pub(super) const TRANSCRIPT_ACTIONS: &[&str] = &["Media", "Error", "Reset"];
pub(super) fn contact_rename(_: Actions) -> View {
    prepare();
    host(crate::conversation::preview_name(
        fixtures::users().remove(0),
        true,
    ))
}
pub(super) fn contact_add(_: Actions) -> View {
    prepare();
    host(crate::conversation::preview_name(
        fixtures::users().remove(0),
        false,
    ))
}
pub(super) fn profile_settings(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::settings::preview_profile(
        fixtures::profile(),
        failed,
    ))
}

pub(super) fn snapshot_inscription(actions: Actions) -> View {
    prepare();
    dialog_host(
        crate::snapshot::surface(view()).child(crate::snapshot::preview(
            fixtures::snapshot(),
            mode(actions, SNAPSHOT_ACTIONS),
        )),
    )
}

pub(super) fn snapshot_transfer(actions: Actions) -> View {
    prepare();
    dialog_host(
        crate::snapshot::surface(view()).child(crate::snapshot::preview(
            fixtures::transfer_snapshot(),
            mode(actions, TRANSFER_DETAIL_ACTIONS),
        )),
    )
}

pub(super) const TRANSFER_DETAIL_ACTIONS: &[&str] = &[
    "Sent",
    "Pending",
    "Deposit",
    "Withdrawal",
    "Legacy",
    "Error",
    "Reset",
];

pub(super) const TRANSFER_ACTIONS: &[&str] = &["Complete", "Error", "Reset"];
pub(super) const SNAPSHOT_ACTIONS: &[&str] = &[
    "Image",
    "Text",
    "Long text",
    "Error",
    "Content error",
    "Reset",
];
pub(super) const SEARCH_ACTIONS: &[&str] = &["Senders", "Error", "Sender error", "Reset"];
pub(super) const CHAT_ACTIONS: &[&str] = &[
    "Draft error",
    "Unread error",
    "Timeline error",
    "Pinned error",
    "Send unavailable",
    "Reset",
];
pub(super) const LIST_ACTIONS: &[&str] = &["Error", "Command error", "Reset"];

pub(super) fn mode(actions: Actions, names: &'static [&'static str]) -> Signal<&'static str> {
    let state = signal("Normal");
    for &name in names {
        actions.on(
            name,
            move || state.set("Normal"),
            move || state.set(if name == "Reset" { "Normal" } else { name }),
        );
    }
    state
}
pub(super) fn notifications(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::settings::preview_notifications(failed))
}
pub(super) fn storage(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::settings::preview_storage(
        fixtures::conversations::stress_rows()
            .into_iter()
            .take(6)
            .enumerate()
            .map(
                |(i, conversation)| mixin_desktop_api::ConversationStorageUsage {
                    conversation,
                    size_bytes: (i as i64 + 1) * 34567890,
                },
            )
            .collect(),
        failed,
    ))
}
pub(super) fn appearance(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::settings::preview_appearance(failed))
}
pub(super) fn proxies(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::settings::preview_proxies(failed))
}
pub(super) const INFO_ACTIONS: &[&str] = &[
    "Long description",
    "Member",
    "Exited",
    "Edit name",
    "Edit description",
    "Error",
    "Reset",
];
pub(super) const CONTACT_INFO_ACTIONS: &[&str] = &[
    "Long bio",
    "Stranger",
    "Blocked",
    "Bot",
    "Edit name",
    "Error",
    "Reset",
];

pub(super) fn info(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, INFO_ACTIONS);
    host(dynamic(move || {
        let mode = mode.get();
        let mut row = fixtures::conversations::stress_rows().remove(0);
        row.category = "GROUP".into();
        row.name = "设计评审".into();
        if mode == "Exited" {
            row.status = 3;
        }
        let mut members = fixtures::participants();
        if mode == "Member" {
            for member in &mut members {
                if member.user_id == fixtures::USER_ID {
                    member.role = None;
                }
            }
        }
        let kind = match mode {
            "Edit name" => "edit",
            "Edit description" => "description",
            "Long description" => "info-long",
            _ => "info",
        };
        untracked(|| {
            crate::conversation::preview_page(
                kind,
                fixtures::USER_ID.into(),
                row,
                members,
                fixtures::users(),
                signal(mode == "Error"),
                fixtures::messages::photo_rows(),
            )
        })
    }))
}

pub(super) fn contact_info(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, CONTACT_INFO_ACTIONS);
    host(dynamic(move || {
        let mode = mode.get();
        let mut users = fixtures::users();
        let user = &mut users[0];
        user.relationship = match mode {
            "Stranger" => "STRANGER",
            "Blocked" => "BLOCKING",
            _ => "FRIEND",
        }
        .into();
        user.is_bot = mode == "Bot";
        if mode == "Long bio" {
            user.biography = "设计与工程，记录产品改进和发布计划。\nLong biographies remain readable in a narrow chat sidebar.\n".repeat(6);
        }
        let mut row = fixtures::conversations::stress_rows().remove(0);
        row.category = "CONTACT".into();
        row.owner_id = user.user_id.clone();
        row.name = user.full_name.clone();
        row.identity_number = user.identity_number.clone();
        row.relationship = user.relationship.clone();
        row.is_bot = user.is_bot;
        untracked(|| {
            crate::conversation::preview_page(
                if mode == "Edit name" { "edit" } else { "info" },
                fixtures::USER_ID.into(),
                row,
                Vec::new(),
                users,
                signal(mode == "Error"),
                Vec::new(),
            )
        })
    }))
}
pub(super) fn shared_media(actions: Actions) -> View {
    conversation("shared", actions)
}
pub(super) fn contact_profile(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(
        ui::dialog_card(ui::DialogWidth::Compact)
            .p_0()
            .gap_0()
            .overflow_hidden()
            .child(crate::conversation::preview_profile(
                fixtures::users().remove(0),
                failed,
            )),
    )
}
pub(super) fn contact_groups(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::conversation::preview_groups(
        (0..5)
            .map(|i| mixin_desktop_api::GroupConversationItem {
                conversation_id: format!("preview-group-{i}"),
                name: format!("Design review {}", i + 1),
                avatar_url: String::new(),
                participant_count: 128,
            })
            .collect(),
        failed,
    ))
}
pub(super) fn contact_apps(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::conversation::preview_apps(
        (0..4)
            .map(|i| mixin_desktop_api::SharedAppItem {
                app_id: format!("preview-app-{i}"),
                name: format!("Design app {}", i + 1),
                icon_url: String::new(),
                description: "Shared workspace".into(),
                home_uri: String::new(),
            })
            .collect(),
        failed,
        || {},
    ))
}
pub(super) fn pinned_messages(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::inspector::preview_pinned(
        fixtures::messages::text_rows(),
        fixtures::USER_ID.into(),
        failed,
    ))
}
pub(super) fn message_search(actions: Actions) -> View {
    prepare();
    host(crate::inspector::preview_search(
        fixtures::messages::text_rows(),
        fixtures::participants(),
        mode(actions, SEARCH_ACTIONS),
    ))
}
pub(super) fn global_search(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(crate::search::preview(
        fixtures::users(),
        fixtures::messages::text_rows(),
        failed,
    ))
}
pub(super) fn quick_search(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    dialog_host(crate::search::preview_palette(fixtures::users(), failed))
}
pub(super) fn messages(actions: Actions) -> View {
    prepare();
    let state = mode(actions, CHAT_ACTIONS);
    messages::timeline_view(
        view()
            .flex_1()
            .min_h_0()
            .child(parts::page_header("设计评审", None, None))
            .child(when(
                move || state.get() == "Timeline error",
                || crate::chat::timeline_error(|| {}),
            ))
            .child(when(
                move || state.get() == "Unread error",
                || crate::chat::unread_error(|| {}),
            ))
            .child(when(
                move || state.get() == "Pinned error",
                || crate::chat::pinned_error(|| {}),
            ))
            .child(crate::chat::detached_messages(
                fixtures::USER_ID,
                fixtures::messages::text_rows(),
                true,
                fixtures::messages::mention_names(),
            ))
            .child(when(
                move || state.get() == "Draft error",
                || crate::chat::draft_error(|| {}),
            ))
            .child(
                when(
                    move || state.get() == "Send unavailable",
                    crate::chat::preview_unavailable,
                )
                .otherwise(move || super::composer::editor(false, actions)),
            ),
    )
}
pub(super) fn conversations(actions: Actions) -> View {
    prepare();
    let state = mode(actions, LIST_ACTIONS);
    view()
        .flex_1()
        .flex_row()
        .bg(ui::surface_pane)
        .child(
            view()
                .w(px(320.0))
                .min_h_0()
                .child(parts::page_header("Mixin", None, None))
                .child(when(
                    move || state.get() == "Error",
                    || crate::home::load_error(|| {}, || false),
                ))
                .child(when(
                    move || state.get() == "Command error",
                    || crate::conversation_list::command_error(|| {}),
                ))
                .child(crate::conversation_list::detached(
                    fixtures::USER_ID,
                    signal(
                        fixtures::conversations::stress_rows()
                            .into_iter()
                            .take(20)
                            .collect(),
                    ),
                    signal(None),
                    element_ref(),
                )),
        )
        .child(view().flex_1().child(ui::placeholder(|| "Mixin Messenger")))
}

pub(super) const LINK_ACTIONS: &[&str] = &[
    "Sticker",
    "Contact",
    "Error",
    "Sticker error",
    "Contact error",
    "Reset",
];
pub(super) fn summary(actions: Actions) -> View {
    prepare();
    chat_controls("summary", actions)
}
pub(super) fn photo_viewer(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, PHOTO_ACTIONS);
    let mut rows = fixtures::messages::photo_rows()
        .into_iter()
        .filter(|row| row.media_url.as_ref().is_some_and(|url| !url.is_empty()))
        .collect::<Vec<_>>();
    for row in &mut rows {
        row.sender_identity_number = Some(fixtures::IDENTITY_NUMBER.into());
    }
    host(
        view()
            .flex_1()
            .min_h_0()
            .relative()
            .child(messages::timeline_view(crate::chat::detached_messages(
                fixtures::USER_ID,
                fixtures::messages::reply_rows(),
                true,
                fixtures::messages::mention_names(),
            )))
            .child(view().absolute().inset_0().child(ui::scrim()).child(
                crate::chat::preview_photos(rows, fixtures::USER_ID.into(), mode),
            )),
    )
}

pub(super) fn emoji(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    host(
        view().items_center().justify_center().flex_1().child(
            view()
                .w(ui::sticker_panel_width())
                .h(ui::sticker_panel_height())
                .bg(ui::surface_popover)
                .rounded(ui::radius_lg())
                .child(crate::composer::preview_emoji(failed)),
        ),
    )
}

pub(super) const LOGIN_ACTIONS: &[&str] = &["Error", "Startup error", "Reset"];
pub(super) const STICKER_DETAIL_ACTIONS: &[&str] =
    &["Animation", "Not added", "No album", "Error", "Reset"];
pub(super) fn sticker_detail(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, STICKER_DETAIL_ACTIONS);
    dialog_host(dynamic(move || {
        let mode = mode.get();
        untracked(|| {
            let mut detail = fixtures::sticker_detail();
            match mode {
                "Animation" => detail.sticker = detail.album_stickers[0].clone(),
                "Not added" => detail.album.as_mut().unwrap().added = false,
                "No album" => {
                    detail.album = None;
                    detail.album_stickers.clear();
                }
                _ => {}
            }
            crate::composer::preview_sticker_detail(detail, signal(mode == "Error"))
        })
    }))
}

pub(super) fn markdown_attachment(actions: Actions) -> View {
    prepare();
    let (_, _, failed) = state(actions);
    let mut row = fixtures::messages::file_rows().remove(0);
    row.media_name = Some("design-review.md".into());
    dialog_host(
        ui::dialog_card(ui::DialogWidth::Wide)
            .flex_1()
            .min_h_0()
            .child(ui::dialog_close(|| {}))
            .child(crate::chat::preview_markdown(
                row,
                fixtures::USER_ID.into(),
                fixtures::POST_PROSE.into(),
                failed,
            )),
    )
}

pub(super) const VIDEO_ACTIONS: &[&str] = &["Playing", "Muted", "Error", "Reset"];
pub(super) fn video_viewer(actions: Actions) -> View {
    prepare();
    let mode = mode(actions, VIDEO_ACTIONS);
    host(
        view()
            .flex_1()
            .min_h_0()
            .relative()
            .child(messages::timeline_view(crate::chat::detached_messages(
                fixtures::USER_ID,
                fixtures::messages::reply_rows(),
                true,
                fixtures::messages::mention_names(),
            )))
            .child(view().absolute().inset_0().child(ui::scrim()).child(
                crate::chat::preview_video_detail(fixtures::video_message(), mode),
            )),
    )
}
