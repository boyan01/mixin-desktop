//! Runs Mixin on the machine's native NSH backend.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    let directory = mixin_desktop_ui::data_directory()?;
    // Bot web profiles, the notification preview and the shell namespace read
    // the resolved directory from the environment.
    // SAFETY: this entry point has not started threads or the NSH engine yet.
    unsafe {
        std::env::set_var("MIXIN_DESKTOP_DATA_DIR", &directory);
    }
    #[cfg(target_os = "windows")]
    let notification = mixin_desktop_ui::prepare_notification_activation()?;
    #[cfg(not(target_os = "windows"))]
    let notification = None;
    let _instance =
        match mixin_desktop_ui::prepare_launch(&directory, std::env::args().skip(1), notification)?
        {
            mixin_desktop_ui::Launch::Primary(instance) => instance,
            mixin_desktop_ui::Launch::Forwarded => return Ok(std::process::ExitCode::SUCCESS),
        };
    #[cfg(target_os = "macos")]
    let _quit_signal = mixin_desktop_ui::runtime::quit_on_signal(platform::quit_app)?;
    #[cfg(target_os = "linux")]
    let _quit_signal = mixin_desktop_ui::runtime::quit_on_signal(|| {
        platform::gtk4::glib::idle_add_once(|| {
            use platform::gtk4::{ApplicationWindow, Window, prelude::*};
            if let Some(window) = Window::list_toplevels()
                .into_iter()
                .find_map(|widget| widget.downcast::<ApplicationWindow>().ok())
            {
                window.close();
            }
        });
    })?;
    let tokens = nsh_base::tokens_of(Some(include_str!("../theme.json")))?;
    let root = platform::Root::of::<mixin_desktop_ui::App>()
        .app_id("one.mixin.messenger.nsh")
        .tokens(tokens);
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    platform::set_components(mixin_desktop_ui::register_native_components);
    let result = platform::run_app(
        platform::AppSource::Rust(root),
        platform::AppOptions {
            title: "Mixin Messenger".into(),
            app_id: "one.mixin.messenger.nsh".into(),
            size: (1280, 750),
            #[cfg(target_os = "macos")]
            keep_alive_on_close: true,
            ..Default::default()
        },
    );
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    mixin_desktop_ui::finish_native_shutdown();
    mixin_desktop_ui::runtime::finish_shutdown();
    #[cfg(target_os = "linux")]
    let result = result.into();
    Ok(result)
}
