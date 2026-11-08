//! Shared native Markdown presentation for posts and downloaded documents.
use nsh_base::{
    TextViewCodeAction, TextViewElement, TextViewLinkPressEvent, TextViewStyles,
    text_view::{PartStyle, text_view},
};
use nsh_core::{element::StyleRefinement, prelude::*};

use super::{fg_primary, post_reader_width, space_xl};

pub(crate) fn markdown(
    content: impl Fn() -> SharedString + 'static,
    size: impl Fn() -> Points + Copy + 'static,
) -> TextViewElement {
    text_view()
        .markdown(move || Some(content()))
        .w_full()
        .min_w_0()
        .text_size(size)
        .line_height(move || size() * 1.7)
        .text_color(fg_primary)
        .styles(move || {
            // Override the parts with fixed font sizes as well as the root, so
            // message font settings preserve the document's reading hierarchy.
            let typography = |scale: f32, line: f32| -> Option<PartStyle> {
                Some(PartStyle(
                    StyleRefinement::default()
                        .text_size(size() * scale)
                        .line_height(size() * scale * line)
                        .into(),
                ))
            };
            TextViewStyles {
                heading1: typography(1.75, 1.25),
                heading2: typography(1.375, 1.35),
                heading3: typography(1.125, 1.35),
                heading4: typography(1.0, 1.35),
                heading5: typography(1.0, 1.35),
                heading6: typography(1.0, 1.35),
                code_block: typography(0.875, 1.6),
                table_cell: typography(0.875, 1.6),
                ..Default::default()
            }
        })
}

pub(crate) fn markdown_reader(
    content: impl Fn() -> SharedString + 'static,
    size: impl Fn() -> Points + Copy + 'static,
    on_link: impl Fn(&TextViewLinkPressEvent) + 'static,
) -> View {
    let i18n = crate::i18n::current();
    view()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .w_full()
        .items_center()
        .child(
            scroll()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .w_full()
                .max_w(post_reader_width())
                .child(
                    view().w_full().min_w_0().p(space_xl()).child(
                        markdown(content, size)
                            .selectable(true)
                            .code_block_actions(move || {
                                vec![
                                    TextViewCodeAction::new("copy")
                                        .with_label(i18n.tr("Copy code", "复制代码")),
                                ]
                            })
                            .on_link_press(on_link),
                    ),
                ),
        )
}
