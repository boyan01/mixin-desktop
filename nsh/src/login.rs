use crate::i18n::{self, Key};
use crate::{Phase, runtime, ui};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat, Luma};
use mixin_desktop_api::{ClientError, ClientResult, DesktopClient, LoginClient};
use nsh_base::theme;
use nsh_core::prelude::*;
use qrcode::QrCode;
use std::cell::RefCell;
use std::io::Cursor;
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
        .bg(move || theme().color("landing-background").unwrap_or(theme().muted))
        .child(view().h_12().window_drag(true))
        .child(
            view().flex_1().items_center().justify_center().p_4().child(
                view()
                    .w(ui::login_width)
                    .max_w(relative(1.0))
                    .h(move || px(theme().number("login-height").unwrap_or(418.0)))
                    .bg(move || theme().popover)
                    .rounded(move || px(theme().number("radius-login").unwrap_or(13.0)))
                    .shadow_lg()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .p_8()
                    .child(
                        view()
                            .size_40()
                            .rounded_lg()
                            .overflow_hidden()
                            .items_center()
                            .justify_center()
                            .child(dynamic(move || match state.get() {
                                LoginPhase::Creating => view()
                                    .size_40()
                                    .gap_3()
                                    .items_center()
                                    .justify_center()
                                    .child(ui::icon(ui::Icon::Chat).size_8())
                                    .child(text(move || i18n.text(Key::Preparing)).text_sm())
                                    .into_element(),
                                LoginPhase::Waiting(source) => image(source)
                                    .size_40()
                                    .label(move || i18n.text(Key::ScanTitle))
                                    .into_element(),
                                LoginPhase::Failed => ui::command()
                                    .size_40()
                                    .on_press(move || retry.update(|v| *v += 1))
                                    .child(text(move || i18n.text(Key::Retry)))
                                    .into_element(),
                            })),
                    )
                    .child(text(move || i18n.text(Key::ScanTitle)).text_base())
                    .child(
                        view()
                            .gap_1()
                            .max_w_96()
                            .child(
                                text(move || format!("1. {}", i18n.text(Key::ScanInstructions1)))
                                    .text_sm()
                                    .text_color(move || theme().muted_foreground),
                            )
                            .child(
                                text(move || format!("2. {}", i18n.text(Key::ScanInstructions2)))
                                    .text_sm()
                                    .text_color(move || theme().muted_foreground),
                            ),
                    )
                    .child(when(
                        move || state.get() == LoginPhase::Failed,
                        move || {
                            text(move || i18n.text(Key::LoginFailed))
                                .text_xs()
                                .text_color(move || theme().destructive)
                        },
                    )),
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
                        .text_sm()
                        .text_color(move || theme().muted_foreground),
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
        let source = runtime::run(async move { qr_source(&url) }).await?;
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

fn qr_source(url: &str) -> ClientResult<String> {
    let code = QrCode::new(url).map_err(|error| ClientError::Internal(error.to_string()))?;
    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(256, 256)
        .quiet_zone(true)
        .build();
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(image)
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|error| ClientError::Internal(error.to_string()))?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    ))
}
