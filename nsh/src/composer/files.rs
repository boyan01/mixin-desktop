use mixin_desktop_api::{AccountClient, MediaClient, MessageListView, PreparedAttachment};
use nsh_base::{checkbox, radio, radio_group, theme};
use nsh_core::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::{AttachmentSource, tr};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mode {
    Auto,
    Media,
    Files,
    Archive,
}

mod preparation;
use preparation::{Cached, Source, prepare_source, prepare_sources};

fn option(
    title: impl Fn() -> &'static str + Copy + 'static,
    checked: Signal<bool>,
    disabled: impl Fn() -> bool + 'static,
) -> impl IntoElement {
    checkbox()
        .checked(checked)
        .disabled(disabled)
        .app_class("disabled:opacity-40")
        .on_change(move |value: &bool| checked.set(*value))
        .flex_row()
        .items_center()
        .gap_2()
        .py_1()
        .px_1()
        .rounded_md()
        .hover(|s| s.bg(move || theme().accent))
        .focus(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
        .indicator_class("w-4 h-4 border border-border rounded-sm checked:bg-primary")
        .indicator(
            text(move || if checked.get() { "✓" } else { "" })
                .text_xs()
                .text_color(move || theme().primary_foreground),
        )
        .child(text(title).text_sm())
}

#[expect(
    clippy::too_many_arguments,
    reason = "The owner supplies the account, quote and dialog lifetime separately."
)]
pub(super) fn preview(
    account: Arc<AccountClient>,
    conversation: String,
    files: Vec<AttachmentSource>,
    initial_mode: Mode,
    incoming: Signal<Vec<AttachmentSource>>,
    quote: Signal<Option<MessageListView>>,
    close: impl Fn() + Copy + 'static,
    global_failed: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let client = stored(account);
    let conversation = stored(conversation);
    let media = use_context::<Stored<Option<Arc<MediaClient>>>>()
        .expect("App provides media")
        .with(|m| m.clone())
        .expect("Desktop media initialized");
    let media = stored(media);
    let sources = signal_with(
        files
            .into_iter()
            .fold(Vec::<Source>::new(), |mut sources, input| {
                if !sources.iter().any(|source| source.input == input) {
                    sources.push(Source {
                        id: sources.len() as u64,
                        input,
                        original: None,
                        edits: Vec::new(),
                        edited: None,
                    });
                }
                sources
            }),
        SignalEquals::Never,
    );
    let next_id = stored(sources.with_untracked(Vec::len) as u64);
    let cache = stored(HashMap::<u64, Cached>::new());
    let editing = signal(None::<u64>);
    let editor_presence = crate::motion::Presence::fade(move || editing.with(Option::is_some));
    let preparing_edit = signal(false);
    let prepared = signal_with(Vec::<PreparedAttachment>::new(), SignalEquals::Never);
    let selected = signal(0_usize);
    let mode = signal(initial_mode);
    let compress = signal(true);
    let silent = signal(false);
    let caption = signal(String::new());
    let password = signal(String::new());
    let input_rejected = signal(false);
    let loading = signal(true);
    let busy = signal(false);
    let picking = signal(false);
    let failed = signal(false);
    let prepare_failed = signal(false);
    let error = signal(String::new());
    let retry = signal(0_u64);
    let request = signal(0_u64);
    effect(move || {
        let sources = sources.get();
        let mode = mode.get();
        let compress = compress.get();
        retry.get();
        let media = media.with(|m| m.clone());
        let ids = sources
            .iter()
            .map(|source| source.id)
            .collect::<HashSet<_>>();
        cache.with(|cache| cache.retain(|id, _| ids.contains(id)));
        let cached = cache.with(|cache| cache.clone());
        prepared.set(
            sources
                .iter()
                .filter_map(|source| cached.get(&source.id).map(|entry| entry.item.clone()))
                .collect(),
        );
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        loading.set(true);
        prepare_failed.set(false);
        spawn(async move {
            let result = runtime::run(async move {
                prepare_sources(media, sources, mode, compress, cached).await
            })
            .await;
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok((values, cached)) => {
                    cache.replace(cached);
                    prepared.set(values);
                    error.set(String::new());
                }
                Err(problem) => {
                    prepare_failed.set(true);
                    error.set(problem.to_string());
                }
            }
            loading.set(false);
        });
    });
    let mode_value = move || match mode.get() {
        Mode::Auto => {
            if prepared.with(|items| {
                items
                    .iter()
                    .any(|item| matches!(item.kind.as_str(), "IMAGE" | "VIDEO"))
            }) {
                Mode::Media
            } else {
                Mode::Files
            }
        }
        mode => mode,
    };
    let append = move |inputs: Vec<AttachmentSource>| {
        if inputs.is_empty() {
            return;
        }
        if busy.get_untracked() || !presence.interactive() {
            input_rejected.set(true);
            return;
        }
        input_rejected.set(false);
        let mut additions = Vec::new();
        for input in inputs {
            if !sources.with_untracked(|sources| sources.iter().any(|source| source.input == input))
                && !additions.contains(&input)
            {
                additions.push(input);
            }
        }
        if additions.is_empty() {
            return;
        }
        sources.update(|sources| {
            for input in additions {
                let id = next_id.get();
                next_id.replace(id + 1);
                sources.push(Source {
                    id,
                    input,
                    original: None,
                    edits: Vec::new(),
                    edited: None,
                });
            }
        });
    };
    effect(move || {
        if !presence.interactive() || incoming.with(Vec::is_empty) {
            return;
        }
        let mut inputs = Vec::new();
        incoming.update(|items| inputs = std::mem::take(items));
        append(inputs);
    });
    on_key_with(
        "mod+v",
        KeyOptions {
            when: nsh_core::keymap::When::NotEditing,
            ..Default::default()
        },
        move |_| {
            if !presence.interactive() || editor_presence.mounted() {
                return;
            }
            if busy.get_untracked() {
                input_rejected.set(true);
                return;
            }
            spawn(async move {
                let inputs = preparation::clipboard_sources().await;
                append(inputs);
            });
        },
    );
    let edit_selected = move || {
        if busy.get_untracked() || loading.get_untracked() || preparing_edit.get_untracked() {
            return;
        }
        let Some(source) =
            sources.with_untracked(|sources| sources.get(selected.get_untracked()).cloned())
        else {
            return;
        };
        preparing_edit.set(true);
        let media = media.with(|media| media.clone());
        spawn(async move {
            let id = source.id;
            let result = runtime::run(async move {
                if let Some(original) = source.original {
                    return Ok(original);
                }
                let original = prepare_source(&media, &source.input, true, false).await?;
                if original.kind != "IMAGE" {
                    return Ok(original);
                }
                // Normalize orientation once so native preview coordinates and
                // the raster recipe agree, without changing the selected source.
                let mut normalized = media.edit_image(original.path.clone(), Vec::new()).await?;
                normalized.name = std::path::Path::new(&original.name)
                    .with_extension("png")
                    .to_string_lossy()
                    .into_owned();
                drop(original);
                Ok(normalized)
            })
            .await;
            preparing_edit.set(false);
            match result {
                Ok(original) if original.kind == "IMAGE" => {
                    let exists = sources
                        .with_untracked(|sources| sources.iter().any(|source| source.id == id));
                    if exists {
                        sources.update(|sources| {
                            if let Some(source) = sources.iter_mut().find(|source| source.id == id)
                            {
                                source.original = Some(original);
                            }
                        });
                        editing.set(Some(id));
                    }
                }
                _ => failed.set(true),
            }
        });
    };
    let body = view().on_file_drop(move |event: &FileDropEvent| append(preparation::local_sources(&event.uris))).w(px(640.0)).max_w(relative(1.0)).max_h(relative(0.9)).min_h_0().p_6().gap_3().rounded_xl().bg(move || theme().popover)
            .child(text(move || i18n.format("Send {count} files", "发送 {count} 个文件", &[("count", &sources.with(Vec::len).to_string())])).flex_none().text_lg().font_weight(FontWeight::SEMIBOLD))
            .child(scroll().flex_shrink(1.0).min_h_0().child(view().gap_3()
            .child(ui::action(move || tr(i18n, "Add files…", "添加文件…"), move || {
                if busy.get_untracked() || picking.get_untracked() || !presence.interactive() { return; }
                picking.set(true);
                spawn(async move {
                    let result = pick_paths(nsh_core::file_dialog::OpenDialog::files().multiple(true)).await;
                    picking.set(false);
                    match result {
                        Ok(Some(paths)) => append(paths.into_iter().map(|path| AttachmentSource::Path(path.to_string_lossy().into_owned())).collect()),
                        Ok(None) => {},
                        Err(_) => input_rejected.set(true),
                    }
                });
            }).disabled(move || busy.get() || picking.get()))
            .child(when(input_rejected, move || text(move || tr(i18n, "Couldn’t add these files. Try again.", "无法添加这些文件，请重试。")).text_sm().text_color(move || theme().destructive)))
            .child(radio_group().flex_row().flex_wrap().gap_2().children([
                (Mode::Media, "Photos and videos", "图片和视频"),
                (Mode::Files, "Files", "文件"),
                (Mode::Archive, "ZIP archive", "ZIP 压缩包"),
            ].into_iter().map(move |(choice, english, chinese)| radio()
                .checked(move || mode_value() == choice)
                .disabled(move || busy.get() || preparing_edit.get() || (choice == Mode::Archive && sources.with(Vec::len) < 2))
                .on_change(move |checked: &bool| { if *checked { mode.set(choice); } })
                .flex_row().items_center().gap_1().px_2().py_1().rounded_md()
                .app_class("disabled:opacity-40").hover(|s| s.bg(move || theme().accent))
                .focus(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
                .child(text(move || if mode_value() == choice { "◉" } else { "○" }).text_color(move || theme().primary))
                .child(text(move || tr(i18n, english, chinese)).text_sm()))))
            .child(when(move || mode_value() == Mode::Archive, move || ui::field().masked(true)
                .label(move || tr(i18n, "Password (optional)", "密码（可选）")).placeholder(move || tr(i18n, "Password (optional)", "密码（可选）").to_owned()).readonly(busy)
                .value(move || password.get()).on_change(move |value: &str| password.set(value.to_owned()))))
            .child(when(move || mode_value() == Mode::Media, move || option(move || tr(i18n, "Compress images", "压缩图片"), compress, move || busy.get())))
            .child(when(loading, move || text(move || tr(i18n, "Preparing files…", "正在准备文件…")).text_sm()))
            .child(when(prepare_failed, move || view().gap_2()
                .child(text(move || tr(i18n, "Couldn’t prepare this media. Send as files or choose other files.", "无法准备此媒体。可以改为文件发送，或选择其他文件。")).text_sm().text_color(move || theme().destructive))
                .child(text(move || error.get()).text_xs().text_color(move || theme().muted_foreground))
                .child(ui::action(move || i18n.text(Key::Retry), move || retry.update(|v| *v += 1)))))
            .child(dynamic(move || {
                prepared.with(|_| ());
                let current = sources.with(|rows| rows.get(selected.get()).map(|source| source.id))
                    .and_then(|id| cache.with(|entries| entries.get(&id).map(|entry| entry.item.clone())));
                untracked(|| match current {
                    Some(item) if item.kind == "IMAGE" => image(item.path).w_full().h_56().object_fit(ResizeMode::Contain).label(move || i18n.text(Key::Photo)).into_element(),
                    Some(item) if item.kind == "VIDEO" => video(item.path).w_full().h_56().paused(true).controls(true).into_element(),
                    _ => view().hidden().into_element(),
                })
            }))
            .child(when(move || mode_value() == Mode::Media && prepared.with(|items| items.get(selected.get()).is_some_and(|item| item.kind == "IMAGE")), move ||
                ui::action(move || tr(i18n, "Edit image…", "编辑图片…"), edit_selected).disabled(move || busy.get() || loading.get() || preparing_edit.get())))
            .child(virtual_list(move || sources.with(Vec::len), move |at| {
                ui::command().justify_start().disabled(busy).style_when(move || selected.get() == at.index(), |s| s.bg(move || theme().accent))
                    .on_press(move || selected.set(at.index()))
                    .child(text(move || {
                        prepared.with(|_| ());
                        sources.with(|rows| rows.get(at.index()).map(|source| {
                            cache.with(|entries| entries.get(&source.id)
                                .map(|entry| format!("{} · {} KB", entry.item.name, entry.item.size.div_ceil(1024)))
                                .unwrap_or_else(|| preparation::name(source)))
                        }).unwrap_or_default())
                    }).text_sm().number_of_lines(1))
            }).keys(move || sources.with(|sources| sources.iter().map(|source| source.id).collect::<Vec<_>>())).estimated_item_size(px(36.0)).h(move || px(sources.with(|rows| rows.len().min(5)) as f32 * 36.0)))
            .child(ui::action(move || tr(i18n, "Remove selected file", "移除选中文件"), move || {
                let index = selected.get_untracked();
                sources.update(|sources| { if index < sources.len() { sources.remove(index); } });
                if sources.with(Vec::is_empty) { close(); } else { selected.set(index.min(sources.with(Vec::len) - 1)); }
            }).disabled(move || busy.get() || loading.get() || sources.with(Vec::is_empty)))
            .child(when(move || mode_value() == Mode::Media && prepared.with(|rows| rows.iter().any(|item| item.kind == "IMAGE")), move || ui::field().multiline(true).max_h_24()
                .label(move || tr(i18n, "Caption", "图片说明")).placeholder(move || tr(i18n, "Add a caption…", "添加图片说明…").to_owned()).readonly(busy)
                .value(move || caption.get()).on_change(move |value: &str| caption.set(value.to_owned()))
                .on_paste(move |event: &nsh_core::PasteEvent| append(preparation::pasted_sources(event)))))
            .child(option(move || tr(i18n, "Send without notification", "静音发送"), silent, move || busy.get()))
            .child(when(failed, move || text(move || tr(i18n, "Some files couldn’t be sent. The remaining files are still here.", "部分文件发送失败，尚未发送的文件已保留。")).text_sm().text_color(move || theme().destructive)))
            ))
            .child(view().flex_none().flex_row().justify_end().gap_2()
                .child(ui::action(move || i18n.text(Key::Cancel), move || { if !busy.get_untracked() && !picking.get_untracked() { close(); } }).disabled(move || busy.get() || picking.get()))
                .child(ui::primary().disabled(move || busy.get() || picking.get() || loading.get() || prepare_failed.get() || prepared.with(Vec::is_empty))
                    .child(text(move || i18n.text(if busy.get() { Key::Sending } else { Key::Send })))
                    .on_press(move || {
                        if !presence.interactive() || busy.get_untracked() { return; }
                        let items = prepared.get_untracked(); let source_guards = sources.get_untracked(); let client = client.with(|c| c.clone()); let id = conversation.with(|id| id.clone());
                        let caption = caption.get_untracked(); let silent = silent.get_untracked();
                        let archive = mode_value() == Mode::Archive;
                        let password = password.get_untracked();
                        let media = media.with(|media| media.clone());
                        let sent_quote = quote.with(|q| q.as_ref().map(|m| m.message_id.clone())); let quote_id = sent_quote.clone();
                        busy.set(true); failed.set(false);
                        spawn(async move {
                            let result = runtime::commit(async move {
                                let _source_guards = source_guards;
                                let items = if archive { vec![media.prepare_archive(items, (!password.trim().is_empty()).then(|| password.trim().to_owned())).await?] } else { items };
                                let mut accepted = 0_usize; let mut upload_failed = false;
                                for item in items {
                                    let result = client.message().send_attachment_receipt(id.clone(), item.path.clone(), item.kind.clone(), item.mime_type.clone(), Some(item.name.clone()), item.width, item.height,
                                        item.duration_millis, item.thumbnail.clone(), (item.kind == "IMAGE").then(|| caption.clone()), quote_id.clone(), silent).await;
                                    // Keep the preparation handle (and its temporary path) until
                                    // the account runtime has imported the original bytes.
                                    drop(item);
                                    match result { Ok(receipt) => { accepted += 1; upload_failed |= receipt.upload_error.is_some(); }, Err(_) => return Ok((accepted, upload_failed, false)) }
                                }
                                Ok((accepted, upload_failed, true))
                            }).await;
                            busy.set(false);
                            match result {
                                Ok((_, upload_failed, true)) => { global_failed.set(upload_failed); if quote.with(|q| q.as_ref().map(|m| m.message_id.clone())) == sent_quote { quote.set(None); } close(); },
                                Ok((accepted, upload_failed, false)) => { global_failed.set(upload_failed); if accepted > 0 { sources.update(|sources| { sources.drain(..accepted); }); selected.set(0); } failed.set(true); },
                                Err(_) => failed.set(true),
                            }
                        });
                    })));
    view()
        .child(
            ui::modal(
                presence,
                move || {
                    if !busy.get_untracked()
                        && !picking.get_untracked()
                        && editing.get_untracked().is_none()
                    {
                        close();
                    }
                },
                body,
            )
            .style_when(move || editor_presence.mounted(), |style| style.hidden()),
        )
        .child(when(
            move || editor_presence.mounted(),
            move || {
                let editing_id = editing.get_untracked();
                let source = sources.with_untracked(|sources| {
                    editing_id.and_then(|id| sources.iter().find(|source| source.id == id).cloned())
                });
                untracked(|| match source {
                    Some(source) if source.original.is_some() => {
                        let id = source.id;
                        super::image_editor::screen(
                            media.with(|media| media.clone()),
                            source.original.expect("editor source checked"),
                            source.edits,
                            editor_presence,
                            move |edits, item| {
                                batch(|| {
                                    sources.update(|sources| {
                                        if let Some(source) =
                                            sources.iter_mut().find(|source| source.id == id)
                                        {
                                            source.edits = edits;
                                            source.edited = Some(item);
                                        }
                                    });
                                    editing.set(None);
                                })
                            },
                            move || editing.set(None),
                        )
                        .into_element()
                    }
                    _ => view().hidden().into_element(),
                })
            },
        ))
}
