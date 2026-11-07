//! Native Canvas uses the same raster coordinates and ImageEdit contract as the composer.
use super::Picker;
use crate::{
    conversation::parts,
    i18n::tr,
    runtime,
    ui::{self, Typography as _},
};
use mixin_desktop_api::{ImageEdit, PreparedAttachment};
use nsh_base::SliderValue;
use nsh_core::prelude::*;

#[derive(Clone, Copy)]
struct Crop {
    width: f32,
    height: f32,
    center: Signal<(f32, f32)>,
    zoom: Signal<f64>,
    viewport: Signal<f32>,
}

impl Crop {
    fn side(self) -> f32 {
        (self.width.min(self.height) / self.zoom.get() as f32).max(1.0)
    }
    fn bounds(self) -> (f32, f32, f32) {
        let side = self.side();
        let (x, y) = self.center.get();
        (
            (x - side / 2.0).clamp(0.0, self.width - side),
            (y - side / 2.0).clamp(0.0, self.height - side),
            side,
        )
    }
    fn move_to(self, x: f32, y: f32) {
        let half = self.side() / 2.0;
        self.center.set((
            x.clamp(half, self.width - half),
            y.clamp(half, self.height - half),
        ));
    }
    fn set_zoom(self, zoom: f64) {
        self.zoom.set(zoom.clamp(1.0, 8.0));
        let (x, y) = self.center.get_untracked();
        self.move_to(x, y);
    }
    fn edit(self) -> ImageEdit {
        let (x, y, side) = self.bounds();
        let side = (side.round() as u32).clamp(1, self.width.min(self.height) as u32);
        ImageEdit::Crop {
            x: (x.round() as u32).min(self.width as u32 - side),
            y: (y.round() as u32).min(self.height as u32 - side),
            width: side,
            height: side,
        }
    }
}

pub(super) fn screen(
    picker: Picker,
    source: PreparedAttachment,
    group: bool,
    save: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let width = source.width.unwrap_or(1).max(1) as f32;
    let height = source.height.unwrap_or(1).max(1) as f32;
    let source = stored(source);
    let crop = Crop {
        width,
        height,
        center: signal((width / 2.0, height / 2.0)),
        zoom: signal(1.0),
        viewport: signal(320.0),
    };
    let drag = signal(None::<((f32, f32), (f32, f32))>);
    let focused = signal(false);
    let saving = signal(false);
    let failed = signal(false);
    let version = signal(0_u64);
    let cancel = move || {
        version.update(|version| *version = version.wrapping_add(1));
        picker.open.set(false);
    };
    let done = move || {
        if saving.get_untracked() || !picker.open.get_untracked() {
            return;
        }
        saving.set(true);
        failed.set(false);
        let ticket = version.get_untracked();
        let source = source.get();
        let edit = crop.edit();
        let media = picker.media.with(|media| media.clone());
        spawn(async move {
            let result =
                runtime::run(async move { media.encode_avatar(source, edit, group).await }).await;
            if !picker.open.get_untracked() || version.get_untracked() != ticket {
                return;
            }
            saving.set(false);
            match result {
                Ok(encoded) => {
                    save(encoded);
                    picker.open.set(false);
                }
                Err(_) => failed.set(true),
            }
        });
    };
    view()
        .child(when(
            move || focused.get() && picker.open.get(),
            move || {
                for (key, dx, dy) in [
                    ("left", -1.0, 0.0),
                    ("right", 1.0, 0.0),
                    ("up", 0.0, -1.0),
                    ("down", 0.0, 1.0),
                ] {
                    on_key(key, move |_| {
                        if saving.get_untracked() {
                            return;
                        }
                        let (x, y) = crop.center.get_untracked();
                        crop.move_to(
                            x - dx * crop.side() / crop.viewport.get(),
                            y - dy * crop.side() / crop.viewport.get(),
                        );
                    });
                }
                view().hidden()
            },
        ))
        .child(ui::dialog(
            picker.presence,
            cancel,
            ui::dialog_card(ui::DialogWidth::Regular)
                .max_h(relative(1.0))
                .child(ui::dialog_title(move || tr("Crop photo", "裁剪头像")))
                .child(
                    text(move || {
                        tr(
                            "Drag to move. Use the slider to zoom. Images up to 10 MB.",
                            "拖动调整位置，使用滑块缩放。图片最大 10 MB。",
                        )
                    })
                    .text_role(ui::TextRole::Body)
                    .text_color(ui::fg_secondary),
                )
                .child(
                    view()
                        .w_full()
                        .aspect_ratio(1.0)
                        .overflow_hidden()
                        .rounded(ui::radius_md())
                        .on_layout(move |bounds: &Bounds<Points>| {
                            crop.viewport.set(bounds.size.width.0.max(1.0))
                        })
                        .child(
                            canvas(move || {
                                let (x, y, side) = crop.bounds();
                                let scale = crop.viewport.get() / side;
                                let path = source.with(|source| source.path.clone());
                                let extent = crop.viewport.get();
                                let mut painter = Painter::new().draw_image(
                                    &path,
                                    Bounds::new(
                                        point(px(-x * scale), px(-y * scale)),
                                        size(px(width * scale), px(height * scale)),
                                    ),
                                );
                                painter = painter
                                    .stroke_color(color(ui::surface_pane()))
                                    .line_width(1.0);
                                for third in [1.0 / 3.0, 2.0 / 3.0] {
                                    painter = painter
                                        .begin_path()
                                        .move_to(extent * third, 0.0)
                                        .line_to(extent * third, extent)
                                        .move_to(0.0, extent * third)
                                        .line_to(extent, extent * third)
                                        .stroke();
                                }
                                painter
                            })
                            .w_full()
                            .h_full()
                            .focusable(true)
                            .label(move || {
                                tr(
                                    "Crop preview. Drag or use arrow keys to move the photo.",
                                    "头像裁剪预览。拖动或使用方向键移动图片。",
                                )
                            })
                            .focus(ui::ring)
                            .on_focus(move || focused.set(true))
                            .on_blur(move || focused.set(false))
                            .claim_touches(TouchClaim::on_start())
                            .keep_touches(true)
                            .on_responder_grant(move |event: &ResponderEvent| {
                                if !saving.get_untracked() {
                                    drag.set(Some((
                                        (event.location.x.0, event.location.y.0),
                                        crop.center.get_untracked(),
                                    )));
                                }
                            })
                            .on_responder_move(move |event: &ResponderEvent| {
                                if saving.get_untracked() {
                                    return;
                                }
                                if let Some((origin, center)) = drag.get_untracked() {
                                    let scale = crop.side() / crop.viewport.get();
                                    crop.move_to(
                                        center.0 - (event.location.x.0 - origin.0) * scale,
                                        center.1 - (event.location.y.0 - origin.1) * scale,
                                    );
                                }
                            })
                            .on_responder_release(move |_: &ResponderEvent| drag.set(None))
                            .on_responder_terminate(move |_: &ResponderEvent| drag.set(None)),
                        ),
                )
                .child(
                    ui::slider()
                        .w_full()
                        .min(1.0)
                        .max(8.0)
                        .step(0.01)
                        .value(move || Some(SliderValue::Single(crop.zoom.get())))
                        .disabled(saving)
                        .accessibility_label(move || Some(tr("Photo zoom", "头像缩放").into()))
                        .on_change(move |value: &SliderValue| {
                            if let SliderValue::Single(zoom) = value {
                                crop.set_zoom(*zoom);
                            }
                        }),
                )
                .child(when(failed, move || {
                    ui::error_text(move || {
                        tr(
                            "Couldn’t prepare the photo. Try again.",
                            "无法处理头像，请重试。",
                        )
                    })
                }))
                .child(
                    parts::dialog_actions()
                        .child(
                            ui::action(
                                move || tr("Reset", "重置"),
                                move || {
                                    crop.zoom.set(1.0);
                                    crop.center.set((width / 2.0, height / 2.0));
                                },
                            )
                            .disabled(saving),
                        )
                        .child(ui::action(move || tr("Cancel", "取消"), cancel))
                        .child(ui::primary().disabled(saving).on_press(done).child(text(
                            move || {
                                if saving.get() {
                                    tr("Preparing…", "正在处理…")
                                } else {
                                    tr("Save", "保存")
                                }
                            },
                        ))),
                ),
        ))
}
