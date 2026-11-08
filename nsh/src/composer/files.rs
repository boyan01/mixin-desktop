use mixin_desktop_api::{AccountClient, MediaClient, MessageListView, PreparedAttachment};
use nsh_core::prelude::*;
use std::{rc::Rc, sync::Arc};

use super::{AttachmentSource, clear_quote, quote_id, tr};
use crate::media_client;
use crate::{
    i18n::{self, I18n, Key},
    motion::Presence,
    runtime,
    ui::{self, Typography as _},
};

const MEDIA_EXTENSIONS: [&str; 13] = [
    "jpg", "jpeg", "png", "gif", "webp", "heic", "bmp", "mp4", "mov", "m4v", "webm", "mkv", "avi",
];

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mode {
    Auto,
    Media,
    Files,
    Archive,
}

pub(super) mod preparation;
use preparation::{Cache, Source, prepare_source, prepare_sources};

struct Outcome {
    accepted: usize,
    upload_failed: bool,
    finished: bool,
    caption_sent: bool,
}

/// What the tray shares with the composer that owns it.
#[derive(Clone, Copy)]
struct Context {
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    quote: Signal<Option<MessageListView>>,
    failed: Signal<bool>,
    navigation: crate::navigation::ChatNavigation,
}

/// The files waiting under the composer's input, and everything that sends them.
#[derive(Clone, Copy)]
pub(crate) struct Tray(Preview);

#[derive(Clone, Copy)]
struct Preview {
    ctx: Context,
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
    // The composer's draft: it is the caption of the first picture.
    caption: Memo<String>,
    clear_caption: Stored<Rc<dyn Fn()>>,
    password: Signal<String>,
    input_rejected: Signal<bool>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    picking: Signal<bool>,
    send_failed: Signal<bool>,
    prepare_error: Signal<Option<String>>,
    retry: Signal<u64>,
}

pub(crate) fn mount(
    account: Arc<AccountClient>,
    conversation: String,
    failed: Signal<bool>,
    quote: Signal<Option<MessageListView>>,
    caption: Memo<String>,
    clear_caption: Rc<dyn Fn()>,
) -> Tray {
    let editing = signal(None::<u64>);
    let preview = Preview {
        ctx: Context {
            account: stored(account),
            conversation: stored(conversation),
            quote,
            failed,
            navigation: use_context::<crate::navigation::ChatNavigation>()
                .expect("Home provides chat navigation"),
        },
        i18n: i18n::current(),
        media: stored(media_client()),
        owner: Owner::current(),
        sources: signal_with(Vec::new(), SignalEquals::Never),
        next_id: stored(0),
        cache: stored(Cache::default()),
        prepared: signal_with(Vec::new(), SignalEquals::Never),
        selected: signal(0),
        editing,
        editor: Presence::fade(move || editing.with(Option::is_some)),
        preparing_edit: signal(false),
        mode: signal(Mode::Auto),
        compress: signal(true),
        silent: signal(false),
        caption,
        clear_caption: stored(clear_caption),
        password: signal(String::new()),
        input_rejected: signal(false),
        loading: signal(false),
        busy: signal(false),
        picking: signal(false),
        send_failed: signal(false),
        prepare_error: signal(None),
        retry: signal(0),
    };
    preview.prepare();
    // With focus outside the input, paste still adds the clipboard's files.
    on_key_with(
        "mod+v",
        KeyOptions {
            when: nsh_core::keymap::When::NotEditing,
            ..Default::default()
        },
        move |_| {
            if preview.busy.get_untracked() || preview.editor.mounted() {
                return;
            }
            preview.owner.run(|| {
                spawn(async move { preview.append(preparation::clipboard_sources().await) });
            });
        },
    );
    Tray(preview)
}

impl Tray {
    /// Whether files wait in the tray: the composer then sends them, with its text as caption.
    pub fn active(self) -> bool {
        self.0.sources.with(|sources| !sources.is_empty())
    }

    pub fn busy(self) -> bool {
        self.0.busy.get()
    }

    /// Whether the tray can send now: prepared, and nothing else holds it.
    pub fn ready(self) -> bool {
        self.0.ready()
    }

    pub fn add(self, inputs: Vec<AttachmentSource>) {
        self.0.append(inputs);
    }

    /// Opens the file dialog. A tray with nothing in it sends what it gets as that kind.
    pub fn pick(self, media: bool) {
        if self.0.sources.with_untracked(Vec::is_empty) {
            self.0
                .mode
                .set(if media { Mode::Media } else { Mode::Files });
        }
        self.0.pick_more(media);
    }

    pub fn send(self, silent: bool) {
        self.0.silent.set(silent);
        self.0.send();
    }

    pub fn view(self) -> impl IntoElement {
        self.0.tray()
    }

    /// The image editor, over the whole window while an image is edited.
    pub fn editor(self) -> impl IntoElement {
        let preview = self.0;
        when(
            move || preview.editor.mounted(),
            move || preview.image_editor(),
        )
    }
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

    fn append(self, inputs: Vec<AttachmentSource>) {
        if inputs.is_empty() {
            return;
        }
        if self.busy.get_untracked() {
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

    fn pick_more(self, media: bool) {
        if self.busy.get_untracked() || self.picking.get_untracked() {
            return;
        }
        self.picking.set(true);
        let i18n = self.i18n;
        self.owner.run(|| {
            spawn(async move {
                let dialog = nsh_core::file_dialog::OpenDialog::files().multiple(true);
                let dialog = if media {
                    dialog.filter(nsh_core::file_dialog::FileFilter::new(
                        tr(i18n, "Pictures and videos", "图片和视频"),
                        MEDIA_EXTENSIONS,
                    ))
                } else {
                    dialog
                };
                let result = pick_paths(dialog).await;
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

    fn remove_at(self, index: usize) {
        if self.busy.get_untracked() || self.loading.get_untracked() {
            return;
        }
        self.sources.update(|sources| {
            if index < sources.len() {
                sources.remove(index);
            }
        });
        match self.sources.with_untracked(Vec::len) {
            0 => batch(|| {
                self.selected.set(0);
                self.mode.set(Mode::Auto);
                self.password.set(String::new());
                self.silent.set(false);
            }),
            remaining => {
                let selected = self.selected.get_untracked();
                let selected = if index < selected {
                    selected - 1
                } else {
                    selected
                };
                self.selected.set(selected.min(remaining - 1));
            }
        }
    }

    /// Whether the files are prepared and nothing else holds the dialog.
    fn ready(self) -> bool {
        !(self.busy.get()
            || self.picking.get()
            || self.loading.get()
            || self.prepare_error.with(Option::is_some)
            || self.prepared.with(Vec::is_empty))
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
        if self.busy.get_untracked() || !self.ready() {
            return;
        }
        let client = self.ctx.account.with(|account| account.clone());
        let id = self.ctx.conversation.with(|id| id.clone());
        let media = self.media.with(|media| media.clone());
        let items = self.prepared.get_untracked();
        let source_guards = self.sources.get_untracked();
        let archive = untracked(|| self.mode_value()) == Mode::Archive;
        let password = self.password.get_untracked();
        let mut caption = Some(self.caption.get_untracked()).filter(|text| !text.trim().is_empty());
        let silent = self.silent.get_untracked();
        let sent_quote = quote_id(self.ctx.quote);
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
                        self.ctx.failed.set(outcome.upload_failed);
                        clear_quote(self.ctx.quote, sent_quote);
                        self.ctx.navigation.show_latest();
                        if outcome.caption_sent {
                            self.clear_caption.with(|clear| clear());
                        }
                        batch(|| {
                            self.sources.set(Vec::new());
                            self.selected.set(0);
                            self.mode.set(Mode::Auto);
                            self.password.set(String::new());
                            self.silent.set(false);
                        });
                    }
                    Ok(outcome) => {
                        self.ctx.failed.set(outcome.upload_failed);
                        if outcome.caption_sent {
                            self.clear_caption.with(|clear| clear());
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

    fn tray(self) -> impl IntoElement {
        view()
            .px_2()
            .pt_2()
            .gap_2()
            .style_when(
                move || self.sources.with(Vec::is_empty),
                |style| style.hidden(),
            )
            .child(self.thumbnails())
            .child(self.options())
            .child(self.status())
    }

    fn open_selected(self, index: usize) {
        self.selected.set(index);
        if self
            .selected_item()
            .is_some_and(|item| item.kind == "IMAGE")
        {
            self.edit_selected();
        }
    }

    /// The files as a grid of thumbnails: select one to preview it, remove it with its
    /// corner button, and add more with the last cell.
    fn thumbnails(self) -> impl IntoElement {
        let i18n = self.i18n;
        let cell = ui::thumbnail_grid_size;
        scroll().horizontal().scrollbar(false).child(
            view()
                .flex_row()
                .gap_2()
                .child(each(
                    move || {
                        self.sources
                            .with(|rows| rows.iter().map(|source| source.id).collect::<Vec<_>>())
                    },
                    |id| *id,
                    move |id, _| {
                        let id = id.with(|id| *id);
                        let index = move || {
                            self.sources
                                .with(|rows| rows.iter().position(|source| source.id == id))
                        };
                        let item = move || {
                            self.prepared.with(|_| ());
                            self.cache.with(|cache| cache.item(id))
                        };
                        view()
                            .relative()
                            .size(cell())
                            .flex_shrink_0()
                            .rounded(ui::radius_thumb())
                            .overflow_hidden()
                            .bg(ui::surface_list)
                            .child(
                                ui::command()
                                    .absolute()
                                    .inset_0()
                                    .p_0()
                                    .disabled(self.busy)
                                    .accessibility_label(move || {
                                        Some(
                                            item().map(|item| item.name).unwrap_or_default().into(),
                                        )
                                    })
                                    .on_press(move || {
                                        if let Some(index) = index() {
                                            self.open_selected(index);
                                        }
                                    })
                                    .child(dynamic(move || {
                                        let current = item();
                                        untracked(|| match current {
                                            Some(item) if item.kind == "IMAGE" => image(item.path)
                                                .size_full()
                                                .object_fit(ResizeMode::Cover)
                                                .into_element(),
                                            Some(item) => view()
                                                .size_full()
                                                .items_center()
                                                .justify_center()
                                                .p_1()
                                                .gap_1()
                                                .child(ui::icon(if item.kind == "VIDEO" {
                                                    ui::Icon::Play
                                                } else {
                                                    ui::Icon::Save
                                                }))
                                                .child(
                                                    text(item.name)
                                                        .text_role(ui::TextRole::Caption)
                                                        .number_of_lines(2),
                                                )
                                                .child(
                                                    text(size_label(item.size))
                                                        .text_role(ui::TextRole::Caption)
                                                        .text_color(ui::fg_secondary),
                                                )
                                                .into_element(),
                                            None => ui::busy_indicator(
                                                || true,
                                                move || tr(i18n, "Preparing…", "准备中…").into(),
                                            )
                                            .into_element(),
                                        })
                                    })),
                            )
                            .child(
                                ui::icon_button()
                                    .absolute()
                                    .top_1()
                                    .right_1()
                                    .size_5()
                                    .px_0()
                                    .py_0()
                                    .rounded_full()
                                    .bg(ui::media_overlay)
                                    .disabled(move || self.busy.get() || self.loading.get())
                                    .accessibility_label(move || {
                                        Some(tr(i18n, "Remove", "移除").into())
                                    })
                                    .on_press(move || {
                                        if let Some(index) = index() {
                                            self.remove_at(index);
                                        }
                                    })
                                    .child(
                                        ui::icon(ui::Icon::Close)
                                            .size(ui::icon_sm())
                                            .text_color(ui::media_overlay_foreground),
                                    ),
                            )
                    },
                ))
                .child(
                    ui::command()
                        .size(cell())
                        .flex_shrink_0()
                        .rounded(ui::radius_thumb())
                        .bg(ui::surface_list)
                        .disabled(move || self.busy.get() || self.picking.get())
                        .accessibility_label(move || {
                            Some(tr(i18n, "Add files…", "添加文件…").into())
                        })
                        .on_press(move || self.pick_more(false))
                        .child(ui::icon(ui::Icon::Add).size(ui::icon_lg())),
                ),
        )
    }

    fn options(self) -> impl IntoElement {
        let i18n = self.i18n;
        let busy = move || self.busy.get();
        view()
            .gap_2()
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap_3()
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
                        move || self.sources.with(Vec::len) >= 2,
                        move || {
                            ui::checkbox(
                                Some(move || tr(i18n, "ZIP archive", "ZIP 压缩包")),
                                move || self.mode.get() == Mode::Archive,
                                busy,
                                move |value| {
                                    self.mode
                                        .set(if value { Mode::Archive } else { Mode::Auto })
                                },
                            )
                        },
                    )),
            )
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
                    .child(
                        text(move || tr(i18n, "Preparing files…", "正在准备文件…"))
                            .text_role(ui::TextRole::Body),
                    )
            }))
            .child(when(self.input_rejected, move || {
                ui::error_text(move || {
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
                        .child(ui::error_text(move || {
                            tr(
                                i18n,
                                "Couldn’t prepare this file. Remove it or send as files.",
                                "无法准备此文件。可以将其移除，或改为文件发送。",
                            )
                        }))
                        .child(
                            text(move || self.prepare_error.get().unwrap_or_default())
                                .text_role(ui::TextRole::Caption)
                                .text_color(ui::fg_secondary),
                        )
                        .child(ui::action(
                            move || i18n.text(Key::Retry),
                            move || self.retry.update(|value| *value += 1),
                        ))
                },
            ))
            .child(when(self.send_failed, move || {
                ui::error_text(move || {
                    tr(
                        i18n,
                        "Some files couldn’t be sent. The remaining files are still here.",
                        "部分文件发送失败，尚未发送的文件已保留。",
                    )
                })
            }))
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
    ui::checkbox(
        Some(title),
        move || checked.get(),
        disabled,
        move |value| checked.set(value),
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
