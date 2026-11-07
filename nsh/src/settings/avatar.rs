use super::{Feedback, tr};
use crate::{i18n, runtime, ui};
use mixin_desktop_api::{AccountClient, AccountProfile};
use nsh_core::prelude::*;
use std::sync::Arc;

pub(super) fn picker(
    account: Arc<AccountClient>,
    profile: Signal<AccountProfile>,
    feedback: Feedback,
    signing_out: Signal<bool>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let picker = crate::avatar::Picker::new();
    let account = stored(account);
    view()
        .child(
            ui::command()
                .justify_start()
                .disabled(move || feedback.busy.get() || signing_out.get() || picker.busy())
                .child(text(move || tr(i18n, "Change photo…", "更换头像…")))
                .on_press(move || {
                    feedback.failed.set(false);
                    spawn(async move {
                        if picker.pick().await.is_err() {
                            feedback.finish(false);
                        }
                    });
                }),
        )
        .child(picker.surface(false, move |encoded| {
            feedback.start();
            let account = account.with(|account| account.clone());
            let task = runtime::commit(async move { account.update_avatar(encoded).await });
            spawn(async move {
                match task.await {
                    Ok(next) => {
                        profile.set(next);
                        feedback.finish(true);
                    }
                    Err(_) => feedback.finish(false),
                }
            });
        }))
}
