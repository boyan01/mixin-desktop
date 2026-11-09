//! Production composer surfaces over local, account-free fixtures.
use super::Actions;
use crate::{
    composer,
    i18n::{self, Key},
    ui,
};
use mixin_desktop_api::{
    PreparedAttachment, VoiceRecorderSnapshot, VoiceRecorderStatus, VoiceRecording,
};
use nsh_core::prelude::*;
use std::rc::Rc;

pub(super) const ACTIONS: &[&str] = &[
    "Text",
    "Lines",
    "Clear",
    "Record",
    "Preview",
    "Cancel",
    "File error",
    "Reset",
];

pub(super) fn scene(actions: Actions) -> View {
    view().flex_1().min_h_0().bg(ui::surface_chat).child(
        scroll().flex_1().min_h_0().child(
            view()
                .py_4()
                .gap_4()
                .child(example("Message", editor(false, actions)))
                .child(example("Attachments & caption", editor(true, actions)))
                .child(example("Recording", recording(false)))
                .child(example("Voice preview", recording(true))),
        ),
    )
}

fn example(label: &'static str, content: impl IntoElement) -> View {
    view()
        .gap_2()
        .child(view().px_4().child(ui::section_label(move || label)))
        .child(content)
}

pub(super) fn editor(files: bool, actions: Actions) -> View {
    let failed = signal(false);
    actions.on(
        "File error",
        move || failed.set(false),
        move || failed.set(true),
    );
    actions.on("Reset", move || failed.set(true), move || failed.set(false));
    let i18n = i18n::current();
    let draft = signal(if files {
        "Here are the files for tomorrow’s review.".to_owned()
    } else {
        String::new()
    });
    let input = composer::Input::detached(draft);
    let open = signal(false);
    let snapshot = signal(voice_snapshot(false));
    if !files {
        for (name, before, after) in [
            ("Text", "", "See you tomorrow."),
            (
                "Lines",
                "See you tomorrow.",
                "First line\nSecond line\nThird line\nFourth line",
            ),
            (
                "Clear",
                "First line\nSecond line\nThird line\nFourth line",
                "",
            ),
        ] {
            actions.on(
                name,
                move || {
                    open.set(false);
                    draft.set(before.to_owned());
                },
                move || draft.set(after.to_owned()),
            );
        }
        for name in ["Record", "Preview", "Cancel"] {
            actions.on(
                name,
                move || {
                    draft.set(String::new());
                    snapshot.set(voice_snapshot(false));
                    open.set(name != "Record");
                },
                move || {
                    if name == "Preview" {
                        snapshot.set(voice_snapshot(true));
                    } else {
                        open.set(name == "Record");
                    }
                },
            );
        }
    }
    let tray = if files {
        let mut item = PreparedAttachment::default();
        item.name = "2026-Q4-project-proposal-and-supporting-documentation-final.pdf".into();
        item.kind = "DATA".into();
        item.size = 1_572_864;
        view().px_3().pt_3().gap_2().child(
            scroll().horizontal().scrollbar(false).child(
                view().flex_row().gap_2().child(
                    composer::attachment_frame(|| true)
                        .child(composer::file_content(item))
                        .child(
                            composer::remove_button(|| false)
                                .accessibility_label(|| Some("Remove file".into())),
                        ),
                ),
            ),
        )
    } else {
        view().hidden()
    };
    let editor = view()
        .px_4()
        .pt_2()
        .pb_3()
        .child(when(
            move || files && failed.get(),
            move || {
                composer::preparation_error(signal(Some("Unsupported media format".into())), || {})
            },
        ))
        .child(composer::surface::editor(
            move || files,
            view().hidden(),
            tray,
            input.view(
                move || i18n.text(Key::EncryptedPlaceholder).to_owned(),
                computed(|| false),
                signal(false),
                Rc::new(|_| {}),
                Rc::new(|| {}),
            ),
            view()
                .flex_row()
                .child(composer::attachment_menu(|| {}, || {}, || {}, || {}))
                .child(tool(ui::Icon::Sticker, "Emoji and stickers"))
                .style_when(move || files, |s| s.hidden()),
            composer::surface::primary_action(
                computed(move || files || draft.with(|s| !s.trim().is_empty())),
                composer::surface::send_button()
                    .accessibility_label(move || Some(i18n.text(Key::Send).into()))
                    .child(ui::icon(ui::Icon::Send)),
                tool(ui::Icon::Microphone, "Record voice message").on_press(move || {
                    snapshot.set(voice_snapshot(false));
                    open.set(true);
                }),
            ),
        ));
    let presence = crate::motion::Presence::fade(move || open.get());
    view()
        .relative()
        .child(composer::surface::text_mode(
            computed(move || open.get()),
            editor,
        ))
        .child(when(
            move || presence.mounted(),
            move || {
                presence.surface(
                    view()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(recording_panel(snapshot, Rc::new(move || open.set(false)))),
                )
            },
        ))
}

fn tool(icon: ui::Icon, label: &'static str) -> nsh_base::ButtonElement {
    composer::surface::preview_tool()
        .accessibility_label(move || Some(label.into()))
        .child(ui::icon(icon))
}

fn voice_snapshot(recorded: bool) -> VoiceRecorderSnapshot {
    let wave: Vec<u8> = (0..64).map(|i| ((i * 37 + 29) % 220 + 20) as u8).collect();
    VoiceRecorderSnapshot {
        session_id: Some(1),
        status: if recorded {
            VoiceRecorderStatus::Recorded
        } else {
            VoiceRecorderStatus::Recording
        },
        recording: recorded.then(|| {
            let mut r = VoiceRecording::default();
            r.duration_millis = 18_000;
            r.waveform = wave.clone();
            r
        }),
        duration_millis: 18_000,
        waveform: wave,
    }
}

fn recording(recorded: bool) -> impl IntoElement {
    recording_panel(signal(voice_snapshot(recorded)), Rc::new(|| {}))
}

fn recording_panel(
    snapshot: Signal<VoiceRecorderSnapshot>,
    close: Rc<dyn Fn()>,
) -> impl IntoElement {
    composer::RecordingPanel {
        snapshot,
        playback: signal(mixin_desktop_api::AudioPlaybackSnapshot {
            status: mixin_desktop_api::AudioPlaybackStatus::Idle,
            item: None,
            position_millis: 0,
            duration_millis: 18_000,
            speed: 1.0,
        }),
        seek: Rc::new(|_| {}),
        busy: signal(false),
        failed: signal(false),
        cancel: close.clone(),
        play: Rc::new(|| {}),
        toggle: Rc::new(move || snapshot.set(voice_snapshot(true))),
        send: close,
    }
    .view()
}

pub(super) const DETAIL_ACTIONS: &[&str] =
    &["ZIP", "Unzip", "Original", "One", "Two", "Files", "Empty"];

pub(super) const REPLY_ACTIONS: &[&str] = &[
    "Long",
    "Image",
    "Attachments",
    "Text",
    "Cancel reply",
    "Reset",
];

/// The real pending reply and editor share one persistent draft in a chat layout.
pub(super) fn reply_scene(actions: Actions) -> View {
    use super::{fixtures, messages};
    messages::provide_services();
    let rows = fixtures::messages::text_rows()
        .into_iter()
        .filter(|row| row.status != "SENDING")
        .collect::<Vec<_>>();
    let initial = rows.last().cloned();
    let long = rows.iter().max_by_key(|row| row.content.len()).cloned();
    let image = fixtures::messages::photo_rows()
        .into_iter()
        .find(|row| row.media_status == "DONE");
    let quote = signal(initial.clone());
    let files = signal(false);
    let draft = signal(String::new());
    let input = composer::Input::detached(draft);
    on_key("escape", move |_| quote.set(None));
    for &name in REPLY_ACTIONS {
        let initial = initial.clone();
        let selected = match name {
            "Long" => long.clone(),
            "Image" => image.clone(),
            _ => initial.clone(),
        };
        actions.on(
            name,
            move || {
                quote.set(initial.clone());
                files.set(false);
                draft.set(if name == "Cancel reply" {
                    "输入的回复不会丢失。".into()
                } else {
                    String::new()
                });
            },
            move || {
                if name == "Cancel reply" {
                    quote.set(None);
                } else {
                    quote.set(selected.clone());
                }
                files.set(name == "Attachments");
                if matches!(name, "Text" | "Attachments") {
                    draft.set("我看到了，稍后确认。".into());
                }
            },
        );
    }
    let i18n = i18n::current();
    let mut item = PreparedAttachment::default();
    item.name = "design-review.pdf".into();
    item.kind = "DATA".into();
    item.size = 1_572_864;
    let item = stored(item);
    let tray = when(files, move || {
        view().px_3().pt_3().child(
            composer::attachment_frame(|| true)
                .child(composer::file_content(item.with(|item| item.clone())))
                .child(composer::remove_button(|| false).on_press(move || files.set(false))),
        )
    });
    view()
        .flex_1()
        .min_h_0()
        .bg(ui::surface_chat)
        .child(messages::timeline_view(crate::chat::detached_messages(
            fixtures::USER_ID,
            rows,
            true,
            fixtures::messages::mention_names(),
        )))
        .child(
            view()
                .px_4()
                .pt_2()
                .pb_3()
                .gap_2()
                .child(composer::surface::editor(
                    move || files.get(),
                    crate::chat::preview_reply(fixtures::USER_ID.into(), quote, input),
                    tray,
                    input.view(
                        move || i18n.text(Key::MessagePlaceholder).to_owned(),
                        computed(|| false),
                        signal(false),
                        Rc::new(|_| {}),
                        Rc::new(|| {}),
                    ),
                    view()
                        .flex_row()
                        .style_when(files, |s| s.hidden())
                        .child(composer::attachment_menu(|| {}, || {}, || {}, || {}))
                        .child(tool(ui::Icon::Sticker, "Emoji and stickers")),
                    composer::surface::primary_action(
                        computed(move || files.get() || draft.with(|s| !s.trim().is_empty())),
                        composer::surface::send_button()
                            .accessibility_label(move || Some(i18n.text(Key::Send).into()))
                            .child(ui::icon(ui::Icon::Send)),
                        tool(ui::Icon::Microphone, "Record voice message"),
                    ),
                )),
        )
}

/// Production surfaces with bounded fixture state for each attachment combination.
pub(super) fn details_scene(actions: Actions) -> View {
    details(actions, false)
}

pub(super) fn fold_scene(actions: Actions) -> View {
    details(actions, true)
}

fn details(actions: Actions, bottom: bool) -> View {
    let i18n = i18n::current();
    let draft = signal(String::new());
    let input = composer::Input::detached(draft);
    let compress = signal(true);
    let archive = signal(false);
    let images = signal(true);
    let count = signal(2_usize);
    let password = signal(String::new());
    let busy = signal(false);
    let last = signal(String::new());
    let strip_revision = signal(0_u64);
    for (name, total) in [("Empty", 0), ("One", 1), ("Two", 2)] {
        actions.on(
            name,
            move || {
                batch(|| {
                    input.blur();
                    compress.set(true);
                    strip_revision.update(|value| *value = value.wrapping_add(1));
                    count.set(2);
                    images.set(true);
                    archive.set(false);
                })
            },
            move || {
                batch(|| {
                    count.set(total);
                    images.set(true);
                    archive.set(false);
                })
            },
        );
    }
    actions.on(
        "Original",
        move || {
            batch(|| {
                input.blur();
                strip_revision.update(|value| *value = value.wrapping_add(1));
                count.set(2);
                images.set(true);
                archive.set(false);
                compress.set(true);
            })
        },
        move || compress.set(false),
    );
    actions.on(
        "Files",
        move || {
            batch(|| {
                input.blur();
                compress.set(true);
                strip_revision.update(|value| *value = value.wrapping_add(1));
                count.set(2);
                images.set(true);
                archive.set(false);
            })
        },
        move || images.set(false),
    );
    actions.on(
        "ZIP",
        move || {
            batch(|| {
                input.blur();
                compress.set(true);
                strip_revision.update(|value| *value = value.wrapping_add(1));
                count.set(2);
                images.set(true);
                archive.set(false);
            })
        },
        move || archive.set(true),
    );
    actions.on(
        "Unzip",
        move || {
            batch(|| {
                input.blur();
                compress.set(true);
                strip_revision.update(|value| *value = value.wrapping_add(1));
                count.set(2);
                images.set(true);
                archive.set(true);
            })
        },
        move || archive.set(false),
    );
    after_answer(move || input.focus());

    let controls = view()
        .flex_row()
        .flex_shrink_0()
        .child(when(
            move || images.get() && count.get() > 0,
            move || {
                composer::original_toggle(
                    move || !compress.get() || archive.get(),
                    move || busy.get() || archive.get(),
                    move || archive.get(),
                    move || compress.update(|value| *value = !*value),
                )
            },
        ))
        .child(when(
            move || count.get() >= 2,
            move || {
                composer::zip_toggle(
                    move || archive.get(),
                    move || busy.get(),
                    move || count.get(),
                    move || archive.update(|value| *value = !*value),
                )
            },
        ));

    view()
        .flex_1()
        .bg(ui::surface_chat)
        .p_4()
        .gap_4()
        .when(bottom, |v| v.child(view().flex_1().min_h_0()))
        .child(composer::surface::editor(
            move || count.get() > 0,
            view().hidden(),
            view()
                .px_3()
                .pt_3()
                .flex_row()
                .items_start()
                .gap_2()
                .style_when(move || count.get() == 0, |s| s.hidden())
                .child(view().flex_1().min_w_0().child(dynamic(move || {
                    strip_revision.get();
                    untracked(|| {
                        composer::attachment_strip(
                            move || archive.get(),
                            move || count.get(),
                            fixture_thumbnails(count, images, archive),
                            composer::archive_password(password, busy),
                        )
                    })
                })))
                .child(controls),
            input.view(
                move || i18n.text(Key::EncryptedPlaceholder).to_owned(),
                computed(|| false),
                signal(false),
                Rc::new(|_| {}),
                Rc::new(|| {}),
            ),
            view().flex_row().child(
                view()
                    .flex_row()
                    .style_when(move || count.get() > 0, |s| s.hidden())
                    .child(composer::attachment_menu(
                        move || last.set("Media selected".into()),
                        move || last.set("Files selected".into()),
                        move || last.set("Contact selected".into()),
                        move || last.set("Post selected".into()),
                    ))
                    .child(tool(ui::Icon::Sticker, "Emoji and stickers")),
            ),
            composer::surface::primary_action(
                computed(move || count.get() > 0 || draft.with(|text| !text.trim().is_empty())),
                composer::surface::send_button().child(ui::icon(ui::Icon::Send)),
                tool(ui::Icon::Microphone, "Record voice message"),
            ),
        ))
        .when(!bottom, |v| {
            v.child(text(move || last.get())).child(live_recording())
        })
}

fn fixture_thumbnails(
    count: Signal<usize>,
    images: Signal<bool>,
    archive: Signal<bool>,
) -> impl IntoElement {
    scroll().horizontal().scrollbar(false).child(
        view()
            .flex_row()
            .gap_2()
            .child(each(
                move || (0..count.get()).collect::<Vec<_>>(),
                |index| *index,
                move |index, _| {
                    let index = index.get_untracked();
                    composer::attachment_frame(move || index != 0 || !images.get())
                        .child(
                            when(
                                move || index == 0 && images.get(),
                                || ui::logo().size_full(),
                            )
                            .otherwise(move || {
                                let mut item = PreparedAttachment::default();
                                item.name = if index == 0 {
                                "2026-Q4-project-proposal-and-supporting-documentation-final.pdf"
                            } else {
                                "2.mp4"
                            }
                            .into();
                                item.kind = if index == 0 { "DATA" } else { "VIDEO" }.into();
                                item.size = 8_900_000;
                                composer::file_content(item)
                            }),
                        )
                        .child(
                            composer::remove_button(move || index == 0 && images.get())
                                .accessibility_label(move || {
                                    Some(format!("Remove attachment {index}").into())
                                })
                                .on_press(move || {
                                    batch(|| {
                                        count.update(|total| *total = total.saturating_sub(1));
                                        if count.get_untracked() < 2 {
                                            archive.set(false);
                                        }
                                    })
                                }),
                        )
                },
            ))
            .child(
                ui::command()
                    .size(ui::thumbnail_grid_size())
                    .min_h_0()
                    .p_0()
                    .flex_shrink_0()
                    .rounded(ui::radius_thumb())
                    .bg(ui::input_fill)
                    .accessibility_label(move || Some(i18n::tr("Add files…", "添加文件…").into()))
                    .on_press(move || count.update(|total| *total += 1))
                    .child(ui::icon(ui::Icon::Add).size(ui::icon_lg())),
            ),
    )
}

/// MIXIN_PREVIEW_AUDIO supplies a 2.4-second local fixture for the native player.
fn live_recording() -> View {
    use futures::StreamExt as _;
    use mixin_desktop_api::{AudioPlaybackEvent, MediaClient};
    let Ok(path) = std::env::var("MIXIN_PREVIEW_AUDIO") else {
        return view().child(recording(true));
    };
    // Match Home: process services exist before the account playback client.
    // Commands and the observed snapshot must resolve to that same scoped client.
    let process = std::sync::Arc::new(MediaClient::new());
    let account_media = std::sync::Arc::new(MediaClient::new());
    provide_context(crate::Clients {
        settings: stored(None),
        media: stored(Some(process.clone())),
    });
    provide_context(stored(Some(account_media.clone())));
    let media = crate::media_client();
    let playback = signal(account_media.audio_playback_snapshot());
    let observe = account_media.clone();
    let failed = signal(false);
    spawn(async move {
        let mut events = Box::pin(crate::runtime::observe(observe.subscribe_audio_playback()));
        while let Some(event) = events.next().await {
            match event {
                AudioPlaybackEvent::Changed(value) => playback.set(value),
                AudioPlaybackEvent::Failed { .. } => failed.set(true),
                _ => {}
            }
        }
    });
    cleanup(move || {
        drop(crate::runtime::commit(async move {
            let account = account_media.close().await;
            let process = process.close().await;
            account?;
            process
        }));
    });
    let mut recording = VoiceRecording::default();
    recording.path = path;
    recording.duration_millis = 2_400;
    recording.waveform = (0..64).map(|i| ((i * 37 + 29) % 220 + 20) as u8).collect();
    let snapshot = signal(VoiceRecorderSnapshot {
        session_id: Some(1),
        status: VoiceRecorderStatus::Recorded,
        duration_millis: recording.duration_millis,
        waveform: recording.waveform.clone(),
        recording: Some(recording),
    });
    let busy = signal(false);
    let owner = Owner::current();
    let playing_media = media.clone();
    view().child(
        composer::RecordingPanel {
            snapshot,
            playback,
            busy,
            failed,
            play: Rc::new(move || {
                if busy.get_untracked() {
                    return;
                }
                let media = playing_media.clone();
                let recording = snapshot
                    .with_untracked(|s| s.recording.clone())
                    .expect("preview recording");
                let current = playback.get_untracked();
                busy.set(true);
                owner.run(|| {
                    spawn(async move {
                        failed.set(
                            crate::runtime::run(async move {
                                composer::preview_playback(&media, &recording, &current).await
                            })
                            .await
                            .is_err(),
                        );
                        busy.set(false);
                    })
                });
            }),
            seek: Rc::new(move |position| media.seek_audio(position)),
            cancel: Rc::new(|| {}),
            toggle: Rc::new(|| {}),
            send: Rc::new(|| {}),
        }
        .view(),
    )
}
