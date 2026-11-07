use crate::i18n::{self, Key};
use crate::{
    Phase, qr, runtime,
    ui::{self, Typography as _},
};
use mixin_desktop_api::{ClientError, ClientResult, DesktopClient, LoginClient};
use nsh_core::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
enum LoginPhase {
    Creating,
    Waiting(SharedString),
    Failed,
}

pub(crate) fn screen(desktop: DesktopClient, phase: Signal<Phase>) -> impl IntoElement {
    let i18n = i18n::current();
    let state = signal(LoginPhase::Creating);
    let retry = signal(0_u64);
    let active = Rc::new(RefCell::new(None::<Arc<LoginClient>>));
    let cancel = active.clone();
    cleanup(move || {
        if let Some(login) = cancel.borrow_mut().take() {
            login.cancel();
        }
    });
    effect(move || {
        let _revision = retry.get();
        let desktop = desktop.clone();
        let active = active.clone();
        state.set(LoginPhase::Creating);
        spawn(async move {
            if let Err(error) = authorize(desktop, state, phase, active).await {
                log::warn!("Sign-in failed: {error}");
                state.set(LoginPhase::Failed);
            }
        });
    });
    view()
        .flex_1()
        .bg(ui::surface_landing)
        .child(ui::title_bar())
        .child(
            view().flex_1().items_center().justify_center().p_4().child(
                view()
                    .w(ui::login_width)
                    .max_w(relative(1.0))
                    .h(ui::login_height())
                    .bg(ui::surface_popover)
                    .rounded(ui::radius_login())
                    .shadow_lg()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .p_8()
                    .child(
                        view()
                            .size_40()
                            .rounded(ui::radius_md())
                            .overflow_hidden()
                            .items_center()
                            .justify_center()
                            .child(dynamic(move || match state.get() {
                                LoginPhase::Creating => view()
                                    .size_40()
                                    .gap_3()
                                    .items_center()
                                    .justify_center()
                                    .child(ui::busy_indicator(
                                        || true,
                                        move || i18n.text(Key::Preparing).into(),
                                    ))
                                    .child(
                                        text(move || i18n.text(Key::Preparing))
                                            .text_role(ui::TextRole::Body)
                                            .text_center()
                                            .text_color(ui::fg_secondary),
                                    )
                                    .into_element(),
                                LoginPhase::Waiting(source) => image(source)
                                    .size_40()
                                    .label(move || i18n.text(Key::ScanTitle))
                                    .into_element(),
                                LoginPhase::Failed => view()
                                    .size_40()
                                    .gap_3()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        ui::error_text(move || i18n.text(Key::LoginFailed))
                                            .text_center(),
                                    )
                                    .child(ui::action(
                                        move || i18n.text(Key::Retry),
                                        move || retry.update(|v| *v += 1),
                                    ))
                                    .into_element(),
                            })),
                    )
                    .child(text(move || i18n.text(Key::ScanTitle)).text_role(ui::TextRole::Label))
                    .child(
                        view()
                            .gap_1()
                            .max_w_96()
                            .child(
                                text(move || format!("1. {}", i18n.text(Key::ScanInstructions1)))
                                    .text_role(ui::TextRole::Body)
                                    .text_color(ui::fg_secondary),
                            )
                            .child(
                                text(move || format!("2. {}", i18n.text(Key::ScanInstructions2)))
                                    .text_role(ui::TextRole::Body)
                                    .text_color(ui::fg_secondary),
                            ),
                    ),
            ),
        )
        .child(
            view()
                .flex_row()
                .justify_between()
                .items_center()
                .px_4()
                .py_3()
                .child(i18n.picker())
                .child(
                    text(env!("CARGO_PKG_VERSION"))
                        .text_role(ui::TextRole::Body)
                        .text_color(ui::fg_secondary),
                ),
        )
}

async fn authorize(
    desktop: DesktopClient,
    state: Signal<LoginPhase>,
    phase: Signal<Phase>,
    active: Rc<RefCell<Option<Arc<LoginClient>>>>,
) -> ClientResult<()> {
    loop {
        let start = desktop.clone();
        let login = runtime::run(async move { start.begin_login().await.map(Arc::new) }).await?;
        if let Some(previous) = active.replace(Some(login.clone())) {
            previous.cancel();
        }
        let url = login.auth_url();
        let source = runtime::run(async move {
            qr::source(&url, 256)
                .ok_or_else(|| ClientError::Internal("Could not render the sign-in code".into()))
        })
        .await?;
        state.set(LoginPhase::Waiting(source.into()));
        log::info!("NSH sign-in QR code ready");
        match runtime::run(async move {
            let account = Arc::new(login.wait().await?);
            runtime::retain_account(account.clone());
            Ok(account)
        })
        .await
        {
            Ok(account) => {
                log::info!("NSH sign-in completed");
                phase.set(Phase::SignedIn(account));
                return Ok(());
            }
            Err(error) if error.to_string().contains("authorization timed out") => continue,
            Err(error) => {
                if error.to_string().contains("login_provisioning_error:") {
                    runtime::run(async move { desktop.abort_saved_login().await }).await?;
                }
                return Err(error);
            }
        }
    }
}
