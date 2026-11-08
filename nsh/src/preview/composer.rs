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

pub(super) const ACTIONS: &[&str] = &[];

pub(super) fn scene(_: Actions) -> View {
    view().flex_1().min_h_0().bg(ui::surface_chat).child(
        scroll().flex_1().min_h_0().child(
            view()
                .py_4()
                .gap_4()
                .child(example("Message", editor(false)))
                .child(example("Attachments & caption", editor(true)))
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

fn editor(files: bool) -> View {
    let i18n = i18n::current();
    let draft = signal(if files {
        "Here are the files for tomorrow’s review.".to_owned()
    } else {
        String::new()
    });
    let input = composer::Input::detached(draft);
    let tray = if files {
        let mut item = PreparedAttachment::default();
        item.name = "Project proposal.pdf".into();
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
    view().px_4().pt_2().pb_3().child(composer::surface::editor(
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
            .child(tool(ui::Icon::Microphone, "Record voice message")),
        composer::surface::send_button()
            .disabled(move || !files && draft.with(|s| s.trim().is_empty()))
            .accessibility_label(move || Some(i18n.text(Key::Send).into()))
            .child(ui::icon(ui::Icon::Send)),
    ))
}

fn tool(icon: ui::Icon, label: &'static str) -> nsh_base::ButtonElement {
    composer::surface::preview_tool()
        .accessibility_label(move || Some(label.into()))
        .child(ui::icon(icon))
}

fn recording(recorded: bool) -> impl IntoElement {
    let wave: Vec<u8> = (0..64).map(|i| ((i * 37 + 29) % 220 + 20) as u8).collect();
    let snapshot = signal(VoiceRecorderSnapshot {
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
    });
    composer::RecordingPanel {
        snapshot,
        playing: computed(|| false),
        busy: signal(false),
        failed: signal(false),
        cancel: Rc::new(|| {}),
        play: Rc::new(|| {}),
        toggle: Rc::new(|| {}),
        send: Rc::new(|| {}),
    }
    .view()
}
