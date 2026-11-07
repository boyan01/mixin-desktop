//! Local QR preview for a selected text range or the original complete message.
use crate::{i18n, ui};
use nsh_base::theme;
use nsh_core::prelude::*;

pub(super) fn screen(content: String) -> impl IntoElement {
    let i18n = i18n::current();
    let source = crate::qr::source(&content, 320);
    let content = stored(content);
    scroll().flex_shrink(1.0).min_h_0().min_w_0().child(
        view()
            .w_full()
            .min_w_0()
            .items_center()
            .gap_4()
            .p_4()
            .child(match source {
                Some(source) => image(source)
                    .size(px(280.0))
                    .max_w(percentage(1.0))
                    .object_fit(ResizeMode::Contain)
                    .label(move || i18n.tr("QR code", "二维码"))
                    .into_element(),
                None => text(move || {
                    i18n.tr(
                        "Text is too long for a QR code.",
                        "文本过长，无法生成二维码。",
                    )
                })
                .text_sm()
                .text_center()
                .text_color(move || theme().destructive)
                .into_element(),
            })
            .child(
                text(move || content.with(|value| value.clone()))
                    .w_full()
                    .min_w_0()
                    .text_sm()
                    .text_center()
                    .select_text(),
            )
            .child(ui::action(
                move || i18n.tr("Copy", "复制"),
                move || {
                    set_clipboard(content.with(|value| value.clone()));
                },
            )),
    )
}
