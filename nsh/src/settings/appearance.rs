use super::{Feedback, body, client, section};
use crate::{
    appearance::Appearance,
    conversation::parts,
    i18n::{self, I18n, Key},
    runtime, ui,
};
use nsh_base::{SelectItem, SliderValue, slider, theme};
use nsh_core::prelude::*;
pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let chat = use_context::<Appearance>().expect("Home provides chat appearance");
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
    let theme_label = move |value: &str| match value {
        "light" => i18n.tr("Light", "浅色"),
        "dark" => i18n.tr("Dark", "深色"),
        _ => i18n.tr("Follow system", "跟随系统"),
    };
    scroll().flex_1().min_h_0().child(
        body()
            .child(
                section(move || i18n.tr("Theme", "主题")).child(
                    parts::option_select()
                        .items(move || {
                            ["system", "light", "dark"]
                                .map(|value| SelectItem::new(value, theme_label(value)))
                                .to_vec()
                        })
                        .value(move || Some(SharedString::from(value.get())))
                        .disabled(move || !ready.get() || feedback.busy.get())
                        .accessibility_label(move || Some(i18n.tr("Theme", "主题").into()))
                        .trigger(parts::select_trigger(move || {
                            value.with(|v| theme_label(v))
                        }))
                        .on_change(move |next: &Option<SharedString>| {
                            if let Some(next) = next.as_ref().map(ToString::to_string) {
                                save_theme(next, value, feedback);
                            }
                        }),
                ),
            )
            .child(
                section(move || i18n.tr("Chat preferences", "聊天偏好"))
                    .child(ui::setting_toggle(
                        move || i18n.tr("Show avatar", "显示头像"),
                        move || chat.show_avatar(),
                        move || !chat.ready() || chat.busy(),
                        move |value| chat.set_avatar(value),
                    ))
                    .child(ui::setting_toggle(
                        move || i18n.tr("Show Mixin ID", "显示 Mixin ID"),
                        move || chat.show_identity(),
                        move || !chat.ready() || chat.busy(),
                        move |value| chat.set_identity(value),
                    )),
            )
            .child(
                section(move || i18n.tr("Chat text size", "聊天文字大小"))
                    .child(text_preview(i18n, chat))
                    .child(text_size_slider(i18n, chat)),
            )
            .child(when(
                move || chat.failed(),
                move || {
                    parts::failure(
                        move || {
                            i18n.tr(
                                "Couldn’t save or load chat preferences. Try again.",
                                "无法保存或读取聊天偏好，请重试。",
                            )
                        },
                        move || chat.retry(),
                    )
                },
            ))
            .child(section(move || i18n.text(Key::Language)).child(i18n.picker()))
            .child(feedback.view(move || i18n.tr("Saving…", "正在保存…")))
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

fn save_theme(next: String, value: Signal<String>, feedback: Feedback) {
    let previous = value.get_untracked();
    if next == previous {
        return;
    }
    value.set(next.clone());
    apply(&next);
    feedback.start();
    let task = runtime::commit(async move {
        client()
            .await?
            .set_setting(
                "desktop_ui_appearance".into(),
                (next != "system").then_some(next),
            )
            .await
    });
    feedback.owner.run(|| {
        spawn(async move {
            let result = task.await;
            if result.is_err() {
                apply(&previous);
                value.set(previous);
            }
            feedback.finish(result.is_ok());
        })
    });
}

fn preview_bubble(
    label: impl Fn() -> &'static str + 'static,
    background: impl Fn() -> AdaptiveColor + 'static,
    chat: Appearance,
) -> View {
    view()
        .px_3()
        .py_2()
        .max_w(relative(0.9))
        .rounded_lg()
        .bg(background)
        .child(text(label).text_size(move || chat.primary_size()))
}

fn text_preview(i18n: I18n, chat: Appearance) -> impl IntoElement {
    view()
        .gap_3()
        .p_4()
        .rounded_lg()
        .bg(move || theme().color("chat-background").unwrap_or(theme().muted))
        .child(view().items_start().child(preview_bubble(
            move || i18n.tr("Hi!", "你好！"),
            move || theme().color("bubble-in").unwrap_or(theme().card),
            chat,
        )))
        .child(view().items_end().child(preview_bubble(
            move || i18n.tr("How are you?", "最近怎么样？"),
            move || theme().color("bubble-out").unwrap_or(theme().accent),
            chat,
        )))
}

fn text_size_slider(i18n: I18n, chat: Appearance) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .child(text("A").text_xs())
        .child(
            slider()
                .flex_1()
                .min(-2.0)
                .max(4.0)
                .step(1.0)
                .disabled(move || !chat.ready())
                .value(move || Some(SliderValue::Single(chat.font_delta())))
                .accessibility_label(move || Some(i18n.tr("Chat text size", "聊天文字大小").into()))
                .track_class("h-8 justify-center disabled:opacity-40")
                .indicator_class("h-1 rounded-full bg-muted-foreground/30")
                .range_class("rounded-full bg-primary")
                .thumb_class("size-4 rounded-full bg-primary focus:outline-2 focus:outline-ring")
                .on_change(move |value: &SliderValue| {
                    if let SliderValue::Single(value) = value {
                        chat.preview_font(*value);
                    }
                })
                .on_change_end(move |value: &SliderValue| {
                    if let SliderValue::Single(value) = value {
                        chat.save_font(*value);
                    }
                }),
        )
        .child(text("A").text_2xl())
}

fn apply(value: &str) {
    nsh_core::element::set_color_scheme(match value {
        "light" => nsh_core::platform::Appearance::Light,
        "dark" => nsh_core::platform::Appearance::Dark,
        _ => nsh_core::platform::Appearance::System,
    });
}
