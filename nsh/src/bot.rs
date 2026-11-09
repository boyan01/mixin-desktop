//! Bot-window presentation. Signing and bot identity remain account business rules.
mod window;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crate::ui::{self, Typography as _};
use mixin_desktop_api::AccountClient;
use nsh_base::alert_dialog;
use nsh_core::prelude::*;
use serde_json::{Value, json};

use crate::{
    i18n,
    links::{AppLink, LinkNavigation},
    runtime,
};

const NATIVE_WINDOWS: bool = cfg!(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux"
));

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as native;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as native;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as native;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub(crate) use native::register as register_native_components;

#[derive(Clone)]
struct Page {
    id: u64,
    key: String,
    app_id: Option<String>,
    title: String,
    immersive: bool,
    currency: String,
    source_conversation: Option<String>,
    url: String,
    focus: u64,
    icon_data: Option<String>,
    shareable: bool,
}

impl PartialEq for Page {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.focus == other.focus && self.icon_data == other.icon_data
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BotWindows {
    owner: Owner,
    session: Stored<String>,
    /// `None` only in the debug preview, which opens no windows.
    account: Stored<Option<Arc<AccountClient>>>,
    pages: Signal<Vec<Page>>,
    opening: Signal<Vec<String>>,
    next_id: Signal<u64>,
    failed: Signal<bool>,
    external: Signal<Option<String>>,
    ending: Signal<bool>,
    purge_failed: Signal<bool>,
    anchor: ElementRef,
    on_cleared: Stored<Option<std::rc::Rc<dyn Fn()>>>,
    links: LinkNavigation,
}

pub(crate) fn mount(account: Arc<AccountClient>) -> BotWindows {
    static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);
    let session = format!(
        "{}:{}",
        account.account_id(),
        NEXT_SESSION.fetch_add(1, Ordering::Relaxed)
    );
    windows(session, Some(account))
}

/// Bot windows for the debug preview, which has no account to open them for.
#[cfg(debug_assertions)]
pub(crate) fn detached() -> BotWindows {
    windows(String::new(), None)
}

fn windows(session: String, account: Option<Arc<AccountClient>>) -> BotWindows {
    BotWindows {
        session: stored(session),
        owner: Owner::current(),
        account: stored(account),
        pages: signal_with(Vec::new(), SignalEquals::Never),
        opening: signal(Vec::new()),
        next_id: signal(0),
        failed: signal(false),
        external: signal(None),
        ending: signal(false),
        purge_failed: signal(false),
        anchor: element_ref(),
        on_cleared: stored(None),
        links: use_context::<LinkNavigation>()
            .expect("Home provides link routing before bot windows"),
    }
}

impl BotWindows {
    pub(crate) fn open(self, link: AppLink) {
        if self.ending.get_untracked() {
            return;
        }
        if !NATIVE_WINDOWS {
            self.failed.set(true);
            return;
        }
        let Ok(url) = url::Url::parse(&link.url) else {
            self.failed.set(true);
            return;
        };
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            self.failed.set(true);
            return;
        }
        // A tuple preserves identity even when a URL contains the separator.
        let key = json!([link.app_id, link.source_conversation, url.as_str()]).to_string();
        if self
            .pages
            .with_untracked(|pages| pages.iter().any(|page| page.key == key))
        {
            self.pages.update(|pages| {
                if let Some(page) = pages.iter_mut().find(|page| page.key == key) {
                    page.focus += 1;
                }
            });
            return;
        }
        if self.opening.with_untracked(|keys| keys.contains(&key)) {
            return;
        }
        self.opening.update(|keys| keys.push(key.clone()));
        let account = self
            .account
            .with(|account| account.clone())
            .expect("the debug preview has no account");
        self.owner.run(|| {
            spawn(async move {
                let currency = account.profile().fiat_currency;
                let app_id = link.app_id.clone();
                let allow_generic = link.allow_generic;
                let source_conversation = link.source_conversation.clone();
                let result = runtime::run(async move {
                    match app_id {
                        Some(app_id) if allow_generic => {
                            account
                                .bot()
                                .resolve_context(app_id, source_conversation)
                                .await
                        }
                        Some(app_id) => account
                            .bot()
                            .open_context(app_id, source_conversation)
                            .await
                            .map(Some),
                        None => Ok(None),
                    }
                })
                .await;
                if self.ending.get_untracked() {
                    return;
                }
                self.opening
                    .update(|keys| keys.retain(|existing| existing != &key));
                match result {
                    Ok(context) => {
                        self.next_id.update(|next| *next += 1);
                        let id = self.next_id.get_untracked();
                        let (app_id, title, icon, immersive, currency, source_conversation) =
                            match context {
                                Some(context) => (
                                    Some(context.app_id),
                                    context.title,
                                    context.icon_url,
                                    context.immersive,
                                    context.currency,
                                    context.source_conversation,
                                ),
                                None => (
                                    None,
                                    url.host_str().unwrap_or_default().to_owned(),
                                    String::new(),
                                    false,
                                    currency,
                                    link.source_conversation,
                                ),
                            };
                        let shareable = link
                            .app_card_data
                            .as_ref()
                            .and_then(|data| data.get("shareable"))
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        self.pages.update(|pages| {
                            pages.push(Page {
                                id,
                                key,
                                app_id,
                                title,
                                immersive,
                                currency,
                                source_conversation,
                                url: url.into(),
                                focus: 0,
                                icon_data: None,
                                shareable,
                            })
                        });
                        self.load_icon(id, icon);
                    }
                    Err(_) => self.failed.set(true),
                }
            })
        });
    }

    fn load_icon(self, id: u64, url: String) {
        if !url::Url::parse(&url).is_ok_and(|url| matches!(url.scheme(), "http" | "https")) {
            return;
        }
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::run(async move {
                    let response = mixin_desktop_api::DesktopClient::open()
                        .await?
                        .http_request(
                            "GET".into(),
                            url,
                            std::collections::HashMap::new(),
                            None,
                            Some(5000),
                            Some(1024 * 1024),
                        )
                        .await?;
                    if response.status_code != 200 {
                        return Ok(None);
                    }
                    Ok(tokio::task::spawn_blocking(move || icon_png(response.body))
                        .await
                        .ok()
                        .flatten())
                })
                .await;
                if self.ending.get_untracked() {
                    return;
                }
                if let Ok(Some(icon)) = result {
                    self.pages.update(|pages| {
                        if let Some(page) = pages.iter_mut().find(|page| page.id == id) {
                            page.icon_data = Some(icon);
                        }
                    });
                }
            })
        });
    }

    pub(crate) fn ending(self) -> bool {
        self.ending.get()
    }

    fn close(self, id: u64) {
        self.pages
            .update(|pages| pages.retain(|page| page.id != id));
    }

    fn event(self, page: Page, reference: ElementRef, event: NativeEvent) {
        if self.ending.get_untracked() {
            return;
        }
        match event.name.as_ref() {
            "closed" => self.close(page.id),
            "failed" => self.failed.set(true),
            "navigate" => {
                if let Some(url) = event.data.as_str() {
                    self.links.open(url.to_owned(), page.source_conversation);
                }
            }
            "external" => self
                .external
                .set(event.data.as_str().map(ToOwned::to_owned)),
            "browser" => {
                if let Some(url) = event
                    .data
                    .as_str()
                    .and_then(|raw| url::Url::parse(raw).ok())
                    && matches!(url.scheme(), "http" | "https")
                {
                    spawn(async move {
                        if !open_url(url.to_string()).await {
                            self.failed.set(true);
                        }
                    });
                }
            }
            "share" => {
                if !page.shareable {
                    return;
                }
                if let Some(url) = event.data.get("url").and_then(Value::as_str) {
                    let title = event
                        .data
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or(&page.title);
                    nsh_core::element::desktop::main_window().focus();
                    self.links.share_web_page(
                        url.to_owned(),
                        title.to_owned(),
                        page.app_id,
                        page.shareable,
                    );
                }
            }
            "signature" => self.sign(page, reference, event.data),
            _ => {}
        }
    }

    fn sign(self, page: Page, reference: ElementRef, request: Value) {
        let Some(id) = request
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| id.len() <= 256)
        else {
            return;
        };
        let Some(page_url) = request.get("page_url").and_then(Value::as_str) else {
            return;
        };
        let Some(generation) = request.get("generation").and_then(Value::as_u64) else {
            return;
        };
        let decline = || {
            reference.command(
                "reply",
                json!({ "generation": generation, "page_url": page_url, "id": id, "args": ["[]"] }),
            )
        };
        let Some(body) = request.get("body").and_then(Value::as_array) else {
            decline();
            return;
        };
        let (Some(app_id), Some(method), Some(path)) = (
            body.first().and_then(Value::as_str),
            body.get(2).and_then(Value::as_str),
            body.get(3).and_then(Value::as_str),
        ) else {
            decline();
            return;
        };
        // Plain websites cannot enter the account signing service.
        if page.app_id.as_deref() != Some(app_id) {
            decline();
            return;
        }
        let (app_id, method, path, page_url, id) = (
            app_id.to_owned(),
            method.to_owned(),
            path.to_owned(),
            page_url.to_owned(),
            id.to_owned(),
        );
        let body_content = body.get(4).and_then(Value::as_str).map(ToOwned::to_owned);
        let reload = body
            .get(1)
            .is_some_and(|value| value == true || value == "true");
        let account = self
            .account
            .with(|account| account.clone())
            .expect("the debug preview has no account");
        spawn(async move {
            let reply_url = page_url.clone();
            let result = runtime::run(async move {
                account
                    .bot()
                    .sign_request(app_id, page_url, method, path, body_content, reload)
                    .await
            })
            .await;
            let args = match result {
                Ok(Some(signature)) => {
                    json!([signature.timestamp.to_string(), signature.signature])
                }
                _ => json!(["[]"]),
            };
            reference.command(
                "reply",
                json!({ "generation": generation, "page_url": reply_url, "id": id, "args": args }),
            );
        });
    }
}

fn icon_png(bytes: Vec<u8>) -> Option<String> {
    use base64::Engine as _;
    use std::io::Cursor;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?.thumbnail(72, 72);
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, image::ImageFormat::Png).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(output.into_inner()))
}

/// Purge native account website data before the caller replaces Home.
pub(crate) fn end_session(windows: BotWindows, on_cleared: impl Fn() + Copy + 'static) {
    if windows.ending.get_untracked() {
        return;
    }
    windows.ending.set(true);
    windows.purge_failed.set(false);
    windows
        .on_cleared
        .replace(Some(std::rc::Rc::new(on_cleared)));
    windows.pages.set(Vec::new());
    windows.opening.set(Vec::new());
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    windows.anchor.command("purge", Value::Null);
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    if let Some(callback) = windows.on_cleared.with(|callback| callback.take()) {
        callback();
    }
}

/// Keep this anchor outside the screen-lock presentation subtree.
pub(crate) fn session(windows: BotWindows) -> impl IntoElement {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        native("MixinBotSession")
            .element_ref(&windows.anchor)
            .prop(
                "session",
                windows.session.with(|key| Value::String(key.clone())),
            )
            .on_native_event(move |event: &NativeEvent| match event.name.as_ref() {
                "failed" => {
                    if windows.ending.get_untracked() {
                        windows.purge_failed.set(true);
                    } else {
                        windows.failed.set(true);
                    }
                }
                "purged" => {
                    windows.purge_failed.set(false);
                    if let Some(callback) = windows.on_cleared.with(|callback| callback.take()) {
                        callback();
                    }
                }
                "session-only" => {
                    log::info!("Bot cookies use session-only storage on this macOS version")
                }
                _ => {}
            })
            .w_0()
            .h_0()
            .into_element()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = windows;
        view().w_0().h_0().into_element()
    }
}

/// Account services have already signed out; never reveal them on purge failure.
pub(crate) fn ending_surface(windows: BotWindows) -> impl IntoElement {
    let i18n = i18n::current();
    view()
        .flex_1()
        .min_w_0()
        .p_8()
        .gap_4()
        .items_center()
        .justify_center()
        .child(
            text(move || {
                if windows.purge_failed.get() {
                    i18n.tr(
                        "Couldn’t clear browser data. Try again.",
                        "无法清除浏览器数据，请重试。",
                    )
                } else {
                    i18n.tr("Signing out…", "正在退出…")
                }
            })
            .text_role(ui::TextRole::Headline),
        )
        .child(when(windows.purge_failed, move || {
            ui::link(
                move || i18n.tr("Retry", "重试"),
                move || {
                    if windows.ending.get_untracked() && windows.purge_failed.get_untracked() {
                        windows.purge_failed.set(false);
                        windows.anchor.command("purge", Value::Null);
                    }
                },
            )
        }))
}

pub(crate) fn surface(windows: BotWindows) -> impl IntoElement {
    let i18n = i18n::current();
    effect(move || {
        if !windows.failed.get() {
            return;
        }
        nsh_core::element::desktop::main_window().focus();
        let answer = alert_dialog()
            .title(if NATIVE_WINDOWS {
                i18n.tr("Couldn’t open the page", "无法打开页面")
            } else {
                i18n.tr("Bot windows unavailable", "无法打开机器人窗口")
            })
            .description(if NATIVE_WINDOWS {
                i18n.tr(
                    "Check your connection and try again.",
                    "请检查网络连接后重试。",
                )
            } else {
                i18n.tr("Open this bot on your phone.", "请在手机端打开此机器人。")
            })
            .action("ok", i18n.tr("OK", "好"))
            .show();
        spawn(async move {
            let _ = answer.await;
            windows.failed.set(false);
        });
    });
    effect(move || {
        let Some(url) = windows.external.get() else {
            return;
        };
        nsh_core::element::desktop::main_window().focus();
        let answer = alert_dialog()
            .title(i18n.tr("Open another app?", "打开其他应用？"))
            .description(url.clone())
            .cancel("cancel", i18n.tr("Cancel", "取消"))
            .action("open", i18n.tr("Open", "打开"))
            .show();
        spawn(async move {
            let result = answer.await;
            if windows.external.get_untracked().as_deref() != Some(&url) {
                return;
            }
            // Clearing reruns this effect and cancels the task, so it comes last.
            if result.is_ok_and(|answer| answer.action.as_deref() == Some("open"))
                && !open_url(url).await
            {
                windows.failed.set(true);
            }
            windows.external.set(None);
        });
    });
    view().w_0().h_0().child(each(
        move || windows.pages.get(),
        |page| page.id,
        move |page, _index| window::host(windows, page),
    ))
}

#[cfg(debug_assertions)]
pub(crate) fn preview_ending(failed: Signal<bool>) -> impl IntoElement {
    let windows = detached();
    windows.ending.set(true);
    effect(move || windows.purge_failed.set(failed.get()));
    ending_surface(windows)
}
