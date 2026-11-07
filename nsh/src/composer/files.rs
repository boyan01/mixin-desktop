use mixin_desktop_api::{MediaClient, PreparedAttachment};
use nsh_base::{checkbox, radio, radio_group, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use super::{
    AttachmentSource, attachments::Host, clear_quote, dialog_card, dialog_title, failure,
    media_client, quote_id, tr,
};
use crate::{
    i18n::{self, I18n, Key},
    motion::Presence,
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
use preparation::{Cache, Source, prepare_source, prepare_sources};

const ROW_HEIGHT: f32 = 36.0;
const VISIBLE_ROWS: usize = 5;

struct Outcome {
    accepted: usize,
    upload_failed: bool,
    finished: bool,
    caption_sent: bool,
}

#[derive(Clone, Copy)]
struct Preview {
    host: Host,
    i18n: I18n,
    media: Stored<Arc<MediaClient>>,
    // Dialog tasks run here, not under the `when` branch of the control that
    // started them: a branch flip must not cancel a task before `busy` settles.
    owner: Owner,
    sources: Signal<Vec<Source>>,
    next_id: Stored<u64>,
    cache: Stored<Cache>,
    prepared: Signal<Vec<PreparedAttachment>>,
    selected: Signal<usize>,
    editing: Signal<Option<u64>>,
    editor: Presence,
    preparing_edit: Signal<bool>,
    mode: Signal<Mode>,
    compress: Signal<bool>,
    silent: Signal<bool>,
    caption: Signal<String>,
    password: Signal<String>,
    input_rejected: Signal<bool>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    picking: Signal<bool>,
    send_failed: Signal<bool>,
    prepare_error: Signal<Option<String>>,
    retry: Signal<u64>,
}

pub(super) fn preview(
    host: Host,
    files: Vec<AttachmentSource>,
    initial_mode: Mode,
    incoming: Signal<Vec<AttachmentSource>>,
) -> impl IntoElement {
    let mut unique = Vec::new();
    for input in files {
        if !unique.contains(&input) {
            unique.push(input);
        }
    }
    let sources = unique
        .into_iter()
        .enumerate()
        .map(|(id, input)| Source::new(id as u64, input))
        .collect::<Vec<_>>();
    let next_id = sources.len() as u64;
    let editing = signal(None::<u64>);
    let preview = Preview {
        host,
        i18n: i18n::current(),
        media: stored(media_client()),
        owner: Owner::current(),
        sources: signal_with(sources, SignalEquals::Never),
        next_id: stored(next_id),
        cache: stored(Cache::default()),
        prepared: signal_with(Vec::new(), SignalEquals::Never),
        selected: signal(0),
        editing,
        editor: Presence::fade(move || editing.with(Option::is_some)),
        preparing_edit: signal(false),
        mode: signal(initial_mode),
        compress: signal(true),
        silent: signal(false),
        caption: signal(String::new()),
        password: signal(String::new()),
        input_rejected: signal(false),
        loading: signal(true),
        busy: signal(false),
        picking: signal(false),
        send_failed: signal(false),
        prepare_error: signal(None),
        retry: signal(0),
    };
    preview.prepare();
    effect(move || {
        if !host.presence.interactive() || incoming.with(Vec::is_empty) {
            return;
        }
        let mut inputs = Vec::new();
        incoming.update(|items| inputs = std::mem::take(items));
        preview.append(inputs);
    });
    on_key_with(
        "mod+v",
        KeyOptions {
            when: nsh_core::keymap::When::NotEditing,
            ..Default::default()
        },
        move |_| preview.paste(),
    );
    view()
        .child(
            ui::modal(host.presence, move || preview.dismiss(), preview.body())
                .style_when(move || preview.editor.mounted(), |style| style.hidden()),
        )
        .child(when(
            move || preview.editor.mounted(),
            move || preview.image_editor(),
        ))
}

impl Preview {
    fn prepare(self) {
        effect(move || {
            let sources = self.sources.get();
            let mode = self.mode.get();
            let compress = self.compress.get();
            self.retry.get();
            let media = self.media.with(|media| media.clone());
            let cache = self.cache.with(|cache| cache.clone());
            self.prepared.set(cache.retain(&sources));
            self.loading.set(true);
            self.prepare_error.set(None);
            // A rerun cancels this task and its runtime work; finished rows stay cached.
            spawn(async move {
                match runtime::run(prepare_sources(media, sources, mode, compress, cache)).await {
                    Ok(items) => self.prepared.set(items),
                    Err(problem) => self.prepare_error.set(Some(problem.to_string())),
                }
                self.loading.set(false);
            });
        });
    }

    /// `Auto` resolves from what was prepared. A ZIP needs at least two files.
    fn mode_value(self) -> Mode {
        match self.mode.get() {
            Mode::Auto => {
                if self.prepared.with(|items| {
                    items
                        .iter()
                        .any(|item| matches!(item.kind.as_str(), "IMAGE" | "VIDEO"))
                }) {
                    Mode::Media
                } else {
                    Mode::Files
                }
            }
            Mode::Archive if self.sources.with(Vec::len) < 2 => Mode::Files,
            mode => mode,
        }
    }

    fn selected_item(self) -> Option<PreparedAttachment> {
        self.prepared.with(|_| ());
        let id = self
            .sources
            .with(|rows| rows.get(self.selected.get()).map(|source| source.id))?;
        self.cache.with(|cache| cache.item(id))
    }

    fn dismiss(self) {
        if !self.busy.get_untracked()
            && !self.picking.get_untracked()
            && self.editing.get_untracked().is_none()
        {
            self.host.close();
        }
    }

    fn append(self, inputs: Vec<AttachmentSource>) {
        if inputs.is_empty() {
            return;
        }
        if self.busy.get_untracked() || !self.host.presence.interactive() {
            self.input_rejected.set(true);
            return;
        }
        self.input_rejected.set(false);
        let mut additions = Vec::new();
        for input in inputs {
            let present = self
                .sources
                .with_untracked(|sources| sources.iter().any(|source| source.input == input));
            if !present && !additions.contains(&input) {
                additions.push(input);
            }
        }
        if additions.is_empty() {
            return;
        }
        self.sources.update(|sources| {
            for input in additions {
                let id = self.next_id.get();
                self.next_id.replace(id + 1);
                sources.push(Source::new(id, input));
            }
        });
    }

    fn paste(self) {
        if !self.host.presence.interactive() || self.editor.mounted() {
            return;
        }
        if self.busy.get_untracked() {
            self.input_rejected.set(true);
            return;
        }
        self.owner.run(|| {
            spawn(async move { self.append(preparation::clipboard_sources().await) });
        });
    }

    fn pick_more(self) {
        if self.busy.get_untracked()
            || self.picking.get_untracked()
            || !self.host.presence.interactive()
        {
            return;
        }
        self.picking.set(true);
        self.owner.run(|| {
            spawn(async move {
                let result =
                    pick_paths(nsh_core::file_dialog::OpenDialog::files().multiple(true)).await;
                self.picking.set(false);
                match result {
                    Ok(Some(paths)) => self.append(
                        paths
                            .into_iter()
                            .map(|path| AttachmentSource::Path(path.to_string_lossy().into_owned()))
                            .collect(),
                    ),
                    Ok(None) => {}
                    Err(_) => self.input_rejected.set(true),
                }
            });
        });
    }

    fn remove_selected(self) {
        let index = self.selected.get_untracked();
        self.sources.update(|sources| {
            if index < sources.len() {
                sources.remove(index);
            }
        });
        match self.sources.with_untracked(Vec::len) {
            0 => self.host.close(),
            remaining => self.selected.set(index.min(remaining - 1)),
        }
    }

    fn edit_selected(self) {
        if self.busy.get_untracked()
            || self.loading.get_untracked()
            || self.preparing_edit.get_untracked()
        {
            return;
        }
        let Some(source) = self
            .sources
            .with_untracked(|sources| sources.get(self.selected.get_untracked()).cloned())
        else {
            return;
        };
        self.preparing_edit.set(true);
        let media = self.media.with(|media| media.clone());
        self.owner.run(|| {
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
                    let mut normalized =
                        media.edit_image(original.path.clone(), Vec::new()).await?;
                    normalized.name = std::path::Path::new(&original.name)
                        .with_extension("png")
                        .to_string_lossy()
                        .into_owned();
                    drop(original);
                    Ok(normalized)
                })
                .await;
                self.preparing_edit.set(false);
                match result {
                    Ok(original) if original.kind == "IMAGE" => {
                        let exists = self
                            .sources
                            .with_untracked(|sources| sources.iter().any(|row| row.id == id));
                        if exists {
                            self.sources.update(|sources| {
                                if let Some(row) = sources.iter_mut().find(|row| row.id == id) {
                                    row.original = Some(original);
                                }
                            });
                            self.editing.set(Some(id));
                        }
                    }
                    _ => self.send_failed.set(true),
                }
            });
        });
    }

    fn send(self) {
        if !self.host.presence.interactive() || self.busy.get_untracked() {
            return;
        }
        let client = self.host.account();
        let id = self.host.conversation();
        let media = self.media.with(|media| media.clone());
        let items = self.prepared.get_untracked();
        let source_guards = self.sources.get_untracked();
        let archive = untracked(|| self.mode_value()) == Mode::Archive;
        let password = self.password.get_untracked();
        let mut caption = Some(self.caption.get_untracked()).filter(|text| !text.trim().is_empty());
        let silent = self.silent.get_untracked();
        let sent_quote = quote_id(self.host.quote);
        let quote = sent_quote.clone();
        self.busy.set(true);
        self.send_failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move {
                    let _source_guards = source_guards;
                    let items = if archive {
                        let password = password.trim();
                        let password = (!password.is_empty()).then(|| password.to_owned());
                        vec![media.prepare_archive(items, password).await?]
                    } else {
                        items
                    };
                    let mut outcome = Outcome {
                        accepted: 0,
                        upload_failed: false,
                        finished: false,
                        caption_sent: false,
                    };
                    for item in items {
                        // One caption describes the batch: only its first image shows it.
                        let item_caption = (item.kind == "IMAGE").then(|| caption.take()).flatten();
                        let captioned = item_caption.is_some();
                        let result = client
                            .message()
                            .send_attachment_receipt(
                                id.clone(),
                                item.path.clone(),
                                item.kind.clone(),
                                item.mime_type.clone(),
                                Some(item.name.clone()),
                                item.width,
                                item.height,
                                item.duration_millis,
                                item.thumbnail.clone(),
                                item_caption,
                                quote.clone(),
                                silent,
                            )
                            .await;
                        // Keep the preparation handle (and its temporary path) until
                        // the account runtime has imported the original bytes.
                        drop(item);
                        match result {
                            Ok(receipt) => {
                                outcome.accepted += 1;
                                outcome.upload_failed |= receipt.upload_error.is_some();
                                outcome.caption_sent |= captioned;
                            }
                            Err(_) => return Ok(outcome),
                        }
                    }
                    outcome.finished = true;
                    Ok(outcome)
                })
                .await;
                self.busy.set(false);
                match result {
                    Ok(outcome) if outcome.finished => {
                        self.host.failed.set(outcome.upload_failed);
                        clear_quote(self.host.quote, sent_quote);
                        self.host.close();
                    }
                    Ok(outcome) => {
                        self.host.failed.set(outcome.upload_failed);
                        if outcome.caption_sent {
                            self.caption.set(String::new());
                        }
                        if outcome.accepted > 0 {
                            self.sources.update(|sources| {
                                sources.drain(..outcome.accepted);
                            });
                            self.selected.set(0);
                        }
                        self.send_failed.set(true);
                    }
                    Err(_) => self.send_failed.set(true),
                }
            });
        });
    }

    fn body(self) -> View {
        let i18n = self.i18n;
        dialog_card()
            .w(px(640.0))
            .on_file_drop(move |event: &FileDropEvent| {
                self.append(preparation::local_sources(&event.uris))
            })
            .child(dialog_title(move || {
                let count = self.sources.with(Vec::len);
                if count == 1 {
                    tr(i18n, "Send 1 file", "发送 1 个文件").to_owned()
                } else {
                    i18n.format(
                        "Send {count} files",
                        "发送 {count} 个文件",
                        &[("count", &count.to_string())],
                    )
                }
            }))
            .child(
                scroll().flex_shrink(1.0).min_h_0().child(
                    view()
                        .gap_3()
                        .child(self.selected_preview())
                        .child(self.file_list())
                        .child(self.file_actions())
                        .child(self.mode_picker())
                        .child(self.options())
                        .child(self.status()),
                ),
            )
            .child(self.footer())
    }

    fn selected_preview(self) -> impl IntoElement {
        let i18n = self.i18n;
        view()
            .gap_2()
            .child(dynamic(move || {
                let current = self.selected_item();
                untracked(|| match current {
                    Some(item) if item.kind == "IMAGE" => image(item.path)
                        .w_full()
                        .h_56()
                        .object_fit(ResizeMode::Contain)
                        .label(move || i18n.text(Key::Photo))
                        .into_element(),
                    Some(item) if item.kind == "VIDEO" => video(item.path)
                        .w_full()
                        .h_56()
                        .paused(true)
                        .controls(true)
                        .into_element(),
                    _ => view().hidden().into_element(),
                })
            }))
            .child(when(
                move || {
                    self.mode_value() == Mode::Media
                        && self
                            .selected_item()
                            .is_some_and(|item| item.kind == "IMAGE")
                },
                move || {
                    ui::action(
                        move || tr(i18n, "Edit image…", "编辑图片…"),
                        move || self.edit_selected(),
                    )
                    .disabled(move || {
                        self.busy.get() || self.loading.get() || self.preparing_edit.get()
                    })
                },
            ))
    }

    fn file_list(self) -> impl IntoElement {
        virtual_list(
            move || self.sources.with(Vec::len),
            move |at| {
                ui::command()
                    .justify_start()
                    .disabled(self.busy)
                    .selected(move || self.selected.get() == at.index())
                    .selected_style(|s| s.bg(move || theme().accent))
                    .on_press(move || self.selected.set(at.index()))
                    .child(
                        text(move || {
                            self.prepared.with(|_| ());
                            self.sources.with(|rows| {
                                rows.get(at.index())
                                    .map(|source| {
                                        self.cache
                                            .with(|cache| cache.item(source.id))
                                            .map(|item| {
                                                format!("{} · {}", item.name, size_label(item.size))
                                            })
                                            .unwrap_or_else(|| source.name())
                                    })
                                    .unwrap_or_default()
                            })
                        })
                        .text_sm()
                        .number_of_lines(1),
                    )
            },
        )
        .keys(move || {
            self.sources
                .with(|sources| sources.iter().map(|source| source.id).collect::<Vec<_>>())
        })
        .estimated_item_size(px(ROW_HEIGHT))
        .h(move || px(self.sources.with(|rows| rows.len().min(VISIBLE_ROWS)) as f32 * ROW_HEIGHT))
    }

    fn file_actions(self) -> impl IntoElement {
        let i18n = self.i18n;
        view()
            .flex_row()
            .gap_2()
            .child(
                ui::action(
                    move || tr(i18n, "Add files…", "添加文件…"),
                    move || self.pick_more(),
                )
                .disabled(move || self.busy.get() || self.picking.get()),
            )
            .child(
                ui::action(
                    move || tr(i18n, "Remove selected file", "移除选中文件"),
                    move || self.remove_selected(),
                )
                .disabled(move || self.busy.get() || self.loading.get()),
            )
    }

    fn mode_picker(self) -> impl IntoElement {
        let i18n = self.i18n;
        radio_group().flex_row().flex_wrap().gap_2().children(
            [
                (Mode::Media, "Photos and videos", "图片和视频"),
                (Mode::Files, "Files", "文件"),
                (Mode::Archive, "ZIP archive", "ZIP 压缩包"),
            ]
            .into_iter()
            .map(move |(choice, english, chinese)| {
                let checked = move || self.mode_value() == choice;
                radio()
                    .checked(checked)
                    .disabled(move || {
                        self.busy.get()
                            || self.preparing_edit.get()
                            || (choice == Mode::Archive && self.sources.with(Vec::len) < 2)
                    })
                    .on_change(move |selected: &bool| {
                        if *selected {
                            self.mode.set(choice);
                        }
                    })
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .app_class("disabled:opacity-40")
                    .hover(|s| s.bg(move || theme().accent))
                    .focus(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
                    .child(radio_mark(checked))
                    .child(text(move || tr(i18n, english, chinese)).text_sm())
            }),
        )
    }

    fn options(self) -> impl IntoElement {
        let i18n = self.i18n;
        let busy = move || self.busy.get();
        view()
            .gap_3()
            .child(when(
                move || self.mode_value() == Mode::Archive,
                move || {
                    ui::field()
                        .masked(true)
                        .label(move || tr(i18n, "Password (optional)", "密码（可选）"))
                        .placeholder(move || {
                            tr(i18n, "Password (optional)", "密码（可选）").to_owned()
                        })
                        .readonly(self.busy)
                        .value(move || self.password.get())
                        .on_change(move |value: &str| self.password.set(value.to_owned()))
                },
            ))
            .child(when(
                move || self.mode_value() == Mode::Media,
                move || {
                    option(
                        move || tr(i18n, "Compress images", "压缩图片"),
                        self.compress,
                        busy,
                    )
                },
            ))
            .child(when(
                move || {
                    self.mode_value() == Mode::Media
                        && self
                            .prepared
                            .with(|rows| rows.iter().any(|item| item.kind == "IMAGE"))
                },
                move || {
                    ui::field()
                        .multiline(true)
                        .max_h_24()
                        .label(move || tr(i18n, "Caption", "图片说明"))
                        .placeholder(move || tr(i18n, "Add a caption…", "添加图片说明…").to_owned())
                        .readonly(self.busy)
                        .value(move || self.caption.get())
                        .on_change(move |value: &str| self.caption.set(value.to_owned()))
                        .on_paste(move |event: &nsh_core::PasteEvent| {
                            self.append(preparation::pasted_sources(event))
                        })
                },
            ))
            .child(option(
                move || tr(i18n, "Send without notification", "静音发送"),
                self.silent,
                busy,
            ))
    }

    fn status(self) -> impl IntoElement {
        let i18n = self.i18n;
        view()
            .gap_2()
            .child(when(self.loading, move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::busy_indicator(
                        move || self.loading.get(),
                        move || tr(i18n, "Preparing files…", "正在准备文件…").into(),
                    ))
                    .child(text(move || tr(i18n, "Preparing files…", "正在准备文件…")).text_sm())
            }))
            .child(when(self.input_rejected, move || {
                failure(move || {
                    tr(
                        i18n,
                        "Couldn’t add these files. Try again.",
                        "无法添加这些文件，请重试。",
                    )
                })
            }))
            .child(when(
                move || self.prepare_error.with(Option::is_some),
                move || {
                    view()
                        .gap_2()
                        .child(failure(move || {
                            tr(
                                i18n,
                                "Couldn’t prepare this file. Remove it or send as files.",
                                "无法准备此文件。可以将其移除，或改为文件发送。",
                            )
                        }))
                        .child(
                            text(move || self.prepare_error.get().unwrap_or_default())
                                .text_xs()
                                .text_color(move || theme().muted_foreground),
                        )
                        .child(ui::action(
                            move || i18n.text(Key::Retry),
                            move || self.retry.update(|value| *value += 1),
                        ))
                },
            ))
            .child(when(self.send_failed, move || {
                failure(move || {
                    tr(
                        i18n,
                        "Some files couldn’t be sent. The remaining files are still here.",
                        "部分文件发送失败，尚未发送的文件已保留。",
                    )
                })
            }))
    }

    fn footer(self) -> impl IntoElement {
        let i18n = self.i18n;
        view()
            .flex_none()
            .flex_row()
            .justify_end()
            .gap_2()
            .child(
                ui::action(move || i18n.text(Key::Cancel), move || self.dismiss())
                    .disabled(move || self.busy.get() || self.picking.get()),
            )
            .child(
                ui::primary()
                    .disabled(move || {
                        self.busy.get()
                            || self.picking.get()
                            || self.loading.get()
                            || self.prepare_error.with(Option::is_some)
                            || self.prepared.with(Vec::is_empty)
                    })
                    .child(text(move || {
                        i18n.text(if self.busy.get() {
                            Key::Sending
                        } else {
                            Key::Send
                        })
                    }))
                    .on_press(move || self.send()),
            )
    }

    fn image_editor(self) -> impl IntoElement {
        let id = self.editing.get_untracked();
        let source = self.sources.with_untracked(|sources| {
            id.and_then(|id| sources.iter().find(|source| source.id == id).cloned())
        });
        untracked(|| match source {
            Some(source) if source.original.is_some() => {
                let id = source.id;
                super::image_editor::screen(
                    self.media.with(|media| media.clone()),
                    source.original.expect("editor source checked"),
                    source.edits,
                    self.editor,
                    move |edits, item| {
                        batch(|| {
                            self.sources.update(|sources| {
                                if let Some(row) = sources.iter_mut().find(|row| row.id == id) {
                                    row.edits = edits;
                                    row.edited = Some(item);
                                }
                            });
                            self.editing.set(None);
                        })
                    },
                    move || self.editing.set(None),
                )
                .into_element()
            }
            _ => view().hidden().into_element(),
        })
    }
}

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
        .indicator_class(
            "w-4 h-4 items-center justify-center border border-input rounded-sm checked:bg-primary",
        )
        .indicator(
            ui::icon(ui::Icon::Check)
                .size_3()
                .text_color(move || theme().primary_foreground)
                .style_when(move || !checked.get(), |style| style.invisible()),
        )
        .child(text(title).text_sm())
}

fn radio_mark(checked: impl Fn() -> bool + Copy + 'static) -> impl IntoElement {
    view()
        .size_4()
        .rounded_full()
        .border_1()
        .border_color(move || {
            if checked() {
                theme().primary
            } else {
                theme().input
            }
        })
        .items_center()
        .justify_center()
        .child(
            view()
                .size_2()
                .rounded_full()
                .bg(move || theme().primary)
                .style_when(move || !checked(), |style| style.invisible()),
        )
}

fn size_label(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes < MIB {
        format!("{} KB", bytes.div_ceil(1024))
    } else {
        format!("{:.1} MB", bytes as f64 / MIB as f64)
    }
}
