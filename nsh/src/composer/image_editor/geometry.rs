use nsh_core::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Crop {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Crop {
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        }
    }
    pub fn centered(width: u32, height: u32, ratio: Option<f32>) -> Self {
        let Some(ratio) = ratio else {
            return Self::full(width, height);
        };
        let crop_width = (width as f32).min(height as f32 * ratio).max(1.0);
        let crop_height = (crop_width / ratio).max(1.0).min(height as f32);
        Self {
            x: (width as f32 - crop_width) / 2.0,
            y: (height as f32 - crop_height) / 2.0,
            width: crop_width,
            height: crop_height,
        }
    }
    pub fn pixels(self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let x = (self.x.round() as u32).min(width.saturating_sub(1));
        let y = (self.y.round() as u32).min(height.saturating_sub(1));
        (
            x,
            y,
            (self.width.round() as u32).clamp(1, width - x),
            (self.height.round() as u32).clamp(1, height - y),
        )
    }
    /// The crop in view coordinates, measured from the image and its scale.
    pub fn bounds(self, scale: f32) -> Bounds<Points> {
        Bounds::new(
            point(px(self.x * scale), px(self.y * scale)),
            size(px(self.width * scale), px(self.height * scale)),
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Grip {
    Move,
    Corner(bool, bool),
}
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Drag {
    pub crop: Crop,
    pub origin: (f32, f32),
    pub grip: Grip,
}

pub(super) fn grip(crop: Crop, point: (f32, f32), tolerance: f32) -> Option<Grip> {
    for (right, bottom) in [(false, false), (true, false), (false, true), (true, true)] {
        let x = crop.x + if right { crop.width } else { 0.0 };
        let y = crop.y + if bottom { crop.height } else { 0.0 };
        if (x - point.0).abs() <= tolerance && (y - point.1).abs() <= tolerance {
            return Some(Grip::Corner(right, bottom));
        }
    }
    (point.0 >= crop.x
        && point.0 <= crop.x + crop.width
        && point.1 >= crop.y
        && point.1 <= crop.y + crop.height)
        .then_some(Grip::Move)
}

pub(super) fn drag(
    drag: Drag,
    point: (f32, f32),
    width: u32,
    height: u32,
    ratio: Option<f32>,
) -> Crop {
    let (width, height) = (width as f32, height as f32);
    match drag.grip {
        Grip::Move => Crop {
            x: (drag.crop.x + point.0 - drag.origin.0)
                .clamp(0.0, (width - drag.crop.width).max(0.0)),
            y: (drag.crop.y + point.1 - drag.origin.1)
                .clamp(0.0, (height - drag.crop.height).max(0.0)),
            ..drag.crop
        },
        Grip::Corner(right, bottom) => {
            let anchor = (
                drag.crop.x + if right { 0.0 } else { drag.crop.width },
                drag.crop.y + if bottom { 0.0 } else { drag.crop.height },
            );
            let (sx, sy) = (
                if right { 1.0 } else { -1.0 },
                if bottom { 1.0 } else { -1.0 },
            );
            let max_width = if right { width - anchor.0 } else { anchor.0 };
            let max_height = if bottom { height - anchor.1 } else { anchor.1 };
            let mut crop_width =
                (sx * (point.0 - anchor.0)).clamp(1.0_f32.min(max_width), max_width.max(1.0));
            let mut crop_height =
                (sy * (point.1 - anchor.1)).clamp(1.0_f32.min(max_height), max_height.max(1.0));
            if let Some(ratio) = ratio {
                crop_width = crop_width
                    .max(crop_height * ratio)
                    .min(max_width)
                    .min(max_height * ratio)
                    .max(1.0);
                crop_height = crop_width / ratio;
            }
            Crop {
                x: if right {
                    anchor.0
                } else {
                    anchor.0 - crop_width
                },
                y: if bottom {
                    anchor.1
                } else {
                    anchor.1 - crop_height
                },
                width: crop_width,
                height: crop_height,
            }
        }
    }
}

pub(super) fn multiply(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

pub(super) fn inverse(m: [f32; 6]) -> [f32; 6] {
    let determinant = m[0] * m[3] - m[1] * m[2];
    [
        m[3] / determinant,
        -m[1] / determinant,
        -m[2] / determinant,
        m[0] / determinant,
        (m[2] * m[5] - m[3] * m[4]) / determinant,
        (m[1] * m[4] - m[0] * m[5]) / determinant,
    ]
}

/// The source-to-canvas transform at each recipe boundary. Stroke coordinates
/// use their creation boundary and are mapped to the final canvas at draw time.
pub(super) type Transform = ([f32; 6], (u32, u32));

pub(super) fn transforms(
    edits: &[mixin_desktop_api::ImageEdit],
    mut width: u32,
    mut height: u32,
) -> Vec<Transform> {
    use mixin_desktop_api::ImageEdit;
    let mut matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut output = vec![(matrix, (width, height))];
    for edit in edits {
        let operation = match edit {
            ImageEdit::RotateClockwise => {
                let h = height;
                std::mem::swap(&mut width, &mut height);
                [0.0, 1.0, -1.0, 0.0, h as f32, 0.0]
            }
            ImageEdit::RotateCounterClockwise => {
                let w = width;
                std::mem::swap(&mut width, &mut height);
                [0.0, -1.0, 1.0, 0.0, 0.0, w as f32]
            }
            ImageEdit::FlipHorizontal => [-1.0, 0.0, 0.0, 1.0, width as f32, 0.0],
            ImageEdit::Crop {
                x,
                y,
                width: w,
                height: h,
            } => {
                width = *w;
                height = *h;
                [1.0, 0.0, 0.0, 1.0, -(*x as f32), -(*y as f32)]
            }
            _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        };
        matrix = multiply(operation, matrix);
        output.push((matrix, (width, height)));
    }
    output
}
