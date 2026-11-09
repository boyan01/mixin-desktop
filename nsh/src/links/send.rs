use super::{
    LinkNavigation,
    parse::{Payload, Recipient, Send},
    tr,
};
use crate::{
    runtime,
    ui::parts,
    ui::{self, Typography as _},
};
use mixin_desktop_api::ClientError;
use nsh_core::prelude::*;

pub(super) fn screen(router: LinkNavigation, request: Send) -> impl IntoElement {
    let recipient = signal_with(request.recipient, SignalEquals::Never);
    let payload = stored(request.payload);
    let choosing = signal(false);
    let loading = signal(false);
    let name = signal(String::new());
    let ready = signal(false);
    let content_ready = signal(true);
    let busy = signal(false);
    let failed = signal(false);
    let load_failed = signal(false);
    let revision = signal(0_u64);
    let ticket = router.request.get_untracked();
    let cancel_button = element_ref();
    let cancel_choosing = move || {
        choosing.set(false);
        cancel_button.focus();
    };
    effect(move || {
        let recipient = recipient.get();
        let _revision = revision.get();
        let client = router.client();
        ready.set(false);
        load_failed.set(false);
        if matches!(recipient, Recipient::Choose) {
            loading.set(false);
            name.set(tr("Choose recipient", "选择接收方").into());
            return;
        }
        loading.set(true);
        spawn(async move {
            let result = runtime::run(async move {
                match recipient {
                    Recipient::User(id) => client
                        .user()
                        .refresh_user_profile(id)
                        .await?
                        .map(|user| user.full_name)
                        .ok_or(ClientError::NotFound),
                    Recipient::Conversation(id) => client
                        .conversation()
                        .resolve_conversation_link(id, None)
                        .await
                        .map(|row| row.name),
                    Recipient::Choose => {
                        Err(ClientError::InvalidArgument("Choose a recipient".into()))
                    }
                }
            })
            .await;
            match result {
                Ok(value) => {
                    name.set(value);
                    ready.set(true);
                }
                Err(_) => load_failed.set(true),
            }
            loading.set(false);
        });
    });
    let send = move || {
        if router.panel.with_untracked(Option::is_none)
            || router.request.get_untracked() != ticket
            || busy.get_untracked()
            || !ready.get_untracked()
        {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = router.client();
        let recipient = recipient.get_untracked();
        let payload = payload.with(|payload| payload.clone());
        let task = runtime::commit(async move {
            let id = match recipient {
                Recipient::User(user) => client.conversation().open_user_conversation(user).await?,
                Recipient::Conversation(id) => id,
                Recipient::Choose => {
                    return Err(ClientError::InvalidArgument("Choose a recipient".into()));
                }
            };
            let message = match payload {
                Payload::Text(content) => {
                    client
                        .message()
                        .send_text(id.clone(), content, None, false)
                        .await?
                }
                Payload::Post(content) => client.message().send_post(id.clone(), content).await?,
                Payload::Sticker(sticker) => {
                    client.message().send_sticker(id.clone(), sticker).await?
                }
                Payload::Contact(user) => {
                    client
                        .message()
                        .send_contact(id.clone(), user, None, false)
                        .await?
                }
                Payload::Image(url) => {
                    client
                        .message()
                        .send_remote_image(
                            id.clone(),
                            url.clone(),
                            url,
                            None,
                            None,
                            String::new(),
                            false,
                        )
                        .await?
                }
                Payload::Card(value) => {
                    client
                        .message()
                        .send_app_card(id.clone(), value.to_string())
                        .await?
                }
            };
            Ok((id, message))
        });
        spawn(async move {
            let result = task.await;
            busy.set(false);
            if !router.current(ticket) {
                return;
            }
            match result {
                Ok((id, message)) => {
                    router.close();
                    router.navigation.open_conversation(
                        router.client(),
                        id,
                        Some(message),
                        router.failed,
                    );
                }
                Err(_) => failed.set(true),
            }
        });
    };
    let payload_view = stored(Box::new(move || {
        super::preview::render(
            router.client(),
            payload.with(|payload| payload.clone()),
            content_ready,
        )
        .into_element()
    }) as Box<dyn Fn() -> AnyElement>);
    let picker = stored(Box::new(move || {
        super::recipient::screen(router.client(), cancel_choosing, move |value| {
            recipient.set(value);
            cancel_choosing();
        })
        .into_element()
    }) as Box<dyn Fn() -> AnyElement>);
    content(
        choosing,
        busy,
        name,
        loading,
        load_failed,
        revision,
        ready,
        content_ready,
        failed,
        cancel_button,
        cancel_choosing,
        send,
        move || router.close(),
        payload_view,
        picker,
    )
}
#[allow(clippy::too_many_arguments)]
fn content(
    choosing: Signal<bool>,
    busy: Signal<bool>,
    name: Signal<String>,
    loading: Signal<bool>,
    load_failed: Signal<bool>,
    revision: Signal<u64>,
    ready: Signal<bool>,
    content_ready: Signal<bool>,
    failed: Signal<bool>,
    cancel_button: ElementRef,
    cancel_choosing: impl Fn() + Copy + 'static,
    send: impl Fn() + Copy + 'static,
    close: impl Fn() + Copy + 'static,
    payload_view: Stored<Box<dyn Fn() -> AnyElement>>,
    picker: Stored<Box<dyn Fn() -> AnyElement>>,
) -> impl IntoElement {
    view()
        .flex_shrink(1.0)
        .min_h_0()
        .gap_3()
        .child(
            view()
                .flex_none()
                .flex_row()
                .items_center()
                .gap_2()
                .child(when(choosing, move || {
                    ui::icon_button()
                        .accessibility_label(move || Some(tr("Back", "返回").into()))
                        .on_press(cancel_choosing)
                        .child(ui::icon(ui::Icon::ArrowLeft))
                }))
                .child(
                    ui::dialog_title(move || {
                        if choosing.get() {
                            tr("Choose recipient", "选择接收方")
                        } else {
                            tr("Send message", "发送消息")
                        }
                    })
                    .flex_1()
                    .min_w_0(),
                )
                .child(
                    ui::icon_button()
                        .accessibility_label(move || Some(tr("Close", "关闭").into()))
                        .disabled(busy)
                        .on_press(close)
                        .child(ui::icon(ui::Icon::Close)),
                ),
        )
        .child(when(
            move || !choosing.get(),
            move || {
                view()
                    .flex_shrink(1.0)
                    .min_h_0()
                    .gap_3()
                    .child(
                        text(move || name.get())
                            .flex_none()
                            .text_role(ui::TextRole::Body),
                    )
                    .child(when(loading, || {
                        parts::status(move || tr("Loading recipient…", "正在加载接收方…"))
                            .flex_none()
                    }))
                    .child(when(load_failed, move || {
                        parts::failure(
                            move || tr("Couldn’t load this recipient.", "无法加载此接收方。"),
                            move || revision.update(|revision| *revision += 1),
                        )
                        .flex_none()
                    }))
                    .child(
                        scroll()
                            .flex_shrink(1.0)
                            .min_h_0()
                            .max_h_60()
                            .bg(ui::surface_card)
                            .rounded(ui::radius_md())
                            .p_4()
                            .child(payload_view.with(|render| render())),
                    )
                    .child(
                        ui::action(
                            move || tr("Choose recipient…", "选择接收方…"),
                            move || choosing.set(true),
                        )
                        .flex_none()
                        .disabled(busy),
                    )
            },
        ))
        .child(when(choosing, move || picker.with(|render| render())))
        .child(when(failed, move || {
            ui::error_text(move || {
                tr(
                    "Couldn’t send this message. Try again.",
                    "无法发送此消息，请重试。",
                )
            })
        }))
        .child(
            parts::dialog_actions()
                .child(
                    ui::action(
                        move || tr("Cancel", "取消"),
                        move || {
                            if choosing.get_untracked() {
                                cancel_choosing();
                            } else if !busy.get_untracked() {
                                close();
                            }
                        },
                    )
                    .element_ref(&cancel_button)
                    .disabled(busy),
                )
                .child(when(
                    move || !choosing.get(),
                    move || {
                        ui::primary()
                            .disabled(move || {
                                busy.get() || choosing.get() || !ready.get() || !content_ready.get()
                            })
                            .on_press(send)
                            .child(text(move || {
                                if busy.get() {
                                    tr("Sending…", "正在发送…")
                                } else {
                                    tr("Send", "发送")
                                }
                            }))
                    },
                )),
        )
}
#[cfg(debug_assertions)]
pub(crate) fn detached(
    users: Vec<mixin_desktop_api::UserProfileItem>,
    state: Signal<&'static str>,
    sticker: String,
    contact: mixin_desktop_api::UserProfileItem,
) -> impl IntoElement {
    let choosing = signal(false);
    let failed = signal(false);
    effect(move || failed.set(state.get() == "Error"));
    let sticker = stored(sticker);
    let contact = stored(contact);
    let users = stored(users);
    content(
        choosing,
        signal(false),
        signal("Design review".into()),
        signal(false),
        signal(false),
        signal(0),
        signal(true),
        signal(true),
        failed,
        element_ref(),
        move || choosing.set(false),
        || {},
        || {},
        stored(Box::new(move || {
            dynamic(move || {
                let mode = state.get();
                untracked(move || match mode {
                    "Sticker" | "Sticker error" => {
                        let failed = signal(false);
                        effect(move || failed.set(state.get() == "Sticker error"));
                        super::preview::sticker_view(signal(sticker.get()), failed, || {})
                            .into_element()
                    }
                    "Contact" | "Contact error" => {
                        let failed = signal(false);
                        effect(move || failed.set(state.get() == "Contact error"));
                        super::preview::contact_view(
                            signal_with(Some(contact.get()), SignalEquals::Never),
                            failed,
                            || {},
                        )
                        .into_element()
                    }
                    _ => super::preview::text_payload("Please review these button styles.".into())
                        .into_element(),
                })
            })
            .into_element()
        }) as Box<dyn Fn() -> AnyElement>),
        stored(Box::new(move || {
            crate::recipients::detached(users.get(), |_| false, move |_| choosing.set(false), false)
                .into_element()
        }) as Box<dyn Fn() -> AnyElement>),
    )
}
