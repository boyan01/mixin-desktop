//! Native Rust presentation over the platform-neutral Mixin client API.

mod appearance;
mod avatar;
mod bot;
mod chat;
mod composer;
mod conversation;
mod conversation_list;
mod create;
mod drafts;
mod home;
mod i18n;
mod inspector;
mod links;
mod login;
mod motion;
mod navigation;
mod notifications;
mod platform;
mod playback;
mod presentation;
mod qr;
mod recipients;
pub mod runtime;
mod search;
mod security;
mod settings;
mod shell;
mod sidebar;
mod snapshot;
mod sticker;
mod transfer;
pub use shell::{Launch, NotificationTarget, prepare_launch};
mod ui;

use crate::ui::Typography as _;
use mixin_desktop_api::{
    AccountClient, ClientError, ClientResult, DesktopClient, MediaClient, SettingsClient,
};
use nsh_core::prelude::*;
use std::sync::Arc;

#[native_module("mixin/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[derive(Clone)]
pub(crate) enum Phase {
    Launching,
    SignedOut(DesktopClient),
    SignedIn(Arc<AccountClient>),
    Failed,
}

/// The application surface. Business services remain outside the UI tree.
#[component]
pub fn App() -> impl IntoElement {
    nsh_core::element::desktop::set_title_bar(
        nsh_core::desktop::TitleBarStyle::Hidden,
        Some(ui::title_bar_requested_height()),
    );
    let i18n = i18n::I18n::new();
    provide_context(i18n);
    let shell = shell::mount();
    provide_context(shell);
    let notifications = notifications::mount();
    provide_context(notifications);
    let settings = stored(None::<SettingsClient>);
    provide_context(settings);
    let media = stored(None::<Arc<MediaClient>>);
    provide_context(media);
    let phase = signal_with(Phase::Launching, SignalEquals::Never);
    let retry = signal(0_u64);
    cleanup(runtime::stop);
    effect(move || {
        let _revision = retry.get();
        phase.set(Phase::Launching);
        spawn(async move {
            let result = launch(i18n, settings, media).await;
            phase.set(result.unwrap_or_else(|error| {
                log::warn!("Opening the desktop runtime failed: {error}");
                Phase::Failed
            }));
        });
    });
    view()
        .flex_1()
        .bg(ui::surface_pane)
        .text_color(ui::fg_primary)
        .child(shell::surface(shell))
        .child(dynamic(move || {
            let current = phase.get();
            untracked(|| match current {
                Phase::Launching => status_screen(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(ui::busy_indicator(
                            || true,
                            move || i18n.text(i18n::Key::Opening).into(),
                        ))
                        .child(
                            text(move || i18n.text(i18n::Key::Opening))
                                .text_role(ui::TextRole::Body)
                                .text_color(ui::fg_secondary),
                        ),
                )
                .into_element(),
                Phase::SignedOut(desktop) => login::screen(desktop, phase).into_element(),
                Phase::SignedIn(account) => ui::owned(account.account_id(), move || {
                    home::screen(account.clone(), move || retry.update(|value| *value += 1))
                })
                .into_element(),
                Phase::Failed => status_screen(
                    view()
                        .gap_3()
                        .items_center()
                        .child(
                            text(move || i18n.text(i18n::Key::OpenFailed))
                                .text_role(ui::TextRole::Headline),
                        )
                        .child(
                            text(move || i18n.text(i18n::Key::OpenFailedBody))
                                .text_role(ui::TextRole::Body)
                                .text_center()
                                .text_color(ui::fg_secondary),
                        )
                        .child(ui::action(
                            move || i18n.text(i18n::Key::Retry),
                            move || retry.update(|value| *value += 1),
                        )),
                )
                .into_element(),
            })
        }))
}

/// A signed-out window has no sidebar, so this screen keeps the title bar draggable.
fn status_screen(content: View) -> View {
    view().flex_1().child(ui::title_bar()).child(
        view()
            .flex_1()
            .p_8()
            .items_center()
            .justify_center()
            .child(content),
    )
}

async fn launch(
    i18n: i18n::I18n,
    settings: Stored<Option<SettingsClient>>,
    media: Stored<Option<Arc<MediaClient>>>,
) -> ClientResult<Phase> {
    let desktop = runtime::run(async {
        if std::env::var_os("MIXIN_DESKTOP_DATA_DIR").is_none_or(|path| path.is_empty()) {
            return Err(ClientError::InvalidArgument(
                "Launch with cargo run -p mixin_desktop_ui, or set MIXIN_DESKTOP_DATA_DIR to an isolated directory.".into(),
            ));
        }
        DesktopClient::open().await
    })
    .await?;
    media.replace(Some(desktop.media()));
    settings.replace(Some(desktop.settings()));
    follow_appearance(desktop.settings());
    i18n.load(desktop.settings()).await;
    runtime::run(async move {
        match desktop.restore_account().await {
            Ok(account) => {
                let account = Arc::new(account);
                runtime::retain_account(account.clone());
                Ok(Phase::SignedIn(account))
            }
            Err(ClientError::NotFound) => {
                runtime::release_account();
                Ok(Phase::SignedOut(desktop))
            }
            Err(error) => Err(error),
        }
    })
    .await
}

fn follow_appearance(settings: SettingsClient) {
    spawn(async move {
        use futures::StreamExt as _;
        let mut updates = Box::pin(runtime::observe(
            settings.subscribe_setting("desktop_ui_appearance".into()),
        ));
        while let Some(update) = updates.next().await {
            match update {
                Ok(value) => set_color_scheme(match value.as_deref() {
                    Some("light") => nsh_core::platform::Appearance::Light,
                    Some("dark") => nsh_core::platform::Appearance::Dark,
                    _ => nsh_core::platform::Appearance::System,
                }),
                Err(error) => log::warn!("Loading appearance failed: {error}"),
            }
        }
    });
}

/// Registers app-owned native controls before the macOS engine starts.
#[cfg(target_os = "macos")]
pub fn register_native_components(components: &nsh_platform_macos::Components) {
    bot::register_native_components(components);
    shell::register_native_components(components);
}

/// Registers Windows controls on the native STA thread before starting NSH.
#[cfg(target_os = "windows")]
pub fn register_native_components(components: &nsh_platform_windows::Components) {
    bot::register_native_components(components);
    shell::register_native_components(components);
}

/// Drains platform work after the native UI loop has stopped accepting events.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn finish_native_shutdown() {
    platform::notifications::finish_shutdown();
}

/// Packaged toast activation must be received before forwarding a second launch.
#[cfg(target_os = "windows")]
pub fn prepare_notification_activation()
-> Result<Option<NotificationTarget>, Box<dyn std::error::Error>> {
    Ok(
        platform::notifications::prepare_activation()?.map(|event| match event {
            platform::notifications::Event::Clicked {
                account_id,
                conversation_id,
                message_id,
            } => NotificationTarget {
                account_id,
                conversation_id,
                message_id,
            },
        }),
    )
}

/// Registers GTK bot controls before starting the native engine.
#[cfg(target_os = "linux")]
pub fn register_native_components(components: &nsh_platform_linux::Components) {
    bot::register_native_components(components);
    shell::register_native_components(components);
}
