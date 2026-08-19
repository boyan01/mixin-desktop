mod biometric;
mod bridge;

#[cfg(debug_assertions)]
use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::sync::Mutex as StdMutex;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

use bridge::{
    AccountCircleChangeEvent, AccountConnectionChangeEvent, AccountConversationChangeEvent,
    AccountDeviceTransferEvent, AccountHealthChangeEvent, AccountMessageChangeEvent,
    AccountNotificationEvent, AccountProfileChangeEvent, AccountUnseenCountChangeEvent,
    AppMenuActionEvent, AppProtocolUrlEvent, BootstrapDto, CommandError,
    ConversationParticipantDto, DeviceTransferEventDto, DownloadSettingsDto, LoginSessionDto,
    MessageDto, SendAttachmentRequest, SendRemoteImageRequest, StickerDetailDto, StickerDto,
};
use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, AccountProfile, CircleItem, ClientError, CodeResult, ConversationDetailItem,
    ConversationListData, ConversationStorageUsage, DesktopClient, DeviceTransferCommand,
    GroupConversationItem, LoginClient, McpServerStatusItem, McpSettingsItem,
    PinMessagePreviewItem, ProxySettingsItem, SharedAppItem, SnapshotDetailItem, StickerAlbumItem,
    StorageCategoryUsage,
};
use mixin_desktop_media::VoiceRecording;
#[cfg(debug_assertions)]
use tauri::Listener as _;
use tauri::{AppHandle, Manager as _, State, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt as _;
use tauri_specta::Event as _;
#[cfg(debug_assertions)]
use tokio::sync::{oneshot, Notify};
use tokio::sync::{Mutex, OnceCell, RwLock};
use zip::{write::SimpleFileOptions, AesMode, CompressionMethod, ZipWriter};

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct AttachmentFileInfo {
    path: String,
    name: String,
    size: u64,
}

#[cfg(debug_assertions)]
const UI_DEBUG_METADATA_FILE: &str = "mixin-typescript-ui-debug.json";

#[cfg(debug_assertions)]
#[derive(Clone)]
struct UiDebugServerState {
    app: AppHandle,
    token: Arc<str>,
    port: u16,
    next_request_id: Arc<AtomicU64>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
    hmr: Arc<UiDebugHmrState>,
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct UiDebugHmrState {
    revision: AtomicU64,
    error: StdMutex<Option<String>>,
    changed: Notify,
}

#[cfg(debug_assertions)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiDebugHmrEvent {
    kind: String,
    message: Option<String>,
}

#[cfg(debug_assertions)]
#[derive(serde::Deserialize)]
struct UiDebugHmrQuery {
    after: Option<u64>,
}

#[cfg(debug_assertions)]
#[derive(serde::Deserialize)]
struct UiDebugEvalRequest {
    expression: String,
}

#[cfg(debug_assertions)]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UiDebugMetadata<'a> {
    pid: u32,
    port: u16,
    token: &'a str,
}

#[cfg(debug_assertions)]
fn ui_debug_authorized(headers: &axum::http::HeaderMap, token: &str) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {token}"))
}

#[cfg(debug_assertions)]
fn ui_debug_eval_script(expression: &str, callback_url: &str) -> Result<String, serde_json::Error> {
    const TEMPLATE: &str = r#"void (async () => {
  const expression = __EXPRESSION__;
  const callbackUrl = __CALLBACK_URL__;
  const respond = (payload) => fetch(callbackUrl, {
    method: 'POST',
    mode: 'no-cors',
    headers: { 'content-type': 'text/plain;charset=UTF-8' },
    body: JSON.stringify(payload),
  }).catch(() => undefined);
  try {
    const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
    const value = await new AsyncFunction('return (' + expression + '\n)')();
    await respond({ ok: true, value });
  } catch (error) {
    await respond({
      ok: false,
      error: {
        name: error?.name ?? 'Error',
        message: error?.message ?? String(error),
        stack: error?.stack ?? null,
      },
    });
  }
})();"#;
    let callback_url = serde_json::to_string(callback_url)?;
    let expression = serde_json::to_string(expression)?;
    Ok(TEMPLATE
        .replace("__CALLBACK_URL__", &callback_url)
        .replace("__EXPRESSION__", &expression))
}

#[cfg(debug_assertions)]
async fn ui_debug_eval(
    axum::extract::State(state): axum::extract::State<UiDebugServerState>,
    headers: axum::http::HeaderMap,
    axum::Json(request): axum::Json<UiDebugEvalRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;

    if !ui_debug_authorized(&headers, &state.token) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": { "message": "Unauthorized" },
            })),
        )
            .into_response();
    }
    let Some(window) = state.app.get_webview_window("main") else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": { "message": "Main webview is unavailable" },
            })),
        )
            .into_response();
    };
    let request_id = state.next_request_id.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = oneshot::channel();
    state.pending.lock().await.insert(request_id, sender);
    let callback_url = format!(
        "http://127.0.0.1:{}/result/{}/{request_id}",
        state.port, state.token
    );
    let script = match ui_debug_eval_script(&request.expression, &callback_url) {
        Ok(script) => script,
        Err(error) => {
            state.pending.lock().await.remove(&request_id);
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": { "message": error.to_string() },
                })),
            )
                .into_response();
        }
    };
    if let Err(error) = window.eval(&script) {
        state.pending.lock().await.remove(&request_id);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": { "message": error.to_string() },
            })),
        )
            .into_response();
    }
    match tokio::time::timeout(std::time::Duration::from_secs(10), receiver).await {
        Ok(Ok(result)) => axum::Json(result).into_response(),
        Ok(Err(_)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": { "message": "UI debug response channel closed" },
            })),
        )
            .into_response(),
        Err(_) => {
            state.pending.lock().await.remove(&request_id);
            (
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": { "message": "UI evaluation timed out" },
                })),
            )
                .into_response()
        }
    }
}

#[cfg(debug_assertions)]
async fn ui_debug_result(
    axum::extract::State(state): axum::extract::State<UiDebugServerState>,
    axum::extract::Path((token, request_id)): axum::extract::Path<(String, u64)>,
    body: String,
) -> axum::http::StatusCode {
    if token != state.token.as_ref() {
        return axum::http::StatusCode::UNAUTHORIZED;
    }
    let Ok(result) = serde_json::from_str(&body) else {
        return axum::http::StatusCode::BAD_REQUEST;
    };
    let Some(sender) = state.pending.lock().await.remove(&request_id) else {
        return axum::http::StatusCode::NOT_FOUND;
    };
    let _ = sender.send(result);
    axum::http::StatusCode::NO_CONTENT
}

#[cfg(debug_assertions)]
async fn ui_debug_hmr(
    axum::extract::State(state): axum::extract::State<UiDebugServerState>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<UiDebugHmrQuery>,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;

    if !ui_debug_authorized(&headers, &state.token) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": { "message": "Unauthorized" },
            })),
        )
            .into_response();
    }
    loop {
        let changed = state.hmr.changed.notified();
        let revision = state.hmr.revision.load(Ordering::Acquire);
        let error = state.hmr.error.lock().expect("HMR error lock").clone();
        if let Some(message) = error {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": { "message": message },
                })),
            )
                .into_response();
        }
        if query.after.is_none_or(|after| revision > after) {
            return axum::Json(serde_json::json!({
                "ok": true,
                "revision": revision,
            }))
            .into_response();
        }
        if tokio::time::timeout(std::time::Duration::from_secs(30), changed)
            .await
            .is_err()
        {
            return (
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": { "message": "Timed out waiting for Vite HMR" },
                })),
            )
                .into_response();
        }
    }
}

#[cfg(debug_assertions)]
fn start_ui_debug_server(app: AppHandle) {
    let hmr = app.state::<AppState>().ui_debug_hmr.clone();
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await
        {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("start UI debug server failed: {error}");
                return;
            }
        };
        let port = match listener.local_addr() {
            Ok(address) => address.port(),
            Err(error) => {
                eprintln!("read UI debug server address failed: {error}");
                return;
            }
        };
        let token = Arc::<str>::from(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>()),
        );
        let state = UiDebugServerState {
            app,
            token: token.clone(),
            port,
            next_request_id: Arc::new(AtomicU64::new(1)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            hmr,
        };
        let metadata = UiDebugMetadata {
            pid: std::process::id(),
            port,
            token: &token,
        };
        let metadata_path = std::env::temp_dir().join(UI_DEBUG_METADATA_FILE);
        if let Err(error) = write_ui_debug_metadata(&metadata_path, &metadata) {
            eprintln!("write UI debug metadata failed: {error}");
            return;
        }
        let router = axum::Router::new()
            .route("/eval", axum::routing::post(ui_debug_eval))
            .route("/hmr", axum::routing::get(ui_debug_hmr))
            .route(
                "/result/{token}/{request_id}",
                axum::routing::post(ui_debug_result),
            )
            .with_state(state);
        eprintln!("UI debug server listening on 127.0.0.1:{port}");
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("UI debug server failed: {error}");
        }
    });
}

#[cfg(debug_assertions)]
fn write_ui_debug_metadata(
    path: &Path,
    metadata: &UiDebugMetadata<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt as _;
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
    };
    #[cfg(not(unix))]
    let file = fs::File::create(path)?;
    serde_json::to_writer(file, metadata)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn setup_system_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };

    let show = MenuItem::with_id(app, "tray-show", "Show", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "tray-exit", "Exit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &separator, &exit])?;
    let icon = app
        .default_window_icon()
        .expect("missing configured application icon")
        .clone();
    TrayIconBuilder::with_id("mixin-tray")
        .icon(icon)
        .tooltip("Mixin")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "tray-exit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn setup_macos_menu(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    fn item(
        app: &tauri::App,
        id: &str,
        text: &str,
        accelerator: Option<&str>,
    ) -> tauri::Result<MenuItem<tauri::Wry>> {
        MenuItem::with_id(app, id, text, true, accelerator)
    }

    let about = PredefinedMenuItem::about(app, Some("About Mixin"), None)?;
    let preferences = item(app, "menu-preferences", "Preferences…", Some("CmdOrCtrl+,"))?;
    let lock = item(app, "menu-lock", "Lock", Some("CmdOrCtrl+Shift+L"))?;
    let quick_search = item(
        app,
        "menu-quick-search",
        "Quick Search",
        Some("CmdOrCtrl+K"),
    )?;
    let hide = PredefinedMenuItem::hide(app, Some("Hide Mixin"))?;
    let show = item(app, "menu-show", "Show Mixin", None)?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit Mixin"))?;
    let app_menu = Submenu::with_items(
        app,
        "Mixin",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &preferences,
            &PredefinedMenuItem::separator(app)?,
            &lock,
            &PredefinedMenuItem::separator(app)?,
            &quick_search,
            &hide,
            &show,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let create_conversation = item(
        app,
        "menu-create-conversation",
        "Create Conversation",
        Some("CmdOrCtrl+N"),
    )?;
    let create_group = item(
        app,
        "menu-create-group",
        "Create Group",
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let create_circle = item(app, "menu-create-circle", "Create Circle", None)?;
    let close = PredefinedMenuItem::close_window(app, Some("Close Window"))?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &create_conversation,
            &create_group,
            &create_circle,
            &PredefinedMenuItem::separator(app)?,
            &close,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let mute = item(app, "menu-conversation-mute", "Mute", None)?;
    let search = item(app, "menu-conversation-search", "Search", None)?;
    let delete = item(app, "menu-conversation-delete", "Delete Chat", None)?;
    let pin = item(app, "menu-conversation-pin", "Pin", None)?;
    let info = item(app, "menu-conversation-info", "Toggle Chat Info", None)?;
    let conversation_menu = Submenu::with_items(
        app,
        "Conversation",
        true,
        &[&mute, &search, &delete, &pin, &info],
    )?;

    let minimize = PredefinedMenuItem::minimize(app, Some("Minimize"))?;
    let zoom = PredefinedMenuItem::maximize(app, Some("Zoom"))?;
    let previous = item(
        app,
        "menu-previous-conversation",
        "Previous Conversation",
        Some("CmdOrCtrl+Up"),
    )?;
    let next = item(
        app,
        "menu-next-conversation",
        "Next Conversation",
        Some("CmdOrCtrl+Down"),
    )?;
    let window_show = item(app, "menu-window-show", "Mixin", Some("CmdOrCtrl+O"))?;
    let bring_all_to_front = PredefinedMenuItem::show_all(app, Some("Bring All to Front"))?;
    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &minimize,
            &zoom,
            &PredefinedMenuItem::separator(app)?,
            &previous,
            &next,
            &PredefinedMenuItem::separator(app)?,
            &window_show,
            &bring_all_to_front,
        ],
    )?;

    let help_center = item(app, "menu-help-center", "Help Center", None)?;
    let terms = item(app, "menu-terms", "Terms of Service", None)?;
    let privacy = item(app, "menu-privacy", "Privacy Policy", None)?;
    let help_menu = Submenu::with_items(app, "Help", true, &[&help_center, &terms, &privacy])?;

    #[cfg(debug_assertions)]
    let developer_menu = {
        let toggle_devtools = item(
            app,
            "menu-toggle-devtools",
            "Toggle Developer Tools",
            Some("CmdOrCtrl+Alt+I"),
        )?;
        Submenu::with_items(app, "Developer", true, &[&toggle_devtools])?
    };

    #[cfg(debug_assertions)]
    let menu = Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &conversation_menu,
            &developer_menu,
            &window_menu,
            &help_menu,
        ],
    )?;
    #[cfg(not(debug_assertions))]
    let menu = Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &conversation_menu,
            &window_menu,
            &help_menu,
        ],
    )?;
    app.set_menu(menu)?;
    *app.state::<AppState>().macos_menu.blocking_lock() = Some(MacosMenuItems {
        app_menu,
        file_menu,
        edit_menu,
        conversation_menu,
        window_menu,
        help_menu,
        about,
        preferences,
        lock,
        quick_search,
        hide,
        show,
        quit,
        create_conversation,
        create_group,
        create_circle,
        close,
        mute,
        search,
        delete,
        pin,
        info,
        minimize,
        zoom,
        previous,
        next,
        bring_all_to_front,
        help_center,
        terms,
        privacy,
    });
    app.on_menu_event(|app, event| match event.id().as_ref() {
        "menu-show" | "menu-window-show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        #[cfg(debug_assertions)]
        "menu-toggle-devtools" => {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_devtools_open() {
                    window.close_devtools();
                } else {
                    window.open_devtools();
                }
            }
        }
        id if id.starts_with("menu-") => {
            let _ = AppMenuActionEvent(id.to_owned()).emit(app);
        }
        _ => {}
    });
    Ok(())
}

#[cfg(target_os = "macos")]
struct MacosMenuItems {
    app_menu: tauri::menu::Submenu<tauri::Wry>,
    file_menu: tauri::menu::Submenu<tauri::Wry>,
    edit_menu: tauri::menu::Submenu<tauri::Wry>,
    conversation_menu: tauri::menu::Submenu<tauri::Wry>,
    window_menu: tauri::menu::Submenu<tauri::Wry>,
    help_menu: tauri::menu::Submenu<tauri::Wry>,
    about: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    preferences: tauri::menu::MenuItem<tauri::Wry>,
    lock: tauri::menu::MenuItem<tauri::Wry>,
    quick_search: tauri::menu::MenuItem<tauri::Wry>,
    hide: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    show: tauri::menu::MenuItem<tauri::Wry>,
    quit: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    create_conversation: tauri::menu::MenuItem<tauri::Wry>,
    create_group: tauri::menu::MenuItem<tauri::Wry>,
    create_circle: tauri::menu::MenuItem<tauri::Wry>,
    close: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    mute: tauri::menu::MenuItem<tauri::Wry>,
    search: tauri::menu::MenuItem<tauri::Wry>,
    delete: tauri::menu::MenuItem<tauri::Wry>,
    pin: tauri::menu::MenuItem<tauri::Wry>,
    info: tauri::menu::MenuItem<tauri::Wry>,
    minimize: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    zoom: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    previous: tauri::menu::MenuItem<tauri::Wry>,
    next: tauri::menu::MenuItem<tauri::Wry>,
    bring_all_to_front: tauri::menu::PredefinedMenuItem<tauri::Wry>,
    help_center: tauri::menu::MenuItem<tauri::Wry>,
    terms: tauri::menu::MenuItem<tauri::Wry>,
    privacy: tauri::menu::MenuItem<tauri::Wry>,
}

#[cfg(target_os = "macos")]
fn dock_badge_count(count: i64) -> Option<i64> {
    (count > 0).then_some(count)
}

struct ActiveLogin {
    request_id: u64,
    client: Arc<LoginClient>,
}

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct MacosMenuLabels {
    about: String,
    preferences: String,
    lock: String,
    quick_search: String,
    hide_mixin: String,
    show_mixin: String,
    quit_mixin: String,
    file: String,
    edit: String,
    create_conversation: String,
    create_group: String,
    create_circle: String,
    close_window: String,
    conversation: String,
    mute: String,
    unmute: String,
    search: String,
    delete_chat: String,
    pin_title: String,
    unpin: String,
    toggle_chat_info: String,
    window: String,
    minimize: String,
    zoom: String,
    previous_conversation: String,
    next_conversation: String,
    bring_all_to_front: String,
    help: String,
    help_center: String,
    terms_of_service: String,
    privacy_policy: String,
}

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct MacosMenuState {
    labels: MacosMenuLabels,
    has_conversation: bool,
    is_muted: bool,
    is_pinned: bool,
    has_passcode: bool,
}

struct AppState {
    desktop: OnceCell<DesktopClient>,
    account: RwLock<Option<Arc<AccountClient>>>,
    login: Mutex<Option<ActiveLogin>>,
    next_login_id: AtomicU64,
    next_bot_webview_id: AtomicU64,
    events_started: AtomicBool,
    pending_protocol_urls: Mutex<Vec<String>>,
    #[cfg(debug_assertions)]
    ui_debug_hmr: Arc<UiDebugHmrState>,
    #[cfg(target_os = "macos")]
    macos_menu: Mutex<Option<MacosMenuItems>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            desktop: OnceCell::new(),
            account: RwLock::new(None),
            login: Mutex::new(None),
            next_login_id: AtomicU64::new(1),
            next_bot_webview_id: AtomicU64::new(1),
            events_started: AtomicBool::new(false),
            pending_protocol_urls: Mutex::new(Vec::new()),
            #[cfg(debug_assertions)]
            ui_debug_hmr: Arc::new(UiDebugHmrState::default()),
            #[cfg(target_os = "macos")]
            macos_menu: Mutex::new(None),
        }
    }
}

#[cfg(target_os = "macos")]
fn menu_error(error: impl std::fmt::Display) -> CommandError {
    CommandError {
        kind: "internal",
        message: error.to_string(),
    }
}

#[tauri::command]
#[specta::specta]
async fn update_macos_menu(
    state: State<'_, AppState>,
    menu_state: MacosMenuState,
) -> Result<(), CommandError> {
    #[cfg(target_os = "macos")]
    {
        let items = state.macos_menu.lock().await;
        let Some(items) = items.as_ref() else {
            return Ok(());
        };
        let labels = menu_state.labels;
        macro_rules! set_text {
            ($item:ident, $text:expr) => {
                items.$item.set_text($text).map_err(menu_error)?
            };
        }
        set_text!(app_menu, "Mixin");
        set_text!(about, format!("{} Mixin", labels.about));
        set_text!(preferences, labels.preferences);
        set_text!(lock, labels.lock);
        items
            .lock
            .set_enabled(menu_state.has_passcode)
            .map_err(menu_error)?;
        set_text!(quick_search, labels.quick_search);
        set_text!(hide, labels.hide_mixin);
        set_text!(show, labels.show_mixin);
        set_text!(quit, labels.quit_mixin);

        set_text!(file_menu, labels.file);
        set_text!(edit_menu, labels.edit);
        set_text!(create_conversation, labels.create_conversation);
        set_text!(create_group, labels.create_group);
        set_text!(create_circle, labels.create_circle);
        set_text!(close, labels.close_window);

        set_text!(conversation_menu, labels.conversation);
        set_text!(
            mute,
            if menu_state.is_muted {
                labels.unmute
            } else {
                labels.mute
            }
        );
        set_text!(search, labels.search);
        set_text!(delete, labels.delete_chat);
        set_text!(
            pin,
            if menu_state.is_pinned {
                labels.unpin
            } else {
                labels.pin_title
            }
        );
        set_text!(info, labels.toggle_chat_info);
        for item in [
            &items.mute,
            &items.search,
            &items.delete,
            &items.pin,
            &items.info,
        ] {
            item.set_enabled(menu_state.has_conversation)
                .map_err(menu_error)?;
        }

        set_text!(window_menu, labels.window);
        set_text!(minimize, labels.minimize);
        set_text!(zoom, labels.zoom);
        set_text!(previous, labels.previous_conversation);
        set_text!(next, labels.next_conversation);
        items
            .previous
            .set_enabled(menu_state.has_conversation)
            .map_err(menu_error)?;
        items
            .next
            .set_enabled(menu_state.has_conversation)
            .map_err(menu_error)?;
        set_text!(bring_all_to_front, labels.bring_all_to_front);

        set_text!(help_menu, labels.help);
        set_text!(help_center, labels.help_center);
        set_text!(terms, labels.terms_of_service);
        set_text!(privacy, labels.privacy_policy);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (state, menu_state);
    }
    Ok(())
}

fn protocol_urls(arguments: &[String]) -> Vec<String> {
    arguments
        .iter()
        .filter(|argument| {
            argument.starts_with("mixin:")
                || argument.starts_with("https://mixin.one/")
                || argument.starts_with("https://www.mixin.one/")
        })
        .cloned()
        .collect()
}

fn enqueue_protocol_urls(app: &AppHandle, urls: Vec<String>) {
    if urls.is_empty() {
        return;
    }
    let state = app.state::<AppState>();
    tauri::async_runtime::block_on(async {
        state
            .pending_protocol_urls
            .lock()
            .await
            .extend(urls.clone());
    });
    for url in urls {
        let _ = AppProtocolUrlEvent(url).emit(app);
    }
}

#[tauri::command]
#[specta::specta]
async fn take_pending_protocol_urls(
    state: State<'_, AppState>,
) -> Result<Vec<String>, CommandError> {
    Ok(std::mem::take(
        &mut *state.pending_protocol_urls.lock().await,
    ))
}

async fn account_bootstrap(account: &AccountClient) -> Result<BootstrapDto, CommandError> {
    let conversations = account.conversation().conversation_items().await?;
    let health = account.account_health();
    futures::pin_mut!(health);
    let account_health = health.next().await;
    let connection = account.connection_status();
    futures::pin_mut!(connection);
    let connection_status = connection.next().await;
    Ok(BootstrapDto {
        signed_in: true,
        profile: Some(account.profile()),
        conversations,
        account_health,
        connection_status,
    })
}

impl AppState {
    async fn desktop(&self) -> Result<&DesktopClient, CommandError> {
        self.desktop
            .get_or_try_init(DesktopClient::open)
            .await
            .map_err(Into::into)
    }

    async fn account(&self) -> Result<Arc<AccountClient>, CommandError> {
        self.account
            .read()
            .await
            .clone()
            .ok_or_else(|| CommandError::from(ClientError::Unauthorized))
    }
}

#[tauri::command]
#[specta::specta]
fn biometric_available() -> bool {
    biometric::is_available()
}

#[tauri::command]
#[specta::specta]
async fn authenticate_biometric() -> bool {
    tokio::task::spawn_blocking(biometric::authenticate)
        .await
        .unwrap_or(false)
}

#[tauri::command]
#[specta::specta]
async fn bootstrap(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BootstrapDto, CommandError> {
    let cached_account = { state.account.read().await.clone() };
    let account = if let Some(account) = cached_account {
        account
    } else {
        let desktop = state.desktop().await?;
        match desktop.restore_account().await {
            Ok(account) => {
                let account = Arc::new(account);
                *state.account.write().await = Some(account.clone());
                account
            }
            Err(ClientError::NotFound | ClientError::Unauthorized) => {
                return Ok(BootstrapDto {
                    signed_in: false,
                    profile: None,
                    conversations: Vec::new(),
                    account_health: None,
                    connection_status: None,
                });
            }
            Err(error) => return Err(error.into()),
        }
    };

    start_account_events(&app, &state, account.clone());
    account_bootstrap(&account).await
}

#[tauri::command]
#[specta::specta]
async fn begin_login(state: State<'_, AppState>) -> Result<LoginSessionDto, CommandError> {
    let client = Arc::new(state.desktop().await?.begin_login().await?);
    let request_id = state.next_login_id.fetch_add(1, Ordering::Relaxed);
    let auth_url = client.auth_url();
    let previous = state
        .login
        .lock()
        .await
        .replace(ActiveLogin { request_id, client });
    if let Some(previous) = previous {
        previous.client.cancel();
    }
    Ok(LoginSessionDto {
        request_id,
        auth_url,
    })
}

#[tauri::command]
#[specta::specta]
async fn wait_login(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: u64,
) -> Result<BootstrapDto, CommandError> {
    let client = {
        let active = state.login.lock().await;
        active
            .as_ref()
            .filter(|login| login.request_id == request_id)
            .map(|login| login.client.clone())
            .ok_or(ClientError::Cancelled)?
    };
    let account = Arc::new(client.wait().await?);
    let current = {
        let mut active = state.login.lock().await;
        let current = active
            .as_ref()
            .is_some_and(|login| login.request_id == request_id);
        if current {
            active.take();
        }
        current
    };
    if !current {
        account.shutdown().await;
        return Err(ClientError::Cancelled.into());
    }

    *state.account.write().await = Some(account.clone());
    start_account_events(&app, &state, account.clone());
    account_bootstrap(&account).await
}

#[tauri::command]
#[specta::specta]
async fn cancel_login(state: State<'_, AppState>, request_id: u64) -> Result<(), CommandError> {
    let login = {
        let mut active = state.login.lock().await;
        if active
            .as_ref()
            .is_some_and(|login| login.request_id == request_id)
        {
            active.take()
        } else {
            None
        }
    };
    if let Some(login) = login {
        login.client.cancel();
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn conversations_by_ids(
    state: State<'_, AppState>,
    conversation_ids: Vec<String>,
) -> Result<Vec<ConversationListData>, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .conversation_items_by_ids(conversation_ids)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn all_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationListData>, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .conversation_items()
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn groups_in_common(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<GroupConversationItem>, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .groups_in_common(user_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn circles(state: State<'_, AppState>) -> Result<Vec<CircleItem>, CommandError> {
    Ok(state.account().await?.conversation().circles().await?)
}

#[tauri::command]
#[specta::specta]
async fn search_user(
    state: State<'_, AppState>,
    query: String,
) -> Result<mixin_desktop_api::UserProfileItem, CommandError> {
    let query = query.trim().to_string();
    if query.len() < 4 {
        return Err(ClientError::InvalidArgument("search query is too short".into()).into());
    }
    Ok(state.account().await?.user().search_user(query).await?)
}

#[tauri::command]
#[specta::specta]
async fn search_local_users(
    state: State<'_, AppState>,
    query: String,
    category: String,
    limit: u32,
) -> Result<Vec<mixin_desktop_api::UserProfileItem>, CommandError> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    Ok(state
        .account()
        .await?
        .user()
        .search_local_users(query, category, limit.clamp(1, 64) as i64)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn search_mao_user(
    state: State<'_, AppState>,
    query: String,
) -> Result<Option<mixin_desktop_api::UserProfileItem>, CommandError> {
    Ok(state.account().await?.user().search_mao_user(query).await?)
}

#[tauri::command]
#[specta::specta]
async fn search_global_messages(
    state: State<'_, AppState>,
    query: String,
    anchor_message_id: Option<String>,
    limit: u32,
) -> Result<Vec<MessageDto>, CommandError> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    Ok(state
        .account()
        .await?
        .message()
        .search_global_messages(query, anchor_message_id, limit.clamp(1, 100))
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn user_profile(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Option<mixin_desktop_api::UserProfileItem>, CommandError> {
    let user = state.account().await?.user();
    let profile = match user.user_profile(Some(user_id.clone()), None).await? {
        Some(profile) => Some(profile),
        None => user.refresh_user_profile(user_id).await?,
    };
    Ok(profile)
}

#[tauri::command]
#[specta::specta]
async fn refresh_user_profile(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Option<mixin_desktop_api::UserProfileItem>, CommandError> {
    Ok(state
        .account()
        .await?
        .user()
        .refresh_user_profile(user_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn selectable_users(
    state: State<'_, AppState>,
) -> Result<Vec<mixin_desktop_api::UserProfileItem>, CommandError> {
    Ok(state.account().await?.user().selectable_users().await?)
}

#[tauri::command]
#[specta::specta]
async fn open_user_conversation(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .open_user_conversation(user_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn resolve_code(
    state: State<'_, AppState>,
    code: String,
) -> Result<CodeResult, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .resolve_code(code)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn join_group(state: State<'_, AppState>, code: String) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .join_group(code)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn snapshot_by_trace(
    state: State<'_, AppState>,
    trace_id: String,
) -> Result<SnapshotDetailItem, CommandError> {
    Ok(state.account().await?.snapshot_by_trace(trace_id).await?)
}

#[tauri::command]
#[specta::specta]
async fn snapshot_by_id(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<SnapshotDetailItem, CommandError> {
    Ok(state.account().await?.snapshot_by_id(snapshot_id).await?)
}

#[tauri::command]
#[specta::specta]
async fn safe_snapshot_by_id(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<SnapshotDetailItem, CommandError> {
    Ok(state
        .account()
        .await?
        .safe_snapshot_by_id(snapshot_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn create_circle(
    state: State<'_, AppState>,
    name: String,
) -> Result<CircleItem, CommandError> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(ClientError::InvalidArgument("invalid circle name".into()).into());
    }
    Ok(state
        .account()
        .await?
        .conversation()
        .create_circle(name)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn update_circle(
    state: State<'_, AppState>,
    circle_id: String,
    name: String,
) -> Result<(), CommandError> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(ClientError::InvalidArgument("invalid circle name".into()).into());
    }
    Ok(state
        .account()
        .await?
        .conversation()
        .update_circle(circle_id, name)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn delete_circle(state: State<'_, AppState>, circle_id: String) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .delete_circle(circle_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn reorder_circles(
    state: State<'_, AppState>,
    circle_ids: Vec<String>,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .reorder_circles(circle_ids)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn create_group(
    state: State<'_, AppState>,
    name: String,
    user_ids: Vec<String>,
) -> Result<String, CommandError> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 || user_ids.is_empty() {
        return Err(ClientError::InvalidArgument("invalid group input".into()).into());
    }
    Ok(state
        .account()
        .await?
        .conversation()
        .create_group(name, user_ids)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn edit_circle_conversation(
    state: State<'_, AppState>,
    circle_id: String,
    conversation_id: String,
    owner_id: String,
    is_group: bool,
    add: bool,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .edit_circle_conversation(circle_id, conversation_id, owner_id, is_group, add)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn edit_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    name: Option<String>,
    announcement: Option<String>,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .edit_conversation(conversation_id, name, announcement)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn exit_group(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .exit_group(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn clear_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .clear_conversation(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn add_contact(
    state: State<'_, AppState>,
    user_id: String,
    full_name: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .user()
        .add_contact(user_id, full_name)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn remove_contact(state: State<'_, AppState>, user_id: String) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .user()
        .remove_contact(user_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn block_user(state: State<'_, AppState>, user_id: String) -> Result<(), CommandError> {
    Ok(state.account().await?.user().block_user(user_id).await?)
}

#[tauri::command]
#[specta::specta]
async fn unblock_user(state: State<'_, AppState>, user_id: String) -> Result<(), CommandError> {
    Ok(state.account().await?.user().unblock_user(user_id).await?)
}

#[tauri::command]
#[specta::specta]
async fn report_user(state: State<'_, AppState>, user_id: String) -> Result<(), CommandError> {
    Ok(state.account().await?.user().report_user(user_id).await?)
}

#[tauri::command]
#[specta::specta]
async fn bot_home_uri(
    state: State<'_, AppState>,
    app_id: String,
) -> Result<Option<String>, CommandError> {
    Ok(state.account().await?.user().bot_home_uri(app_id).await?)
}

fn bot_context_script(context: &str, icon_color: &str) -> String {
    format!(
        r#"(() => {{
  if (window.top !== window) return;
  window.MixinContext = {{ getContext: () => {context} }};
  const install = () => {{
    if (document.getElementById('mixin-webview-navigation')) return;
    const bar = document.createElement('nav');
    bar.id = 'mixin-webview-navigation';
    bar.setAttribute('aria-label', 'Web view navigation');
    bar.innerHTML = `<button type="button" aria-label="Back" data-action="back"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10.5 2.5 5 8l5.5 5.5"/></svg></button><button type="button" aria-label="Forward" data-action="forward"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m5.5 2.5L11 8l-5.5 5.5"/></svg></button><button type="button" aria-label="Reload" data-action="reload"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M13 7.5A5 5 0 1 1 11.4 4M11 1.75v3.5h3.5"/></svg></button>`;
    const style = document.createElement('style');
    style.textContent = '#mixin-webview-navigation{{position:fixed;z-index:2147483647;top:0;left:0;height:22px;padding-left:10px;display:flex;align-items:center;gap:16px;color:{icon_color};pointer-events:auto}}#mixin-webview-navigation button{{width:16px;height:16px;margin:0;padding:0;border:0;background:transparent;color:inherit;display:grid;place-items:center;cursor:pointer}}#mixin-webview-navigation button:disabled{{opacity:.5;cursor:default}}#mixin-webview-navigation svg{{width:16px;height:16px;fill:none;stroke:currentColor;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}}';
    document.documentElement.append(style, bar);
    const back = bar.querySelector('[data-action="back"]');
    const forward = bar.querySelector('[data-action="forward"]');
    const refresh = bar.querySelector('[data-action="reload"]');
    const update = () => {{
      back.disabled = history.length <= 1;
      // Tauri's Webview API does not expose forward-history state for remote pages.
      forward.disabled = true;
    }};
    back.addEventListener('click', () => {{ if (!back.disabled) history.back(); }});
    forward.addEventListener('click', () => {{ if (!forward.disabled) history.forward(); }});
    refresh.addEventListener('click', () => location.reload());
    addEventListener('popstate', update);
    update();
  }};
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', install, {{ once: true }});
  else install();
}})();"#,
        context = context,
        icon_color = icon_color,
    )
}

#[tauri::command]
#[specta::specta]
async fn open_bot_webview(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    title: String,
    conversation_id: String,
    locale: String,
    appearance: String,
) -> Result<(), CommandError> {
    let url = Url::parse(&url).map_err(|error| CommandError {
        kind: "invalidArgument",
        message: error.to_string(),
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(CommandError {
            kind: "invalidArgument",
            message: "Bot home URL must use HTTP or HTTPS".to_string(),
        });
    }
    let account = state.account().await?;
    let context = serde_json::json!({
        "app_version": app.package_info().version.to_string(),
        "immersive": false,
        "appearance": if appearance == "light" { "light" } else { "dark" },
        "platform": "Desktop",
        "locale": locale,
        "conversation_id": conversation_id,
        "currency": account.profile().fiat_currency,
    });
    let context = serde_json::to_string(&context).map_err(|error| CommandError {
        kind: "internal",
        message: error.to_string(),
    })?;
    let label = format!(
        "bot-{}",
        state.next_bot_webview_id.fetch_add(1, Ordering::Relaxed),
    );
    let icon_color = if appearance == "light" {
        "#2f3032"
    } else {
        "rgba(255,255,255,.9)"
    };
    let builder = WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url))
        .title(title)
        .inner_size(380.0, 750.0)
        .user_agent(&format!(" Mixin/{}", app.package_info().version))
        .initialization_script(bot_context_script(&context, icon_color));
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay);
    #[cfg(not(target_os = "macos"))]
    let builder = builder;
    builder.build().map_err(|error| CommandError {
        kind: "internal",
        message: error.to_string(),
    })?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn send_post(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .send_post(conversation_id, content)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn send_app_card(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .send_app_card(conversation_id, content)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn messages(
    state: State<'_, AppState>,
    conversation_id: String,
    before_created_at_micros: Option<i64>,
    before_message_id: Option<String>,
    limit: i64,
) -> Result<Vec<MessageDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .messages(
            conversation_id,
            before_created_at_micros,
            before_message_id,
            limit.clamp(1, 200),
        )
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn messages_around(
    state: State<'_, AppState>,
    conversation_id: String,
    target_message_id: String,
    before: i64,
    after: i64,
) -> Result<Vec<MessageDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .messages_around(
            conversation_id,
            target_message_id,
            before.clamp(0, 200),
            after.clamp(0, 200),
        )
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn send_text(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
    quote_message_id: Option<String>,
    silent: bool,
) -> Result<String, CommandError> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(ClientError::InvalidArgument("message is empty".to_string()).into());
    }
    Ok(state
        .account()
        .await?
        .message()
        .send_text(conversation_id, content, quote_message_id, silent)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn conversation_is_encrypted(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<bool, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .conversation_is_encrypted(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn mark_conversation_read(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .mark_conversation_read(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn update_draft(
    state: State<'_, AppState>,
    conversation_id: String,
    draft: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .update_draft(conversation_id, draft)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn set_conversation_pinned(
    state: State<'_, AppState>,
    conversation_id: String,
    pinned: bool,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .set_pinned(conversation_id, pinned)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn set_conversation_muted(
    state: State<'_, AppState>,
    conversation_id: String,
    owner_id: String,
    category: String,
    duration_seconds: i64,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .set_muted(conversation_id, owner_id, category, duration_seconds)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn delete_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .delete_conversation(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn set_message_pinned(
    state: State<'_, AppState>,
    conversation_id: String,
    message_id: String,
    pinned: bool,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .set_message_pinned(conversation_id, message_id, pinned)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn pinned_message_ids(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<String>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .pinned_message_ids(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn pin_message_preview(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Option<PinMessagePreviewItem>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .pin_message_preview(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn unread_mention_message_ids(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<String>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .unread_mention_message_ids(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn mark_mention_read(
    state: State<'_, AppState>,
    conversation_id: String,
    message_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .mark_mention_read(conversation_id, message_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn mark_audio_read(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .attachment()
        .mark_audio_read(message_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn delete_messages(
    state: State<'_, AppState>,
    conversation_id: String,
    message_ids: Vec<String>,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .delete_messages(conversation_id, message_ids)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn recall_messages(
    state: State<'_, AppState>,
    conversation_id: String,
    message_ids: Vec<String>,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .recall_messages(conversation_id, message_ids)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn download_attachment(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .attachment()
        .download_attachment(message_id)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn save_attachment_as(
    state: State<'_, AppState>,
    message_id: String,
    destination: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .attachment()
        .save_attachment_as(message_id, destination)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn cancel_attachment(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .attachment()
        .cancel_attachment(message_id)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn retry_attachment(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .attachment()
        .retry_attachment(message_id)
        .await?;
    Ok(())
}

/// Persists the browser canvas result as a local PNG so the existing attachment
/// pipeline remains the only writer/sender of media messages.
#[tauri::command]
#[specta::specta]
fn write_edited_image(data_url: String) -> Result<String, CommandError> {
    const PREFIX: &str = "data:image/png;base64,";
    const MAX_BYTES: usize = 32 * 1024 * 1024;
    let encoded = data_url
        .strip_prefix(PREFIX)
        .ok_or_else(|| ClientError::InvalidArgument("expected PNG data URL".into()))?;
    let bytes = BASE64
        .decode(encoded)
        .map_err(|_| ClientError::InvalidArgument("invalid PNG data".into()))?;
    if bytes.len() > MAX_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(ClientError::InvalidArgument("invalid PNG data".into()).into());
    }
    let sequence = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ClientError::Internal("system clock before UNIX epoch".into()))?
        .as_micros();
    let path = std::env::temp_dir().join(format!("mixin_edited_{sequence}.png"));
    fs::write(&path, bytes).map_err(|error| ClientError::Internal(error.to_string()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Reads only metadata for files selected by the user. Keeping this native
/// avoids granting the webview arbitrary filesystem access just to show sizes.
#[tauri::command]
#[specta::specta]
fn attachment_file_info(paths: Vec<String>) -> Result<Vec<AttachmentFileInfo>, CommandError> {
    paths
        .into_iter()
        .map(|path| {
            let source = PathBuf::from(&path);
            let metadata = fs::metadata(&source)
                .map_err(|error| ClientError::InvalidArgument(error.to_string()))?;
            if !metadata.is_file() {
                return Err(ClientError::InvalidArgument("attachment is not a file".into()).into());
            }
            let name = source
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .ok_or_else(|| ClientError::InvalidArgument("attachment has no file name".into()))?
                .to_owned();
            Ok(AttachmentFileInfo {
                path,
                name,
                size: metadata.len(),
            })
        })
        .collect()
}

/// Creates the same temporary ZIP attachment used by Flutter's preview dialog.
/// Archive names are always local basenames so a selected path cannot create
/// traversal entries inside the archive.
#[tauri::command]
#[specta::specta]
fn create_attachment_archive(
    paths: Vec<String>,
    password: Option<String>,
) -> Result<String, CommandError> {
    if paths.is_empty() {
        return Err(
            ClientError::InvalidArgument("archive requires at least one file".into()).into(),
        );
    }
    let password = password.unwrap_or_default();
    if password.chars().count() > 128 {
        return Err(ClientError::InvalidArgument("archive password is too long".into()).into());
    }
    let sequence = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ClientError::Internal("system clock before UNIX epoch".into()))?
        .as_micros();
    let target = std::env::temp_dir().join(format!("mixin_archive_{sequence}.zip"));
    let output =
        fs::File::create(&target).map_err(|error| ClientError::Internal(error.to_string()))?;
    let mut archive = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let options = if password.is_empty() {
        options
    } else {
        options.with_aes_encryption(AesMode::Aes256, &password)
    };
    let result = (|| -> Result<(), CommandError> {
        let mut names = std::collections::HashSet::new();
        for path in paths {
            let source = Path::new(&path);
            let name = source
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    ClientError::InvalidArgument("attachment has no file name".into())
                })?;
            if !names.insert(name.to_owned()) {
                return Err(ClientError::InvalidArgument(
                    "archive contains duplicate file names".into(),
                )
                .into());
            }
            let mut input = fs::File::open(source)
                .map_err(|error| ClientError::InvalidArgument(error.to_string()))?;
            archive
                .start_file(name, options)
                .map_err(|error| ClientError::Internal(error.to_string()))?;
            std::io::copy(&mut input, &mut archive)
                .map_err(|error| ClientError::Internal(error.to_string()))?;
        }
        archive
            .finish()
            .map_err(|error| ClientError::Internal(error.to_string()))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&target);
        return Err(error);
    }
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
#[specta::specta]
async fn send_attachment(
    state: State<'_, AppState>,
    request: SendAttachmentRequest,
) -> Result<String, CommandError> {
    if request.path.trim().is_empty() || request.mime_type.trim().is_empty() {
        return Err(ClientError::InvalidArgument("invalid attachment".into()).into());
    }
    Ok(state
        .account()
        .await?
        .message()
        .send_attachment(
            request.conversation_id,
            request.path,
            request.kind,
            request.mime_type,
            request.name,
            request.width,
            request.height,
            request.duration_millis,
            request.thumbnail,
            request.caption,
            request.quote_message_id,
            request.silent,
        )
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn send_remote_image(
    state: State<'_, AppState>,
    request: SendRemoteImageRequest,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .send_remote_image(
            request.conversation_id,
            request.url,
            request.preview_url,
            request.width,
            request.height,
            request.mime_type,
            request.silent,
        )
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn search_messages(
    state: State<'_, AppState>,
    conversation_id: String,
    query: String,
    sender_id: Option<String>,
    categories: Option<Vec<String>>,
    anchor_message_id: Option<String>,
    limit: u32,
) -> Result<Vec<MessageDto>, CommandError> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    Ok(state
        .account()
        .await?
        .message()
        .search_messages(
            conversation_id,
            query,
            sender_id,
            categories.unwrap_or_default(),
            anchor_message_id,
            limit.clamp(1, 100),
        )
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn update_profile(
    state: State<'_, AppState>,
    full_name: String,
    biography: String,
) -> Result<AccountProfile, CommandError> {
    let full_name = full_name.trim().to_string();
    if full_name.is_empty() {
        return Err(ClientError::InvalidArgument("full name is empty".into()).into());
    }
    Ok(state
        .account()
        .await?
        .update_profile(full_name, biography)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn refresh_profile(state: State<'_, AppState>) -> Result<AccountProfile, CommandError> {
    Ok(state.account().await?.refresh_profile().await?)
}

#[tauri::command]
#[specta::specta]
async fn refresh_account_health(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.account().await?.refresh_account_health().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn retry_connection(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.account().await?.retry_connection();
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn device_transfer_command(
    state: State<'_, AppState>,
    command: String,
) -> Result<(), CommandError> {
    let command = match command.as_str() {
        "pullToRemote" => DeviceTransferCommand::PullToRemote,
        "pushToRemote" => DeviceTransferCommand::PushToRemote,
        "cancelRestore" => DeviceTransferCommand::CancelRestore,
        "cancelBackup" => DeviceTransferCommand::CancelBackup,
        "cancelBackupRequest" => DeviceTransferCommand::CancelBackupRequest,
        "cancelRestoreRequest" => DeviceTransferCommand::CancelRestoreRequest,
        "confirmRestore" => DeviceTransferCommand::ConfirmRestore,
        "confirmBackup" => DeviceTransferCommand::ConfirmBackup,
        _ => {
            return Err(
                ClientError::InvalidArgument("unknown device transfer command".into()).into(),
            )
        }
    };
    state
        .account()
        .await?
        .device_transfer_command(command)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn download_settings(
    state: State<'_, AppState>,
) -> Result<DownloadSettingsDto, CommandError> {
    let settings = state.desktop().await?.settings();
    Ok(DownloadSettingsDto {
        photo: settings.photo_auto_download().await?,
        video: settings.video_auto_download().await?,
        file: settings.file_auto_download().await?,
    })
}

#[tauri::command]
#[specta::specta]
async fn set_download_setting(
    state: State<'_, AppState>,
    kind: String,
    value: bool,
) -> Result<(), CommandError> {
    let settings = state.desktop().await?.settings();
    match kind.as_str() {
        "photo" => settings.set_photo_auto_download(value).await?,
        "video" => settings.set_video_auto_download(value).await?,
        "file" => settings.set_file_auto_download(value).await?,
        _ => return Err(ClientError::InvalidArgument("invalid download setting".into()).into()),
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn proxy_settings(state: State<'_, AppState>) -> Result<ProxySettingsItem, CommandError> {
    Ok(state.desktop().await?.settings().proxy_settings().await?)
}

#[tauri::command]
#[specta::specta]
async fn set_proxy_settings(
    state: State<'_, AppState>,
    settings: ProxySettingsItem,
) -> Result<(), CommandError> {
    state
        .desktop()
        .await?
        .settings()
        .set_proxy_settings(settings)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn mcp_settings(state: State<'_, AppState>) -> Result<McpSettingsItem, CommandError> {
    Ok(state.desktop().await?.settings().mcp_settings().await?)
}

#[tauri::command]
#[specta::specta]
async fn mcp_server_status(
    state: State<'_, AppState>,
) -> Result<McpServerStatusItem, CommandError> {
    Ok(state
        .desktop()
        .await?
        .settings()
        .mcp_server_status()
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn update_mcp_settings(
    state: State<'_, AppState>,
    settings: McpSettingsItem,
) -> Result<McpServerStatusItem, CommandError> {
    Ok(state
        .desktop()
        .await?
        .settings()
        .update_mcp_settings(settings)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, CommandError> {
    Ok(state.desktop().await?.settings().setting(key).await?)
}

#[tauri::command]
#[specta::specta]
async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: Option<String>,
) -> Result<(), CommandError> {
    state
        .desktop()
        .await?
        .settings()
        .set_setting(key, value)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn storage_usage(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationStorageUsage>, CommandError> {
    Ok(state.account().await?.storage_usage().await?)
}

#[tauri::command]
#[specta::specta]
async fn conversation_storage_usage(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<StorageCategoryUsage>, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation_storage_usage(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn clear_conversation_storage(
    state: State<'_, AppState>,
    conversation_id: String,
    categories: Vec<String>,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .clear_conversation_storage(conversation_id, categories)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn conversation_participants(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ConversationParticipantDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .conversation_participants(conversation_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn transcript_messages(
    state: State<'_, AppState>,
    transcript_id: String,
) -> Result<Vec<MessageDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .transcript_messages(transcript_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn mention_names(
    state: State<'_, AppState>,
    contents: Vec<String>,
) -> Result<std::collections::HashMap<String, String>, CommandError> {
    Ok(state
        .account()
        .await?
        .user()
        .mention_names(contents)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn update_participants(
    state: State<'_, AppState>,
    conversation_id: String,
    action: String,
    user_ids: Vec<String>,
    role: Option<String>,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .conversation()
        .update_participants(conversation_id, action, user_ids, role)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn conversation_detail(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationDetailItem, CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .conversation_detail(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn rotate_group_invite(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    Ok(state
        .account()
        .await?
        .conversation()
        .rotate_group_invite(conversation_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn set_disappearing_messages(
    state: State<'_, AppState>,
    conversation_id: String,
    duration: i64,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .conversation()
        .set_disappearing_messages(conversation_id, duration)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn shared_messages(
    state: State<'_, AppState>,
    conversation_id: String,
    kind: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<MessageDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .shared_messages(conversation_id, kind, offset, limit)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn shared_apps(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<SharedAppItem>, CommandError> {
    Ok(state.account().await?.user().shared_apps(user_id).await?)
}

#[tauri::command]
#[specta::specta]
async fn stickers(state: State<'_, AppState>) -> Result<Vec<StickerDto>, CommandError> {
    let access = state.account().await?.sticker();
    let mut values = access.recent_stickers().await?;
    let personal = access.personal_stickers().await?;
    let existing: std::collections::HashSet<_> =
        values.iter().map(|item| item.sticker_id.clone()).collect();
    values.extend(
        personal
            .into_iter()
            .filter(|item| !existing.contains(&item.sticker_id)),
    );
    Ok(values.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
async fn recent_stickers(state: State<'_, AppState>) -> Result<Vec<StickerDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .sticker()
        .recent_stickers()
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn personal_stickers(state: State<'_, AppState>) -> Result<Vec<StickerDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .sticker()
        .personal_stickers()
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn sticker_albums(state: State<'_, AppState>) -> Result<Vec<StickerAlbumItem>, CommandError> {
    Ok(state.account().await?.sticker().sticker_albums().await?)
}

#[tauri::command]
#[specta::specta]
async fn sticker_store_albums(
    state: State<'_, AppState>,
) -> Result<Vec<StickerAlbumItem>, CommandError> {
    Ok(state
        .account()
        .await?
        .sticker()
        .sticker_store_albums()
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn album_stickers(
    state: State<'_, AppState>,
    album_id: String,
) -> Result<Vec<StickerDto>, CommandError> {
    Ok(state
        .account()
        .await?
        .sticker()
        .album_stickers(album_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
async fn set_sticker_album_added(
    state: State<'_, AppState>,
    album_id: String,
    added: bool,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .set_sticker_album_added(album_id, added)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn set_sticker_album_order(
    state: State<'_, AppState>,
    album_ids: Vec<String>,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .set_sticker_album_order(album_ids)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn sticker_detail(
    state: State<'_, AppState>,
    sticker_id: String,
) -> Result<StickerDetailDto, CommandError> {
    Ok(state
        .account()
        .await?
        .sticker()
        .sticker_detail(sticker_id)
        .await?
        .into())
}

#[tauri::command]
#[specta::specta]
async fn refresh_sticker(
    state: State<'_, AppState>,
    sticker_id: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .refresh_sticker(sticker_id)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn remove_sticker(
    state: State<'_, AppState>,
    sticker_id: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .remove_sticker(sticker_id)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn add_sticker(state: State<'_, AppState>, sticker_id: String) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .add_sticker(sticker_id)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn add_sticker_from_file(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .add_sticker_from_file(message_id)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn add_sticker_from_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), CommandError> {
    state
        .account()
        .await?
        .sticker()
        .add_sticker_from_path(path)
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn send_sticker(
    state: State<'_, AppState>,
    conversation_id: String,
    sticker_id: String,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .send_sticker(conversation_id, sticker_id)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn send_contact(
    state: State<'_, AppState>,
    conversation_id: String,
    shared_user_id: String,
    quote_message_id: Option<String>,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .send_contact(conversation_id, shared_user_id, quote_message_id, false)
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn start_voice_recording(state: State<'_, AppState>) -> Result<(), CommandError> {
    state
        .desktop()
        .await?
        .media()
        .start_voice_recording()
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn cancel_voice_recording(state: State<'_, AppState>) -> Result<(), CommandError> {
    state
        .desktop()
        .await?
        .media()
        .cancel_voice_recording()
        .await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn stop_and_send_voice(
    state: State<'_, AppState>,
    conversation_id: String,
    quote_message_id: Option<String>,
) -> Result<String, CommandError> {
    let recording = state
        .desktop()
        .await?
        .media()
        .stop_voice_recording()
        .await?;
    Ok(state
        .account()
        .await?
        .message()
        .send_audio(
            conversation_id,
            recording.path,
            i64::try_from(recording.duration_millis)
                .map_err(|error| ClientError::Internal(error.to_string()))?,
            recording.waveform,
            quote_message_id,
        )
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn stop_voice_recording(state: State<'_, AppState>) -> Result<VoiceRecording, CommandError> {
    Ok(state
        .desktop()
        .await?
        .media()
        .stop_voice_recording()
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn send_voice_recording(
    state: State<'_, AppState>,
    conversation_id: String,
    recording: VoiceRecording,
    quote_message_id: Option<String>,
) -> Result<String, CommandError> {
    Ok(state
        .account()
        .await?
        .message()
        .send_audio(
            conversation_id,
            recording.path,
            i64::try_from(recording.duration_millis)
                .map_err(|error| ClientError::Internal(error.to_string()))?,
            recording.waveform,
            quote_message_id,
        )
        .await?)
}

#[tauri::command]
#[specta::specta]
async fn forward_messages(
    state: State<'_, AppState>,
    target_conversation_id: String,
    source_message_ids: Vec<String>,
    combined: bool,
) -> Result<Vec<String>, CommandError> {
    let message = state.account().await?.message();
    if combined {
        Ok(vec![
            message
                .combine_forward_messages(target_conversation_id, source_message_ids)
                .await?,
        ])
    } else {
        Ok(message
            .forward_messages(target_conversation_id, source_message_ids)
            .await?)
    }
}

#[tauri::command]
#[specta::specta]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
#[specta::specta]
async fn log_typescript(
    state: State<'_, AppState>,
    level: String,
    message: String,
) -> Result<(), CommandError> {
    state.desktop().await?;
    let level = match level.as_str() {
        "verbose" => mixin_desktop_api::LogLevel::Trace,
        "debug" => mixin_desktop_api::LogLevel::Debug,
        "info" => mixin_desktop_api::LogLevel::Info,
        "warning" => mixin_desktop_api::LogLevel::Warning,
        "error" | "wtf" => mixin_desktop_api::LogLevel::Error,
        _ => mixin_desktop_api::LogLevel::Info,
    };
    mixin_desktop_api::write_log("typescript", level, &message);
    Ok(())
}

#[tauri::command]
#[specta::specta]
fn logs() -> Result<Vec<String>, CommandError> {
    let directory = mixin_desktop_api::log_directory()?;
    let mut paths = fs::read_dir(directory)
        .map_err(|error| ClientError::Internal(error.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    let mut lines = Vec::new();
    for path in paths {
        let content =
            fs::read_to_string(path).map_err(|error| ClientError::Internal(error.to_string()))?;
        lines.extend(content.lines().map(str::to_owned));
    }
    Ok(lines)
}

#[tauri::command]
#[specta::specta]
async fn sign_out(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    let account = state.account.write().await.take();
    if let Some(account) = account {
        account.sign_out().await?;
    }
    state.events_started.store(false, Ordering::Release);
    #[cfg(target_os = "macos")]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_badge_count(None);
    }
    Ok(())
}

fn start_account_events(app: &AppHandle, state: &AppState, account: Arc<AccountClient>) {
    if state.events_started.swap(true, Ordering::AcqRel) {
        return;
    }

    let conversation_app = app.clone();
    let conversation_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = conversation_account.conversation_changes();
        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            if AccountConversationChangeEvent(event)
                .emit(&conversation_app)
                .is_err()
            {
                break;
            }
        }
    });

    let message_app = app.clone();
    let message_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = message_account.message_changes();
        futures::pin_mut!(stream);
        while let Some(revision) = stream.next().await {
            if AccountMessageChangeEvent(revision)
                .emit(&message_app)
                .is_err()
            {
                break;
            }
        }
    });

    let circle_app = app.clone();
    let circle_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = circle_account.circle_changes();
        futures::pin_mut!(stream);
        while let Some(circles) = stream.next().await {
            if AccountCircleChangeEvent(circles).emit(&circle_app).is_err() {
                break;
            }
        }
    });

    let unseen_app = app.clone();
    let unseen_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = unseen_account.unseen_count_changes();
        futures::pin_mut!(stream);
        while let Some(counts) = stream.next().await {
            if AccountUnseenCountChangeEvent(counts)
                .emit(&unseen_app)
                .is_err()
            {
                break;
            }
        }
    });

    #[cfg(target_os = "macos")]
    {
        let badge_app = app.clone();
        let badge_account = account.clone();
        tauri::async_runtime::spawn(async move {
            let stream = badge_account.unseen_message_count_changes();
            futures::pin_mut!(stream);
            while let Some(count) = stream.next().await {
                let Some(window) = badge_app.get_webview_window("main") else {
                    continue;
                };
                if window.set_badge_count(dock_badge_count(count)).is_err() {
                    break;
                }
            }
        });
    }

    let profile_app = app.clone();
    let profile_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = profile_account.profile_changes();
        futures::pin_mut!(stream);
        while let Some(profile) = stream.next().await {
            if AccountProfileChangeEvent(profile)
                .emit(&profile_app)
                .is_err()
            {
                break;
            }
        }
    });

    let connection_app = app.clone();
    let connection_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = connection_account.connection_status();
        futures::pin_mut!(stream);
        while let Some(connected) = stream.next().await {
            if AccountConnectionChangeEvent(connected)
                .emit(&connection_app)
                .is_err()
            {
                break;
            }
        }
    });

    let health_app = app.clone();
    let health_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = health_account.account_health();
        futures::pin_mut!(stream);
        while let Some(health) = stream.next().await {
            if AccountHealthChangeEvent(health).emit(&health_app).is_err() {
                break;
            }
        }
    });

    let transfer_app = app.clone();
    let transfer_account = account.clone();
    tauri::async_runtime::spawn(async move {
        let stream = transfer_account.device_transfer_events();
        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            if AccountDeviceTransferEvent(DeviceTransferEventDto::from(event))
                .emit(&transfer_app)
                .is_err()
            {
                break;
            }
        }
    });

    let notification_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let stream = account.notification_events();
        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            let Ok(event) = event else { break };
            if AccountNotificationEvent(event)
                .emit(&notification_app)
                .is_err()
            {
                break;
            }
        }
    });
}

fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::new()
        .commands(tauri_specta::collect_commands![
            biometric_available,
            authenticate_biometric,
            update_macos_menu,
            bootstrap,
            begin_login,
            wait_login,
            cancel_login,
            conversations_by_ids,
            all_conversations,
            groups_in_common,
            circles,
            search_user,
            search_local_users,
            search_mao_user,
            search_global_messages,
            user_profile,
            refresh_user_profile,
            selectable_users,
            open_user_conversation,
            take_pending_protocol_urls,
            resolve_code,
            join_group,
            snapshot_by_trace,
            snapshot_by_id,
            safe_snapshot_by_id,
            create_circle,
            update_circle,
            delete_circle,
            reorder_circles,
            create_group,
            edit_circle_conversation,
            edit_conversation,
            exit_group,
            clear_conversation,
            add_contact,
            remove_contact,
            block_user,
            unblock_user,
            report_user,
            bot_home_uri,
            open_bot_webview,
            send_post,
            send_app_card,
            messages,
            messages_around,
            send_text,
            conversation_is_encrypted,
            mark_conversation_read,
            update_draft,
            set_conversation_pinned,
            set_conversation_muted,
            delete_conversation,
            set_message_pinned,
            pinned_message_ids,
            pin_message_preview,
            unread_mention_message_ids,
            mark_mention_read,
            mark_audio_read,
            delete_messages,
            recall_messages,
            download_attachment,
            save_attachment_as,
            cancel_attachment,
            retry_attachment,
            write_edited_image,
            attachment_file_info,
            create_attachment_archive,
            send_attachment,
            send_remote_image,
            search_messages,
            update_profile,
            refresh_profile,
            refresh_account_health,
            retry_connection,
            device_transfer_command,
            download_settings,
            set_download_setting,
            proxy_settings,
            set_proxy_settings,
            mcp_settings,
            mcp_server_status,
            update_mcp_settings,
            setting,
            set_setting,
            storage_usage,
            conversation_storage_usage,
            clear_conversation_storage,
            conversation_participants,
            transcript_messages,
            mention_names,
            update_participants,
            conversation_detail,
            rotate_group_invite,
            set_disappearing_messages,
            shared_messages,
            shared_apps,
            stickers,
            recent_stickers,
            personal_stickers,
            sticker_albums,
            sticker_store_albums,
            album_stickers,
            set_sticker_album_added,
            set_sticker_album_order,
            sticker_detail,
            refresh_sticker,
            remove_sticker,
            add_sticker,
            add_sticker_from_file,
            add_sticker_from_path,
            send_sticker,
            send_contact,
            start_voice_recording,
            cancel_voice_recording,
            stop_and_send_voice,
            stop_voice_recording,
            send_voice_recording,
            forward_messages,
            app_version,
            log_typescript,
            logs,
            sign_out,
        ])
        .events(tauri_specta::collect_events![
            AppMenuActionEvent,
            AppProtocolUrlEvent,
            AccountConversationChangeEvent,
            AccountMessageChangeEvent,
            AccountCircleChangeEvent,
            AccountUnseenCountChangeEvent,
            AccountProfileChangeEvent,
            AccountConnectionChangeEvent,
            AccountHealthChangeEvent,
            AccountDeviceTransferEvent,
            AccountNotificationEvent,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .dangerously_cast_bigints_to_number()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, arguments, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            // The deep-link feature forwards configured URI schemes before this
            // callback, including on Windows and Linux secondary instances.
            // Keep the legacy web URI filter for callers that invoke the binary
            // directly with a Mixin web URL.
            let urls = protocol_urls(&arguments)
                .into_iter()
                .filter(|url| !url.starts_with("mixin:"))
                .collect();
            enqueue_protocol_urls(app, urls);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            #[cfg(target_os = "windows")]
            setup_system_tray(app)?;
            #[cfg(target_os = "macos")]
            setup_macos_menu(app)?;
            #[cfg(debug_assertions)]
            {
                let hmr = app.state::<AppState>().ui_debug_hmr.clone();
                app.listen("ui-debug-hmr", move |event| {
                    let Ok(event) = serde_json::from_str::<UiDebugHmrEvent>(event.payload()) else {
                        return;
                    };
                    if event.kind == "error" {
                        *hmr.error.lock().expect("HMR error lock") = event.message;
                    } else {
                        *hmr.error.lock().expect("HMR error lock") = None;
                        hmr.revision.fetch_add(1, Ordering::Release);
                    }
                    hmr.changed.notify_waiters();
                });
                start_ui_debug_server(app.handle().clone());
            }
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    window.set_decorations(true)?;
                    let close_window = window.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = close_window.hide();
                        }
                    });
                }
            }
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            if let Err(error) = app.deep_link().register_all() {
                eprintln!("register Mixin URL handler failed: {error}");
            }

            // The plugin receives initial configured scheme URLs before setup.
            // Store those URLs for the frontend's existing protocol queue, then
            // forward subsequent macOS open events through the same queue.
            if let Some(urls) = app.deep_link().get_current()? {
                enqueue_protocol_urls(
                    app.handle(),
                    urls.into_iter().map(|url| url.into()).collect(),
                );
            }
            let protocol_app = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                enqueue_protocol_urls(
                    &protocol_app,
                    event.urls().into_iter().map(|url| url.into()).collect(),
                );
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Mixin Messenger TypeScript desktop")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::RunEvent::Reopen { .. }) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}

#[cfg(test)]
mod binding_tests {
    use super::specta_builder;

    #[test]
    fn export_typescript_bindings() {
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/generated/bindings.ts");
        std::fs::create_dir_all(output.parent().expect("bindings output has a parent"))
            .expect("create bindings output directory");
        specta_builder()
            .export(specta_typescript::Typescript::default(), output)
            .expect("export TypeScript bindings");
    }
}
