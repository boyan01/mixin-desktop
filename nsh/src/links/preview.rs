use super::{parse::Payload, tr};
use crate::{
    runtime,
    ui::{self, Typography as _},
};
use mixin_desktop_api::AccountClient;
use nsh_base::text_view;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(super) fn render(
    account: Arc<AccountClient>,
    payload: Payload,
    ready: Signal<bool>,
) -> AnyElement {
    match payload {
        Payload::Text(content) => text(content)
            .text_role(ui::TextRole::Body)
            .select_text()
            .into_element(),
        Payload::Post(content) => text_view()
            .markdown(Some(SharedString::from(content)))
            .selectable(true)
            .text_role(ui::TextRole::Body)
            .into_element(),
        Payload::Image(url) => image(url)
            .w_full()
            .h_64()
            .object_fit(ResizeMode::Contain)
            .label("Image to send")
            .into_element(),
        Payload::Card(value) => {
            let mut view = view().gap_3();
            if let Some(url) = value
                .get("cover_url")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
            {
                view = view.child(
                    image(url.to_owned())
                        .w_full()
                        .h_32()
                        .object_fit(ResizeMode::Cover),
                );
            }
            view.child(
                text(
                    value
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                )
                .text_role(ui::TextRole::Body)
                .text_emphasis(),
            )
            .child(
                text(
                    value
                        .get("description")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                )
                .text_role(ui::TextRole::Body)
                .select_text(),
            )
            .into_element()
        }
        Payload::Sticker(id) => {
            let asset = signal(String::new());
            let failed = signal(false);
            let retry = signal(0_u64);
            let account = stored(account);
            let id = stored(id);
            effect(move || {
                let _retry = retry.get();
                ready.set(false);
                failed.set(false);
                let client = account.with(|account| account.clone());
                let id = id.with(|id| id.clone());
                spawn(async move {
                    match runtime::run(async move { client.sticker().sticker_detail(id).await })
                        .await
                    {
                        Ok(sticker) => {
                            asset.set(sticker.sticker.asset_url);
                            ready.set(true);
                        }
                        Err(_) => failed.set(true),
                    }
                });
            });
            view()
                .gap_2()
                .child(
                    image(move || asset.get())
                        .size_40()
                        .object_fit(ResizeMode::Contain)
                        .label("Sticker to send"),
                )
                .child(when(failed, move || {
                    view()
                        .gap_2()
                        .child(
                            text(move || tr("Sticker preview unavailable", "无法预览贴纸"))
                                .text_role(ui::TextRole::Body),
                        )
                        .child(ui::action(
                            move || tr("Retry preview", "重试预览"),
                            move || retry.update(|retry| *retry += 1),
                        ))
                }))
                .into_element()
        }
        Payload::Contact(id) => {
            let profile = signal_with(
                None::<mixin_desktop_api::UserProfileItem>,
                SignalEquals::Never,
            );
            let failed = signal(false);
            let retry = signal(0_u64);
            let account = stored(account);
            let id = stored(id);
            effect(move || {
                let _retry = retry.get();
                ready.set(false);
                failed.set(false);
                let client = account.with(|account| account.clone());
                let id = id.with(|id| id.clone());
                spawn(async move {
                    match runtime::run(async move { client.user().refresh_user_profile(id).await })
                        .await
                    {
                        Ok(Some(value)) => {
                            profile.set(Some(value));
                            ready.set(true);
                        }
                        _ => failed.set(true),
                    }
                });
            });
            view()
                .gap_2()
                .child(ui::avatar_view(
                    move || {
                        profile.with(|profile| {
                            profile
                                .as_ref()
                                .map(|profile| profile.full_name.clone())
                                .unwrap_or_default()
                                .into()
                        })
                    },
                    move || {
                        profile.with(|profile| {
                            profile
                                .as_ref()
                                .map(|profile| profile.avatar_url.clone())
                                .unwrap_or_default()
                        })
                    },
                    ui::AvatarSize::Medium,
                ))
                .child(
                    text(move || {
                        profile.with(|profile| {
                            profile
                                .as_ref()
                                .map(|profile| profile.full_name.clone())
                                .unwrap_or_else(|| tr("Contact", "联系人").into())
                        })
                    })
                    .text_role(ui::TextRole::Body),
                )
                .child(
                    text(move || {
                        profile.with(|profile| {
                            profile
                                .as_ref()
                                .map(|profile| profile.identity_number.clone())
                                .unwrap_or_default()
                        })
                    })
                    .text_role(ui::TextRole::Caption),
                )
                .child(when(failed, move || {
                    view()
                        .gap_2()
                        .child(
                            text(move || tr("Contact preview unavailable", "无法预览联系人"))
                                .text_role(ui::TextRole::Body),
                        )
                        .child(ui::action(
                            move || tr("Retry preview", "重试预览"),
                            move || retry.update(|retry| *retry += 1),
                        ))
                }))
                .into_element()
        }
    }
}
