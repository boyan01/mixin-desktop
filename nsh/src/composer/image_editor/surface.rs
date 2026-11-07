use super::{Editor, Tool, geometry::Crop, raster_color};
use crate::ui;
use mixin_desktop_api::ImageEdit;
use nsh_base::{SelectEntry, SelectItem, SliderValue, dialog, select, slider, theme};
use nsh_core::prelude::*;

fn tr(en: &'static str, zh: &'static str) -> &'static str {
    crate::i18n::tr(en, zh)
}

pub(super) fn screen(
    editor: Editor,
    presence: crate::motion::Presence,
    done: impl Fn() + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    view()
        .child(when(
            move || presence.interactive() && editor.focused.get() && editor.tool.get() == Tool::Crop,
            move || {
                for (key, dx, dy) in [
                    ("left", -1.0, 0.0),
                    ("right", 1.0, 0.0),
                    ("up", 0.0, -1.0),
                    ("down", 0.0, 1.0),
                ] {
                    on_key(key, move |_| editor.nudge_crop(dx, dy, false));
                    on_key(&format!("shift+{key}"), move |_| {
                        editor.nudge_crop(dx, dy, true)
                    });
                }
                view().hidden()
            },
        ))
        .child(
            dialog()
                .animation(nsh_core::ModalAnimation::None)
                .on_cancel(cancel)
                .close_on_escape(false)
                .close_on_backdrop_press(false)
                .flex_1()
                .child(presence.surface(view().flex_1()
                .px_5()
                .gap_0()
                .bg(move || theme().background.opacity(0.96))
                .child(view().h_14())
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .items_center()
                        .justify_center()
                        .on_layout(move |bounds: &Bounds<Points>| {
                            editor
                                .viewport
                                .set((bounds.size.width.0.max(1.0), bounds.size.height.0.max(1.0)))
                        })
                        .child(
                            view()
                                .relative()
                                .w(move || px(editor.dimensions().0 as f32 * editor.scale()))
                                .h(move || px(editor.dimensions().1 as f32 * editor.scale()))
                                .child(
                                    canvas(move || super::preview::image(editor))
                                        .w_full()
                                        .h_full()
                                        .label(move || tr("Image preview", "图片预览")),
                                )
                                .child(
                                    canvas(move || super::preview::foreground(editor))
                                        .absolute()
                                        .top_0()
                                        .left_0()
                                        .w_full()
                                        .h_full()
                                        .focusable(true)
                                        .focus(|style| {
                                            style
                                                .outline(px(2.0))
                                                .outline_color(move || theme().ring)
                                        })
                                        .on_focus(move || editor.focused.set(true))
                                        .on_blur(move || editor.focused.set(false))
                                        .label(move || {
                                            if editor.tool.get() == Tool::Crop {
                                                tr(
                                                    "Drag crop corners or move the crop",
                                                    "拖动裁剪角点或移动裁剪框",
                                                )
                                            } else {
                                                tr("Draw on the image", "在图片上绘画")
                                            }
                                        })
                                        .claim_touches(TouchClaim::on_start())
                                        .keep_touches(true)
                                        .on_responder_grant(move |event: &ResponderEvent| {
                                            editor.start(event)
                                        })
                                        .on_responder_move(move |event: &ResponderEvent| {
                                            editor.moved(event)
                                        })
                                        .on_responder_release(move |_: &ResponderEvent| {
                                            editor.end()
                                        })
                                        .on_responder_terminate(move |_: &ResponderEvent| {
                                            editor.stroke.set(Vec::new());
                                            editor.drag.set(None);
                                        }),
                                ),
                        ),
                )
                .child(when(editor.loading, || {
                    text(move || tr("Preparing image…", "正在准备图片…"))
                        .text_sm()
                        .text_center()
                        .text_color(move || theme().muted_foreground)
                }))
                .child(when(editor.failed, move || {
                    view()
                        .items_center()
                        .gap_2()
                        .child(
                            text(move || {
                                tr(
                                    "Couldn’t edit this image. Undo the last edit or try again.",
                                    "无法编辑此图片，请撤销上一步或重试。",
                                )
                            })
                            .text_sm()
                            .text_color(move || theme().destructive),
                        )
                        .child(ui::action(move || tr("Retry", "重试"), done))
                }))
                .child(when(
                    move || editor.tool.get() == Tool::Crop,
                    move || {
                        view().items_center().mt_5().child(
                            ui::action(move || tr("Reset", "重置"), move || editor.reset())
                                .disabled(editor.loading),
                        )
                    },
                ))
                .child(when(
                    move || editor.tool.get() != Tool::Crop,
                    move || view().mt_5().child(colors(editor)),
                ))
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .mt_2()
                        .gap_2()
                        .child(ui::action(move || tr("Cancel", "取消"), cancel))
                        .child(when(
                            move || editor.tool.get() == Tool::Crop,
                            move || {
                                view()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        ui::action(
                                            move || tr("Rotate", "旋转"),
                                            move || editor.edit(ImageEdit::RotateCounterClockwise),
                                        )
                                        .disabled(editor.loading),
                                    )
                                    .child(
                                        ui::action(
                                            move || tr("Flip", "翻转"),
                                            move || editor.edit(ImageEdit::FlipHorizontal),
                                        )
                                        .disabled(editor.loading),
                                    )
                                    .child(ratios(editor))
                                    .child(
                                        ui::action(
                                            move || tr("Draw", "画笔"),
                                            move || editor.enter_drawing(),
                                        )
                                        .disabled(editor.loading),
                                    )
                            },
                        ))
                        .child(when(
                            move || editor.tool.get() != Tool::Crop,
                            move || {
                                view()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        ui::command()
                                            .selected(move || editor.tool.get() == Tool::Brush)
                                            .selected_style(|style| {
                                                style.bg(move || theme().accent)
                                            })
                                            .disabled(editor.loading)
                                            .on_press(move || editor.tool.set(Tool::Brush))
                                            .child(text(move || tr("Brush", "画笔"))),
                                    )
                                    .child(
                                        ui::command()
                                            .selected(move || editor.tool.get() == Tool::Eraser)
                                            .selected_style(|style| {
                                                style.bg(move || theme().accent)
                                            })
                                            .disabled(editor.loading)
                                            .on_press(move || editor.tool.set(Tool::Eraser))
                                            .child(text(move || tr("Eraser", "橡皮"))),
                                    )
                                    .child(
                                        ui::action(
                                            move || tr("Undo", "撤销"),
                                            move || editor.undo(),
                                        )
                                        .disabled(
                                            move || editor.loading.get() || !editor.has_drawing(),
                                        ),
                                    )
                                    .child(
                                        ui::action(
                                            move || tr("Redo", "重做"),
                                            move || editor.redo(),
                                        )
                                        .disabled(
                                            move || {
                                                editor.loading.get()
                                                    || editor.redo.with(Vec::is_empty)
                                            },
                                        ),
                                    )
                            },
                        ))
                        .child(
                            ui::primary()
                                .on_press(done)
                                .disabled(editor.loading)
                                .child(text(move || tr("Done", "完成"))),
                        ),
                )
                .child(view().h_14()))),
        )
}

fn ratios(editor: Editor) -> impl IntoElement {
    const CHOICES: [(&str, Option<f32>); 8] = [
        ("free", None),
        ("1:1", Some(1.0)),
        ("3:2", Some(1.5)),
        ("2:3", Some(2.0 / 3.0)),
        ("4:3", Some(4.0 / 3.0)),
        ("3:4", Some(0.75)),
        ("16:9", Some(16.0 / 9.0)),
        ("9:16", Some(9.0 / 16.0)),
    ];
    select()
        .items(|| {
            CHOICES
                .iter()
                .map(|(name, _)| {
                    SelectItem::new(
                        *name,
                        if *name == "free" {
                            tr("Original image", "原图")
                        } else {
                            name
                        },
                    )
                })
                .collect::<Vec<_>>()
        })
        .value(move || {
            Some(SharedString::from(
                CHOICES
                    .iter()
                    .find(|(_, value)| match (*value, editor.ratio.get()) {
                        (Some(a), Some(b)) => (a - b).abs() < 0.001,
                        (None, None) => true,
                        _ => false,
                    })
                    .map(|(name, _)| *name)
                    .unwrap_or("free"),
            ))
        })
        .disabled(editor.loading)
        .accessibility_label(move || Some(tr("Crop ratio", "裁剪比例").into()))
        .popup_style(ui::popup)
        .item_class("rounded-md focus:bg-accent selected:bg-accent")
        .render_item(|entry: &SelectEntry| {
            view()
                .px_3()
                .py_2()
                .child(text(entry.title.clone()).text_sm())
        })
        .trigger(
            ui::command()
                .child(text(move || tr("Aspect ratio", "比例")))
                .child(ui::icon(ui::Icon::ChevronDown)),
        )
        .on_change(move |value: &Option<SharedString>| {
            if let Some((_, ratio)) = value
                .as_ref()
                .and_then(|value| CHOICES.iter().find(|(name, _)| *name == value.as_str()))
            {
                let (width, height) = editor.dimensions();
                editor.ratio.set(*ratio);
                editor.crop.set(Crop::centered(width, height, *ratio));
            }
        })
}

fn colors(editor: Editor) -> impl IntoElement {
    // Palette values are user-selected raster data, not application theme colors.
    const COLORS: [([u8; 4], &str, &str); 8] = [
        ([255, 255, 255, 255], "White", "白色"),
        ([0, 0, 0, 255], "Black", "黑色"),
        ([142, 142, 147, 255], "Gray", "灰色"),
        ([232, 77, 61, 255], "Red", "红色"),
        ([248, 205, 62, 255], "Yellow", "黄色"),
        ([100, 211, 79, 255], "Green", "绿色"),
        ([48, 119, 255, 255], "Blue", "蓝色"),
        ([172, 104, 222, 255], "Purple", "紫色"),
    ];
    let custom = signal(false);
    let initial: nsh_core::color::Hsla = raster_color(editor.color.get_untracked()).into();
    let hue = signal(f64::from(initial.h) * 360.0);
    let bar_width = signal(1.0_f32);
    view().flex_row().flex_wrap().items_center().justify_center().gap_2()
        .child(ui::command().disabled(editor.loading).selected(custom)
            .selected_style(|style| style.bg(move || theme().accent))
            .on_press(move || {
                if !custom.get_untracked() {
                    let current: nsh_core::color::Hsla = raster_color(editor.color.get_untracked()).into();
                    hue.set(f64::from(current.h) * 360.0);
                }
                custom.update(|custom| *custom = !*custom);
            }).child(text(move || tr("Custom color…", "自定义颜色…"))))
        .child(when(move || !custom.get(), move || view().flex_row().flex_wrap().gap_2()
            .children(COLORS.into_iter().map(move |(color, en, zh)| {
                ui::command().size_8().px_1().py_1().disabled(editor.loading)
                    .selected(move || editor.color.get() == color)
                    .selected_style(|style| style.border_2().border_color(move || theme().primary))
                    .accessibility_label(move || Some(tr(en, zh).into()))
                    .on_press(move || editor.color.set(color))
                    .child(view().size_5().rounded_full().border_1().border_color(move || theme().border).bg(raster_color(color)))
            }))))
        .child(when(custom, move || view().relative().w_56().h_8()
            .on_layout(move |bounds: &Bounds<Points>| bar_width.set(bounds.size.width.0.max(1.0)))
            .child(canvas(move || {
                let width = bar_width.get();
                let mut painter = Painter::new();
                for hue in 0..360 { painter = painter.fill_color(nsh_core::color::hsla(hue as f32 / 360.0, 1.0, 0.5, 1.0))
                    .fill_rect(width * hue as f32 / 360.0, 0.0, width / 360.0 + 1.0, 8.0); }
                painter
            }).absolute().top_3().w_full().h_2().rounded_full().overflow_hidden())
            .child(slider().w_full().min(0.0).max(360.0).step(1.0)
                .value(move || Some(SliderValue::Single(hue.get()))).disabled(editor.loading)
                .accessibility_label(move || Some(tr("Custom drawing color", "自定义画笔颜色").into()))
                .track_class("h-8 justify-center").indicator_class("h-2 rounded-full").range_class("bg-transparent")
                .thumb_class("w-2 h-7 rounded-full bg-background border border-foreground/40 focus:outline-2 focus:outline-ring")
                .on_change(move |value: &SliderValue| {
                    if let SliderValue::Single(value) = value {
                        hue.set(*value);
                        let color: Rgba = nsh_core::color::hsla(*value as f32 / 360.0, 1.0, 0.5, 1.0).into();
                        editor.color.set([(color.r * 255.0).round() as u8, (color.g * 255.0).round() as u8, (color.b * 255.0).round() as u8, 255]);
                    }
                }))))
}
