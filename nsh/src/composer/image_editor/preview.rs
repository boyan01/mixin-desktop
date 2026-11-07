//! Native canvas composition keeps drawing responsive; encoding happens only on Done.
use super::{Editor, Tool, raster_color};
use crate::ui;
use mixin_desktop_api::ImageEdit;
use nsh_core::{
    canvas::{Composite, LineCap},
    prelude::*,
};

pub(super) fn foreground(editor: Editor) -> Painter {
    let scale = editor.scale();
    let crop = editor.crop.get();
    let dimensions = editor.dimensions();
    let (width, height) = (dimensions.0 as f32 * scale, dimensions.1 as f32 * scale);
    let mut painter = paint_annotations(editor);
    if editor.tool.get() == Tool::Crop {
        let (left, top, right, bottom) = (
            crop.x * scale,
            crop.y * scale,
            (crop.x + crop.width) * scale,
            (crop.y + crop.height) * scale,
        );
        painter = painter
            .fill_color(color(ui::fg_primary()).opacity(0.3))
            .fill_rect(0.0, 0.0, width, top)
            .fill_rect(0.0, bottom, width, (height - bottom).max(0.0))
            .fill_rect(0.0, top, left, bottom - top)
            .fill_rect(right, top, (width - right).max(0.0), bottom - top)
            .stroke_color(color(ui::fg_primary()))
            .line_width(3.0)
            .stroke_rect(crop.bounds(scale))
            .stroke_color(color(ui::surface_pane()))
            .line_width(1.0)
            .stroke_rect(crop.bounds(scale));
        for divider in [1.0 / 3.0, 2.0 / 3.0] {
            let x = left + (right - left) * divider;
            let y = top + (bottom - top) * divider;
            painter = painter
                .begin_path()
                .move_to(x, top)
                .line_to(x, bottom)
                .move_to(left, y)
                .line_to(right, y)
                .stroke();
        }
        // Eight-point grips describe raster selection, with a wider hit tolerance.
        for (x, y) in [(left, top), (right, top), (left, bottom), (right, bottom)] {
            painter =
                painter
                    .fill_color(color(ui::surface_pane()))
                    .fill_rect(x - 4.0, y - 4.0, 8.0, 8.0);
        }
    }
    painter
}

pub(super) fn image(editor: Editor) -> Painter {
    let dimensions = editor.source_dimensions();
    let matrix = editor
        .transforms
        .with(|transforms| transforms.last().expect("initial transform").0);
    let scale = editor.scale();
    let source = editor.source.with(|source| source.path.clone());
    Painter::new()
        .scale(scale, scale)
        .transform(matrix)
        .draw_image(
            &source,
            Bounds::new(
                point(Points::ZERO, Points::ZERO),
                // Source pixels, in the scaled painter's own coordinates.
                size(px(dimensions.0 as f32), px(dimensions.1 as f32)),
            ),
        )
}

fn paint_annotations(editor: Editor) -> Painter {
    let scale = editor.scale();
    let mut painter = editor.edits.with(|edits| {
        let transforms = editor.transforms.get();
        let final_matrix = transforms.last().expect("initial transform").0;
        let mut painter = Painter::new();
        for (index, edit) in edits.iter().enumerate() {
            let (points, color, width) = match edit {
                ImageEdit::Stroke {
                    points,
                    color,
                    width,
                } => (points, Some(*color), *width),
                ImageEdit::EraseStroke { points, width } => (points, None, *width),
                _ => continue,
            };
            let suffix = super::geometry::multiply(
                final_matrix,
                super::geometry::inverse(transforms[index].0),
            );
            painter = paint_stroke(
                painter.save().scale(scale, scale).transform(suffix),
                points,
                color,
                width,
            )
            .restore();
        }
        painter
    });
    let points = editor.stroke.get();
    if !points.is_empty() {
        let color = (editor.tool.get() != Tool::Eraser).then(|| editor.color.get());
        painter = paint_stroke(painter.save().scale(scale, scale), &points, color, 11).restore();
    }
    painter.composite(Composite::SourceOver)
}

fn paint_stroke(
    mut painter: Painter,
    points: &[(u32, u32)],
    color: Option<[u8; 4]>,
    width: u32,
) -> Painter {
    let Some(first) = points.first() else {
        return painter;
    };
    painter = painter
        .composite(if color.is_some() {
            Composite::SourceOver
        } else {
            Composite::DestinationOut
        })
        .stroke_color(raster_color(color.unwrap_or([255; 4])))
        .line_width(width as f32)
        .line_cap(LineCap::Round)
        .begin_path()
        .move_to(first.0 as f32 + 0.5, first.1 as f32 + 0.5);
    for point in points.iter().skip(1) {
        painter = painter.line_to(point.0 as f32 + 0.5, point.1 as f32 + 0.5);
    }
    if points.len() == 1 {
        painter = painter.line_to(first.0 as f32 + 0.51, first.1 as f32 + 0.5);
    }
    painter.stroke()
}
