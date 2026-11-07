use super::{Feedback, body, client, section, tr};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};
use nsh_base::{SelectEntry, SelectItem, SliderValue, select, slider, theme};
use nsh_core::prelude::*;
pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let chat =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    cleanup(move || chat.cancel_font_preview());
    let value = signal(String::from("system"));
    let ready = signal(false);
    let feedback = Feedback::new();
    let retry = signal(0_u64);
    effect(move || {
        let _revision = retry.get();
        feedback.failed.set(false);
        spawn(async move {
            match runtime::run(async {
                client()
                    .await?
                    .setting("desktop_ui_appearance".into())
                    .await
            })
            .await
            {
                Ok(next) => {
                    value.set(next.unwrap_or_else(|| "system".into()));
                    ready.set(true);
                }
                Err(_) => feedback.failed.set(true),
            }
        });
    });
    scroll().flex_1().min_h_0().child(
        body()
            .child(
                section(move || tr(i18n, "Theme", "主题")).child(
                    select()
                        .items(move || {
                            vec![
                                SelectItem::new("system", tr(i18n, "Follow system", "跟随系统")),
                                SelectItem::new("light", tr(i18n, "Light", "浅色")),
                                SelectItem::new("dark", tr(i18n, "Dark", "深色")),
                            ]
                        })
                        .value(move || Some(SharedString::from(value.get())))
                        .disabled(move || !ready.get() || feedback.busy.get())
                        .accessibility_label(move || Some(tr(i18n, "Theme", "主题").into()))
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
                                .justify_start()
                                .child(text(move || match value.get().as_str() {
                                    "light" => tr(i18n, "Light", "浅色"),
                                    "dark" => tr(i18n, "Dark", "深色"),
                                    _ => tr(i18n, "Follow system", "跟随系统"),
                                }))
                                .child(ui::icon(ui::Icon::ChevronDown)),
                        )
                        .on_change(move |next: &Option<SharedString>| {
                            let Some(next) = next.as_ref().map(ToString::to_string) else {
                                return;
                            };
                            if next == value.get_untracked() {
                                return;
                            }
                            let previous = value.get_untracked();
                            value.set(next.clone());
                            apply(&next);
                            feedback.start();
                            spawn(async move {
                                let result = runtime::commit(async move {
                                    client()
                                        .await?
                                        .set_setting(
                                            "desktop_ui_appearance".into(),
                                            (next != "system").then_some(next),
                                        )
                                        .await
                                })
                                .await;
                                if result.is_err() {
                                    apply(&previous);
                                    value.set(previous);
                                }
                                feedback.busy.set(false);
                                feedback.failed.set(result.is_err());
                            });
                        }),
                ),
            )
            .child(section(move || tr(i18n, "Chat preferences", "聊天偏好"))
                .child(ui::setting_toggle(move || tr(i18n, "Show avatar", "显示头像"), move || chat.show_avatar(), move || !chat.ready() || chat.busy(), move |value| chat.set_avatar(value)))
                .child(ui::setting_toggle(move || tr(i18n, "Show Mixin ID", "显示 Mixin ID"), move || chat.show_identity(), move || !chat.ready() || chat.busy(), move |value| chat.set_identity(value))))
            .child(section(move || tr(i18n, "Chat text size", "聊天文字大小"))
                .child(view().gap_3().p_4().rounded_lg().bg(move || theme().color("chat-background").unwrap_or(theme().muted))
                    .child(view().items_start().child(view().px_3().py_2().max_w(relative(0.9)).rounded_lg().bg(move || theme().color("bubble-in").unwrap_or(theme().card))
                        .child(text(move || tr(i18n, "Hi!", "你好！")).text_size(move || chat.primary_size()))))
                    .child(view().items_end().child(view().px_3().py_2().max_w(relative(0.9)).rounded_lg().bg(move || theme().color("bubble-out").unwrap_or(theme().accent))
                        .child(text(move || tr(i18n, "How are you?", "最近怎么样？")).text_size(move || chat.primary_size())))))
                .child(view().flex_row().items_center().gap_3()
                    .child(text("A").text_xs())
                    .child(slider().flex_1().min(-2.0).max(4.0).step(1.0)
                        .disabled(move || !chat.ready())
                        .value(move || Some(SliderValue::Single(chat.font_delta())))
                        .accessibility_label(move || Some(tr(i18n, "Chat text size", "聊天文字大小").into()))
                        .track_class("h-8 justify-center disabled:opacity-40")
                        .indicator_class("h-1 rounded-full bg-muted-foreground/30")
                        .range_class("rounded-full bg-primary")
                        .thumb_class("size-4 rounded-full bg-primary focus:outline-2 focus:outline-ring")
                        .on_change(move |value: &SliderValue| { if let SliderValue::Single(value) = value { chat.preview_font(*value); } })
                        .on_change_end(move |value: &SliderValue| { if let SliderValue::Single(value) = value { chat.save_font(*value); } }))
                    .child(text("A").text_2xl())))
            .child(when(move || chat.failed(), move || view().gap_2()
                .child(text(move || tr(i18n, "Couldn’t save or load chat preferences. Try again.", "无法保存或读取聊天偏好，请重试。")).text_sm().text_color(move || theme().destructive))
                .child(ui::action(move || i18n.text(Key::Retry), move || chat.retry()))))
            .child(section(move || i18n.text(Key::Language)).child(i18n.picker()))
            .child(feedback.view(move || tr(i18n, "Saving…", "正在保存…")))
            .child(when(
                move || feedback.failed.get() && !ready.get(),
                move || {
                    ui::action(
                        move || i18n.text(Key::Retry),
                        move || retry.update(|r| *r += 1),
                    )
                },
            )),
    )
}

fn apply(value: &str) {
    nsh_core::element::set_color_scheme(match value {
        "light" => nsh_core::platform::Appearance::Light,
        "dark" => nsh_core::platform::Appearance::Dark,
        _ => nsh_core::platform::Appearance::System,
    });
}
