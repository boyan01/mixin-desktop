use crate::{
    i18n::{self, Key},
    ui,
    ui::parts,
};
use mixin_desktop_api::{DesktopClient, SettingsClient};
use nsh_core::prelude::*;

pub(in crate::settings) async fn client() -> mixin_desktop_api::ClientResult<SettingsClient> {
    Ok(DesktopClient::open().await?.settings())
}

#[derive(Clone, Copy)]
pub(in crate::settings) struct Feedback {
    /// The page, which outlives the list cell or dialog that starts a save.
    pub(in crate::settings) owner: Owner,
    pub(in crate::settings) busy: Signal<bool>,
    pub(in crate::settings) failed: Signal<bool>,
}

impl Feedback {
    pub(in crate::settings) fn new() -> Self {
        Self {
            owner: Owner::current(),
            busy: signal(false),
            failed: signal(false),
        }
    }
    pub(in crate::settings) fn start(self) {
        batch(|| {
            self.busy.set(true);
            self.failed.set(false);
        });
    }
    pub(in crate::settings) fn finish(self, succeeded: bool) {
        batch(|| {
            self.busy.set(false);
            self.failed.set(!succeeded);
        });
    }
    pub(in crate::settings) fn view(
        self,
        busy_label: impl Fn() -> &'static str + Copy + 'static,
    ) -> impl IntoElement {
        let i18n = i18n::current();
        // Mounted only with a message: an empty child would double the page's gap.
        when(
            move || self.busy.get() || self.failed.get(),
            move || {
                view()
                    .gap(ui::space_sm())
                    .child(when(self.busy, move || parts::status(busy_label)))
                    .child(when(self.failed, move || {
                        ui::error_text(move || i18n.text(Key::CommandFailed))
                    }))
            },
        )
    }
}
