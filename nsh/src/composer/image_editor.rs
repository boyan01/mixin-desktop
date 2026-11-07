//! Disposable image-editing presentation. The media client owns pixel transforms and files.
mod geometry;
mod preview;
mod surface;

use crate::runtime;
use geometry::{Crop, Drag};
use mixin_desktop_api::{ImageEdit, MediaClient, PreparedAttachment};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Crop,
    Brush,
    Eraser,
}

#[derive(Clone, Copy)]
struct Editor {
    source: Stored<PreparedAttachment>,
    media: Stored<Arc<MediaClient>>,
    edits: Signal<Vec<ImageEdit>>,
    transforms: Memo<Vec<geometry::Transform>>,
    redo: Signal<Vec<(usize, ImageEdit)>>,
    drawing_backup: Stored<Vec<ImageEdit>>,
    crop: Signal<Crop>,
    ratio: Signal<Option<f32>>,
    tool: Signal<Tool>,
    color: Signal<[u8; 4]>,
    stroke: Signal<Vec<(u32, u32)>>,
    drag: Signal<Option<Drag>>,
    viewport: Signal<(f32, f32)>,
    focused: Signal<bool>,
    loading: Signal<bool>,
    failed: Signal<bool>,
}

pub(super) fn screen(
    media: Arc<MediaClient>,
    source: PreparedAttachment,
    mut edits: Vec<ImageEdit>,
    presence: crate::motion::Presence,
    save: impl Fn(Vec<ImageEdit>, PreparedAttachment) + Copy + 'static,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let dimensions = (
        source.width.unwrap_or(1).max(1) as u32,
        source.height.unwrap_or(1).max(1) as u32,
    );
    let crop = if let Some(ImageEdit::Crop {
        x,
        y,
        width,
        height,
    }) = edits.last()
    {
        let crop = Crop {
            x: *x as f32,
            y: *y as f32,
            width: *width as f32,
            height: *height as f32,
        };
        edits.pop();
        crop
    } else {
        let edited = geometry::transforms(&edits, dimensions.0, dimensions.1)
            .last()
            .expect("initial transform")
            .1;
        Crop::full(edited.0, edited.1)
    };
    let edits = signal(edits);
    let transforms = computed(move || {
        edits.with(|edits| geometry::transforms(edits, dimensions.0, dimensions.1))
    });
    let editor = Editor {
        source: stored(source),
        media: stored(media),
        edits,
        transforms,
        redo: signal(Vec::new()),
        drawing_backup: stored(Vec::new()),
        crop: signal(crop),
        ratio: signal(None),
        tool: signal(Tool::Crop),
        color: signal([232, 77, 61, 255]),
        stroke: signal(Vec::new()),
        drag: signal(None),
        viewport: signal((1.0, 1.0)),
        focused: signal(false),
        loading: signal(false),
        failed: signal(false),
    };
    let done = move || {
        if !presence.interactive() || editor.loading.get_untracked() {
            return;
        }
        if editor.tool.get_untracked() != Tool::Crop {
            editor.tool.set(Tool::Crop);
            return;
        }
        let (width, height) = editor.dimensions();
        let crop = editor.crop.get_untracked().pixels(width, height);
        let mut edits = editor.edits.get_untracked();
        if crop != (0, 0, width, height) {
            edits.push(ImageEdit::Crop {
                x: crop.0,
                y: crop.1,
                width: crop.2,
                height: crop.3,
            });
        }
        let media = editor.media.with(|media| media.clone());
        let source = editor.source.with(|source| source.clone());
        let recipe = edits.clone();
        editor.loading.set(true);
        editor.failed.set(false);
        spawn(async move {
            let result = runtime::run(async move {
                let result = media.edit_image(source.path.clone(), recipe).await;
                drop(source);
                result
            })
            .await;
            if !presence.interactive() {
                return;
            }
            match result {
                Ok(item) => save(edits, item),
                Err(_) => {
                    editor.failed.set(true);
                    editor.loading.set(false);
                }
            }
        });
    };
    let i18n = crate::i18n::current();
    let initial = stored((editor.edits.get_untracked(), crop));
    let discarding = signal(false);
    let cancel = move || {
        if !presence.interactive() {
            return;
        }
        if editor.tool.get_untracked() != Tool::Crop {
            editor
                .edits
                .set(editor.drawing_backup.with(|edits| edits.clone()));
            editor.redo.set(Vec::new());
            editor.stroke.set(Vec::new());
            editor.tool.set(Tool::Crop);
        } else {
            if discarding.get_untracked() {
                return;
            }
            let changed = initial.with(|initial| {
                initial.0 != editor.edits.get_untracked()
                    || initial.1 != editor.crop.get_untracked()
            });
            if !changed {
                close();
                return;
            }
            discarding.set(true);
            spawn(async move {
                let result = nsh_base::alert_dialog()
                    .title(i18n.tr("Discard image edits?", "放弃图片修改？"))
                    .description(i18n.tr(
                        "The image in the send preview will stay unchanged.",
                        "发送预览中的图片会保留原样。",
                    ))
                    .cancel("cancel", i18n.tr("Cancel", "取消"))
                    .destructive("discard", i18n.tr("Discard", "放弃修改"))
                    .show()
                    .await;
                discarding.set(false);
                if result.is_ok_and(|answer| answer.action.as_deref() == Some("discard")) {
                    close();
                }
            });
        }
    };
    on_key("mod+z", move |_| {
        if presence.interactive() {
            editor.undo();
        }
    });
    on_key("mod+shift+z", move |_| {
        if presence.interactive() {
            editor.redo();
        }
    });
    surface::screen(editor, presence, done, cancel)
}

impl Editor {
    fn dimensions(self) -> (u32, u32) {
        self.transforms
            .with(|transforms| transforms.last().expect("initial transform").1)
    }
    fn source_dimensions(self) -> (u32, u32) {
        self.source.with(|item| {
            (
                item.width.unwrap_or(1).max(1) as u32,
                item.height.unwrap_or(1).max(1) as u32,
            )
        })
    }
    fn scale(self) -> f32 {
        let (width, height) = self.dimensions();
        let viewport = self.viewport.get();
        (viewport.0 / width as f32)
            .min(viewport.1 / height as f32)
            .clamp(0.001, 1.0)
    }
    fn edit(self, edit: ImageEdit) {
        if self.loading.get_untracked() {
            return;
        }
        self.failed.set(false);
        self.transform_crop(&edit, false);
        self.redo.set(Vec::new());
        self.edits.update(|edits| edits.push(edit));
    }
    fn transform_crop(self, edit: &ImageEdit, undo: bool) {
        let (width, height) = self.dimensions();
        match edit {
            ImageEdit::RotateClockwise | ImageEdit::RotateCounterClockwise => {
                let undo = undo != matches!(edit, ImageEdit::RotateCounterClockwise);
                self.crop.update(|crop| {
                    *crop = if undo {
                        Crop {
                            x: crop.y,
                            y: width as f32 - crop.x - crop.width,
                            width: crop.height,
                            height: crop.width,
                        }
                    } else {
                        Crop {
                            x: height as f32 - crop.y - crop.height,
                            y: crop.x,
                            width: crop.height,
                            height: crop.width,
                        }
                    }
                });
                self.ratio
                    .update(|ratio| *ratio = ratio.map(|ratio| 1.0 / ratio));
            }
            ImageEdit::FlipHorizontal => self
                .crop
                .update(|crop| crop.x = width as f32 - crop.x - crop.width),
            _ => {}
        }
    }
    fn has_drawing(self) -> bool {
        self.edits.with(|edits| {
            edits.iter().any(|edit| {
                matches!(
                    edit,
                    ImageEdit::Stroke { .. } | ImageEdit::EraseStroke { .. }
                )
            })
        })
    }
    fn undo(self) {
        if self.loading.get_untracked() {
            return;
        }
        let selected = self.edits.with_untracked(|edits| {
            let index = if self.tool.get_untracked() == Tool::Crop {
                edits.len().checked_sub(1)
            } else {
                edits.iter().rposition(|edit| {
                    matches!(
                        edit,
                        ImageEdit::Stroke { .. } | ImageEdit::EraseStroke { .. }
                    )
                })
            }?;
            Some((index, edits[index].clone()))
        });
        if let Some((index, edit)) = selected {
            self.failed.set(false);
            self.transform_crop(&edit, true);
            self.edits.update(|edits| {
                edits.remove(index);
            });
            self.redo.update(|redo| redo.push((index, edit)));
        }
    }
    fn redo(self) {
        if self.loading.get_untracked() {
            return;
        }
        let mut edit = None;
        self.redo.update(|redo| edit = redo.pop());
        if let Some((index, edit)) = edit {
            self.failed.set(false);
            self.transform_crop(&edit, false);
            self.edits
                .update(|edits| edits.insert(index.min(edits.len()), edit));
        }
    }
    fn enter_drawing(self) {
        self.drawing_backup.replace(self.edits.get_untracked());
        self.redo.set(Vec::new());
        self.tool.set(Tool::Brush);
    }
    fn reset(self) {
        if self.loading.get_untracked() {
            return;
        }
        let dimensions = self.source.with(|item| {
            (
                item.width.unwrap_or(1).max(1) as u32,
                item.height.unwrap_or(1).max(1) as u32,
            )
        });
        batch(|| {
            self.failed.set(false);
            self.edits.set(Vec::new());
            self.redo.set(Vec::new());
            self.crop.set(Crop::full(dimensions.0, dimensions.1));
            self.ratio.set(None);
        });
    }
    fn nudge_crop(self, dx: f32, dy: f32, resize: bool) {
        if self.loading.get_untracked() {
            return;
        }
        let (width, height) = self.dimensions();
        self.crop.update(|crop| {
            if resize {
                let next_width = (crop.width + dx)
                    .clamp(1.0_f32.min(width as f32 - crop.x), width as f32 - crop.x);
                let next_height = (crop.height + dy)
                    .clamp(1.0_f32.min(height as f32 - crop.y), height as f32 - crop.y);
                if let Some(ratio) = self.ratio.get_untracked() {
                    crop.width = (if dx != 0.0 {
                        next_width
                    } else {
                        next_height * ratio
                    })
                    .min(width as f32 - crop.x)
                    .min((height as f32 - crop.y) * ratio)
                    .max(1.0);
                    crop.height = crop.width / ratio;
                } else {
                    crop.width = next_width;
                    crop.height = next_height;
                }
            } else {
                crop.x = (crop.x + dx).clamp(0.0, width as f32 - crop.width);
                crop.y = (crop.y + dy).clamp(0.0, height as f32 - crop.height);
            }
        });
    }
    fn pointer(self, position: Point<Points>) -> (f32, f32) {
        let scale = self.scale();
        let (width, height) = self.dimensions();
        if self.tool.get_untracked() == Tool::Crop {
            (
                (position.x.0 / scale).clamp(0.0, width as f32),
                (position.y.0 / scale).clamp(0.0, height as f32),
            )
        } else {
            (
                (position.x.0 / scale).clamp(0.0, width.saturating_sub(1) as f32),
                (position.y.0 / scale).clamp(0.0, height.saturating_sub(1) as f32),
            )
        }
    }
    fn start(self, event: &ResponderEvent) {
        if self.loading.get_untracked() {
            return;
        }
        let point = self.pointer(event.location);
        if self.tool.get_untracked() == Tool::Crop {
            let crop = self.crop.get_untracked();
            self.drag.set(
                geometry::grip(crop, point, 12.0 / self.scale()).map(|grip| Drag {
                    crop,
                    origin: point,
                    grip,
                }),
            );
        } else {
            self.stroke
                .set(vec![(point.0.round() as u32, point.1.round() as u32)]);
        }
    }
    fn moved(self, event: &ResponderEvent) {
        if self.loading.get_untracked() {
            return;
        }
        let point = self.pointer(event.location);
        if self.tool.get_untracked() == Tool::Crop {
            if let Some(drag) = self.drag.get_untracked() {
                let dimensions = self.dimensions();
                self.crop.set(geometry::drag(
                    drag,
                    point,
                    dimensions.0,
                    dimensions.1,
                    self.ratio.get_untracked(),
                ));
            }
        } else {
            let point = (point.0.round() as u32, point.1.round() as u32);
            self.stroke.update(|points| {
                if points.last() != Some(&point) && points.len() < 65536 {
                    points.push(point);
                }
            });
        }
    }
    fn end(self) {
        self.drag.set(None);
        let mut points = Vec::new();
        self.stroke.update(|stroke| points = std::mem::take(stroke));
        if points.is_empty() {
            return;
        }
        self.edit(if self.tool.get_untracked() == Tool::Eraser {
            ImageEdit::EraseStroke { points, width: 11 }
        } else {
            ImageEdit::Stroke {
                points,
                color: self.color.get_untracked(),
                width: 11,
            }
        });
    }
}

fn raster_color(color: [u8; 4]) -> nsh_core::color::Rgba {
    nsh_core::color::Rgba {
        r: f32::from(color[0]) / 255.0,
        g: f32::from(color[1]) / 255.0,
        b: f32::from(color[2]) / 255.0,
        a: f32::from(color[3]) / 255.0,
    }
}
