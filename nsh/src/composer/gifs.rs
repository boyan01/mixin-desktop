use mixin_desktop_api::{DesktopClient, GifItem};
use nsh_base::theme;
use nsh_core::prelude::*;

use super::{stickers::State, tr};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

pub(super) fn panel(state: State) -> impl IntoElement {
    let i18n = i18n::current();
    let query = signal(String::new());
    let offset = signal(0_u32);
    let rows = signal_with(Vec::<GifItem>::new(), SignalEquals::Never);
    let available = signal(true);
    let loading = signal(true);
    let failed = signal(false);
    let more = signal(false);
    let retry = signal(0_u64);
    effect(move || {
        let query = query.get();
        let offset = offset.get();
        retry.get();
        loading.set(true);
        failed.set(false);
        spawn(async move {
            match runtime::run(async move {
                let desktop = DesktopClient::open().await?;
                if !desktop.gif_search_available() {
                    return Ok(None);
                }
                desktop.search_gifs(query, offset, 33).await.map(Some)
            })
            .await
            {
                Ok(Some(mut items)) => {
                    available.set(true);
                    more.set(items.len() > 32);
                    items.truncate(32);
                    rows.set(items);
                }
                Ok(None) => {
                    available.set(false);
                    rows.set(Vec::new());
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    view()
        .flex_1()
        .min_h_0()
        .gap_2()
        .child(
            ui::field()
                .flex_none()
                .label(move || tr(i18n, "Search GIFs", "搜索 GIF"))
                .placeholder(move || tr(i18n, "Search GIPHY…", "搜索 GIPHY…").to_owned())
                .value(move || query.get())
                .readonly(move || state.busy.get() || !available.get())
                .disabled(move || !available.get())
                .auto_focus(true)
                .on_change(move |value: &str| {
                    batch(|| {
                        query.set(value.to_owned());
                        offset.set(0);
                    })
                }),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .child(when(
                    move || !loading.get() && !failed.get() && !available.get(),
                    move || {
                        view().flex_1().items_center().justify_center().child(
                            text(move || {
                                tr(i18n, "GIF search is unavailable.", "GIF 搜索暂不可用。")
                            })
                            .text_sm()
                            .text_color(move || theme().muted_foreground),
                        )
                    },
                ))
                .child(when(loading, move || {
                    view()
                        .flex_1()
                        .flex_row()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(ui::busy_indicator(
                            move || loading.get(),
                            move || tr(i18n, "Loading GIFs…", "正在加载 GIF…").into(),
                        ))
                        .child(text(move || tr(i18n, "Loading GIFs…", "正在加载 GIF…")).text_sm())
                }))
                .child(when(
                    move || !loading.get() && failed.get(),
                    move || {
                        view()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .child(
                                text(move || i18n.text(Key::CommandFailed))
                                    .text_sm()
                                    .text_color(move || theme().destructive),
                            )
                            .child(
                                ui::action(
                                    move || i18n.text(Key::Retry),
                                    move || retry.update(|v| *v += 1),
                                )
                                .disabled(state.busy),
                            )
                    },
                ))
                .child(when(
                    move || {
                        !loading.get()
                            && !failed.get()
                            && available.get()
                            && rows.with(Vec::is_empty)
                    },
                    move || {
                        view().flex_1().items_center().justify_center().child(
                            text(move || tr(i18n, "No GIFs found", "未找到 GIF"))
                                .text_sm()
                                .text_color(move || theme().muted_foreground),
                        )
                    },
                ))
                .child(
                    virtual_list(
                        move || rows.with(Vec::len),
                        move |at| {
                            ui::command()
                                .size_24()
                                .px_1()
                                .py_1()
                                .disabled(move || state.busy.get() || loading.get())
                                .accessibility_label("Send GIF")
                                .child(
                                    image(move || {
                                        rows.with(|rows| {
                                            rows.get(at.index())
                                                .map(|g| g.preview_url.clone())
                                                .unwrap_or_default()
                                        })
                                    })
                                    .size_20()
                                    .object_fit(ResizeMode::Contain),
                                )
                                .on_press(move || {
                                    if state.busy.get_untracked() {
                                        return;
                                    }
                                    let Some(gif) = rows.with(|rows| rows.get(at.index()).cloned())
                                    else {
                                        return;
                                    };
                                    let client = state.account.with(|a| a.clone());
                                    let id = state.conversation.with(|id| id.clone());
                                    let sent_quote = state
                                        .quote
                                        .with(|q| q.as_ref().map(|m| m.message_id.clone()));
                                    state.busy.set(true);
                                    spawn(async move {
                                        state.failed.set(
                                            runtime::commit(async move {
                                                client
                                                    .message()
                                                    .send_remote_image(
                                                        id,
                                                        gif.url,
                                                        gif.preview_url,
                                                        Some(gif.width),
                                                        Some(gif.height),
                                                        "image/gif".into(),
                                                        false,
                                                    )
                                                    .await
                                            })
                                            .await
                                            .is_err(),
                                        );
                                        state.busy.set(false);
                                        if !state.failed.get_untracked() {
                                            state.clear_quote(sent_quote);
                                            state.open.set(false);
                                        }
                                    });
                                })
                        },
                    )
                    .keys(move || {
                        rows.with(|rows| rows.iter().map(|g| g.id.clone()).collect::<Vec<_>>())
                    })
                    .columns(move || {
                        (((window().width.0 * 0.95).min(464.0) - 24.0) / 96.0)
                            .floor()
                            .max(1.0) as u32
                    })
                    .estimated_item_size(px(96.0))
                    .flex_1()
                    .min_h_0()
                    .style_when(
                        move || {
                            loading.get()
                                || failed.get()
                                || !available.get()
                                || rows.with(Vec::is_empty)
                        },
                        |style| style.hidden().invisible(),
                    ),
                ),
        )
        .child(
            view()
                .flex_none()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text("GIPHY")
                        .text_xs()
                        .text_color(move || theme().muted_foreground)
                        .flex_1(),
                )
                .child(when(
                    move || available.get() && !failed.get() && (offset.get() > 0 || more.get()),
                    move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::action(
                                    move || tr(i18n, "Previous page", "上一页"),
                                    move || offset.update(|v| *v = v.saturating_sub(32)),
                                )
                                .disabled(move || {
                                    offset.get() == 0 || loading.get() || state.busy.get()
                                }),
                            )
                            .child(
                                ui::action(
                                    move || tr(i18n, "Next page", "下一页"),
                                    move || offset.update(|v| *v += 32),
                                )
                                .disabled(move || !more.get() || loading.get() || state.busy.get()),
                            )
                    },
                )),
        )
}
