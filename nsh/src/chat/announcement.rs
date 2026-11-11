//! The group announcement banner and its conversation-scoped dismissal.

use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, SettingsClient};
use nsh_core::prelude::*;

use crate::{
    i18n, runtime,
    ui::{self, Typography},
};

pub(super) fn banner(account: Arc<AccountClient>, conversation_id: String) -> impl IntoElement {
    let i18n = i18n::current();
    let owner = Owner::current();
    let announcement = signal(String::new());
    let dismissed = signal(None::<String>);
    let busy = signal(false);
    let settings = crate::settings_client();
    let setting_key = stored(SettingsClient::dismissed_announcement_key(
        &account.account_id(),
        &conversation_id,
    ));
    let preferences = settings.clone();
    effect(move || {
        let settings = preferences.clone();
        let key = setting_key.with(|key| key.clone());
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(settings.subscribe_setting(key)));
            while let Some(value) = changes.next().await {
                if !owner.is_live() {
                    return;
                }
                match value {
                    Ok(value) => dismissed.set(value),
                    Err(error) => log::warn!("Could not load announcement dismissal: {error}"),
                }
            }
        });
    });
    effect(move || {
        let account = account.clone();
        let conversation_id = conversation_id.clone();
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
            loop {
                let client = account.clone();
                let id = conversation_id.clone();
                let result = runtime::run(async move {
                    client.conversation().local_conversation_detail(id).await
                })
                .await;
                if !owner.is_live() {
                    return;
                }
                match result {
                    Ok(detail) => announcement.set(detail.announcement),
                    Err(error) => log::warn!("Could not load announcement: {error}"),
                }
                loop {
                    let Some(change) = changes.next().await else {
                        return;
                    };
                    if change.reload_all || change.conversation_ids.contains(&conversation_id) {
                        break;
                    }
                }
            }
        });
    });
    let unread = computed(move || {
        announcement.with(|announcement| {
            dismissed.with(|dismissed| {
                SettingsClient::unread_announcement(announcement, dismissed.as_deref())
                    .map(str::to_owned)
            })
        })
    });
    let settings = stored(settings);
    when(
        move || unread.with(Option::is_some),
        move || {
            ui::glass(view())
                .flex_row()
                .items_center()
                .gap_2()
                .mx_3()
                .mb_2()
                .px(ui::space_md())
                .py_2()
                .rounded(ui::radius_lg())
                .child(
                    text(move || unread.get().unwrap_or_default())
                        .text_role(ui::TextRole::Body)
                        .number_of_lines(2)
                        .flex_1()
                        .min_w_0(),
                )
                .child(
                    ui::icon_button()
                        .size(ui::pin_close_size())
                        .px_0()
                        .py_0()
                        .disabled(busy)
                        .accessibility_label(move || Some(i18n.tr("Close", "关闭").into()))
                        .on_press(move || {
                            if busy.get_untracked() {
                                return;
                            }
                            let settings = settings.with(|settings| settings.clone());
                            let key = setting_key.with(|key| key.clone());
                            let value = announcement.get_untracked();
                            busy.set(true);
                            let command = runtime::commit(async move {
                                settings.set_setting(key, Some(value)).await
                            });
                            spawn(async move {
                                let result = command.await;
                                if owner.is_live() {
                                    busy.set(false);
                                    if let Err(error) = result {
                                        log::warn!("Could not dismiss announcement: {error}");
                                    }
                                }
                            });
                        })
                        .child(ui::icon(ui::Icon::Close).size(ui::icon_sm())),
                )
        },
    )
}
