use nsh_core::prelude::*;

use super::{LinkNavigation, Panel, tr};
use crate::{
    conversation::parts,
    runtime,
    ui::{self, Typography as _},
};

pub(super) fn surface(router: LinkNavigation) -> impl IntoElement {
    let presence = crate::motion::Presence::modal(move || router.panel.with(Option::is_some));
    let shown = signal_with(None::<Panel>, SignalEquals::Never);
    effect(move || {
        if let Some(panel) = router.panel.get() {
            shown.set(Some(panel));
        }
    });
    when(
        move || presence.mounted(),
        move || {
            cleanup(move || {
                let _ = shown.try_set(None);
            });
            dynamic(move || {
                let panel = shown.get();
                untracked(|| match panel {
                    None => view().hidden().into_element(),
                    Some(panel) => {
                        let send = matches!(&panel, Panel::Send(_));
                        let compact = send
                            || matches!(
                                &panel,
                                Panel::Loading | Panel::Error | Panel::Unsupported(_)
                            );
                        let content = match panel {
                            Panel::Loading => parts::status(move || tr("Loading…", "加载中…"))
                                .justify_center()
                                .py_4()
                                .into_element(),
                            Panel::Error => view()
                                .gap_3()
                                .p_4()
                                .child(ui::modal_title(move || {
                                    tr("Couldn’t open this link", "无法打开此链接")
                                }))
                                .child(
                                    text(move || {
                                        tr(
                                            "Check the link and your connection, then try again.",
                                            "请检查链接和网络连接后重试。",
                                        )
                                    })
                                    .text_role(ui::TextRole::Body),
                                )
                                .into_element(),
                            Panel::Profile(id) => {
                                crate::conversation::profile(router.client(), id, move || {
                                    router.close()
                                })
                                .into_element()
                            }
                            Panel::Snapshot(reference) => {
                                crate::snapshot::screen(router.client(), reference).into_element()
                            }
                            Panel::Send(request) => {
                                super::send::screen(router, request).into_element()
                            }
                            Panel::Code(code, value) => {
                                code_panel(router, code, *value).into_element()
                            }
                            Panel::Unsupported(raw) => unsupported(raw).into_element(),
                        };
                        let mut body = view().flex_1().min_h_0();
                        if compact {
                            body = body.flex_auto().flex_grow(0.0);
                        }
                        let mut card = parts::tall_dialog_card()
                            .p(ui::space_lg())
                            .gap(ui::space_md())
                            .child(body.child(content));
                        if compact {
                            card = card.h(auto()).min_h_0();
                        }
                        if !send {
                            card = card.child(parts::dialog_actions().child(ui::action(
                                move || tr("Close", "关闭"),
                                move || router.close(),
                            )));
                        }
                        ui::modal(presence, move || router.close(), card).into_element()
                    }
                })
            })
        },
    )
}

fn code_panel(
    router: LinkNavigation,
    code: String,
    value: mixin_desktop_api::CodeResult,
) -> impl IntoElement {
    if matches!(value.kind.as_str(), "payment" | "multisig_request") {
        return payment_code(value, format!("mixin://codes/{code}")).into_element();
    }
    if value.kind != "conversation" {
        return unsupported(format!("mixin://codes/{code}")).into_element();
    }
    let members = value.participant_count;
    let title = value.conversation_name.unwrap_or_default();
    let Some(id) = value.conversation_id else {
        return text(move || tr("Invalid invitation", "无效邀请"))
            .text_role(ui::TextRole::Body)
            .into_element();
    };
    let joined = value.already_member;
    let code = stored(code);
    let id = stored(id);
    let failed = signal(false);
    let busy = signal(false);
    let ticket = router.request.get_untracked();
    view()
        .flex_1()
        .min_h_0()
        .gap_4()
        .p_4()
        .items_center()
        .child(text(title).text_role(ui::TextRole::Headline).text_center())
        .child(
            text(move || crate::i18n::current().members(members))
                .text_role(ui::TextRole::Body)
                .text_color(ui::fg_secondary),
        )
        .child(view().flex_row().gap_2().children(
            value.participant_avatars.into_iter().take(4).map(|avatar| {
                let name = stored(avatar.name);
                let url = stored(avatar.avatar_url);
                ui::avatar_view(
                    move || name.with(|name| name.clone().into()),
                    move || url.with(|url| url.clone()),
                    ui::AvatarSize::Medium,
                )
                .size_12()
            }),
        ))
        .child(when(failed, || {
            ui::error_text(move || {
                tr(
                    "Couldn’t join this group. Try again.",
                    "无法加入群组，请重试。",
                )
            })
        }))
        .child(
            ui::primary()
                .disabled(busy)
                .on_press(move || {
                    if router.panel.with_untracked(Option::is_none) || busy.get_untracked() {
                        return;
                    }
                    if joined {
                        let id = id.with(|id| id.clone());
                        router.close();
                        router.navigation.open_conversation(
                            router.client(),
                            id,
                            None,
                            router.failed,
                        );
                        return;
                    }
                    busy.set(true);
                    failed.set(false);
                    let client = router.client();
                    let code = code.with(|code| code.clone());
                    let command =
                        runtime::commit(
                            async move { client.conversation().join_group(code).await },
                        );
                    spawn(async move {
                        let result = command.await;
                        busy.set(false);
                        if !router.current(ticket) {
                            return;
                        }
                        match result {
                            Ok(id) => {
                                router.close();
                                router.navigation.open_conversation(
                                    router.client(),
                                    id,
                                    None,
                                    router.failed,
                                );
                            }
                            Err(_) => failed.set(true),
                        }
                    });
                })
                .child(text(move || {
                    if joined {
                        tr("Open chat", "打开聊天")
                    } else {
                        tr("Join group", "加入群组")
                    }
                })),
        )
        .into_element()
}

fn payment_code(value: mixin_desktop_api::CodeResult, uri: String) -> impl IntoElement {
    let qr = crate::qr::source(&uri, 320);
    let mut fields = vec![
        (
            tr("Amount", "金额"),
            format!(
                "{} {}",
                value.amount.unwrap_or_default(),
                value.asset_symbol.unwrap_or_default()
            ),
        ),
        (
            tr("Asset ID", "资产 ID"),
            value.asset_id.unwrap_or_default(),
        ),
        (tr("State", "状态"), value.state.unwrap_or_default()),
        (tr("Senders", "发送方"), value.senders.join("\n")),
        (tr("Receivers", "接收方"), value.receivers.join("\n")),
        (tr("Threshold", "签名阈值"), value.threshold.to_string()),
    ];
    if let Some(action) = value.action {
        fields.push((tr("Action", "操作"), action));
    }
    scroll().flex_1().min_h_0().child(
        view()
            .p_4()
            .gap_4()
            .child(ui::modal_title(move || tr("Payment details", "付款详情")))
            .child(
                text(move || {
                    tr(
                        "Complete this payment in Mixin on your phone.",
                        "请在手机 Mixin 中完成付款。",
                    )
                })
                .text_role(ui::TextRole::Body)
                .text_color(ui::fg_secondary),
            )
            .child(qr.map(|source| image(source).size_48().label("Payment code for phone")))
            .children(
                fields
                    .into_iter()
                    .filter(|(_, value)| !value.is_empty())
                    .map(|(label, value)| {
                        view()
                            .gap_1()
                            .child(
                                text(label)
                                    .text_role(ui::TextRole::Caption)
                                    .text_color(ui::fg_secondary),
                            )
                            .child(text(value).text_role(ui::TextRole::Body).select_text())
                    }),
            ),
    )
}

fn unsupported(raw: String) -> impl IntoElement {
    let image = crate::qr::source(&raw, 320);
    let raw = stored(raw);
    scroll().flex_shrink(1.0).min_h_0().child(
        view()
            .gap_4()
            .p_4()
            .items_center()
            .child(
                text(move || {
                    tr(
                        "Open this link in Mixin on your phone",
                        "请在手机 Mixin 中打开此链接",
                    )
                })
                .text_role(ui::TextRole::Body)
                .text_center(),
            )
            .child(image.map(|source| {
                nsh_core::prelude::image(source)
                    .size_60()
                    .flex_none()
                    .label("Mixin link QR code")
            }))
            .child(ui::action(
                move || tr("Copy link", "复制链接"),
                move || set_clipboard(raw.with(|raw| raw.clone())),
            )),
    )
}
