use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use nsh_platform_windows::{ComponentContext, NativeComponent};
use serde_json::{Value, json};
use webview2_com::{
    AddScriptToExecuteOnDocumentCreatedCompletedHandler,
    CreateCoreWebView2ControllerCompletedHandler, Microsoft::Web::WebView2::Win32::*, take_pwstr,
};
use windows::{
    Win32::UI::Shell::IFileSaveDialog,
    core::{HSTRING, Interface, PWSTR},
};

use super::{chrome, dispatch, events, session};

pub(super) struct State {
    pub context: ComponentContext,
    pub native: Rc<dispatch::Window>,
    pub page: RefCell<Value>,
    pub web: RefCell<Option<ICoreWebView2>>,
    pub controller: RefCell<Option<ICoreWebView2Controller>>,
    pub removers: RefCell<Vec<Box<dyn FnOnce()>>>,
    pub generation: Cell<u64>,
    pub closed: Cell<bool>,
    pub dark: Cell<bool>,
    pub download_started: Cell<bool>,
    pub chrome: RefCell<Option<chrome::Chrome>>,
    pub pending: RefCell<HashSet<String>>,
    pub saving: RefCell<Option<IFileSaveDialog>>,
    pub downloads: RefCell<Vec<ICoreWebView2DownloadOperation>>,
    pub icon: RefCell<Option<chrome::Icon>>,
    script: RefCell<Option<String>>,
    script_revision: Cell<u64>,
    navigated: Cell<bool>,
    focus: Cell<u64>,
}

impl State {
    fn open(
        context: ComponentContext,
        page: Value,
        entry: Rc<session::Entry>,
        dark: bool,
    ) -> Result<Rc<Self>, &'static str> {
        let raw = page
            .get("url")
            .and_then(Value::as_str)
            .ok_or("Missing page URL")?;
        if !is_web(raw) {
            return Err("Invalid browser URL");
        }
        let title = page
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let native =
            dispatch::Window::new(title, true).map_err(|_| "Could not create browser window")?;
        let state = Rc::new(Self {
            context,
            native,
            page: RefCell::new(page),
            web: RefCell::default(),
            controller: RefCell::default(),
            removers: RefCell::default(),
            generation: Cell::new(0),
            closed: Cell::new(false),
            dark: Cell::new(dark),
            download_started: Cell::new(false),
            chrome: RefCell::default(),
            pending: RefCell::default(),
            saving: RefCell::default(),
            downloads: RefCell::default(),
            icon: RefCell::default(),
            script: RefCell::default(),
            script_revision: Cell::new(0),
            navigated: Cell::new(false),
            focus: Cell::new(0),
        });
        chrome::install(&state).map_err(|_| "Could not create browser controls")?;
        let weak = Rc::downgrade(&state);
        *state.native.closing.borrow_mut() = Some(Box::new(move || {
            if let Some(state) = weak.upgrade() {
                state.close();
            }
        }));
        let weak = Rc::downgrade(&state);
        *state.native.resize.borrow_mut() = Some(Box::new(move || {
            if let Some(state) = weak.upgrade() {
                state.resize();
            }
        }));
        let weak = Rc::downgrade(&state);
        state.native.timeout(move || {
            if let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) {
                state.fail();
                state.close();
            }
        });
        entry.register(&state);
        let weak = Rc::downgrade(&state);
        entry.environment(move |environment| {
            let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) else {
                return;
            };
            let Some(environment) = environment else {
                state.fail();
                state.close();
                return;
            };
            let weak = Rc::downgrade(&state);
            let done = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                move |result, controller| {
                    let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) else {
                        if let Some(controller) = controller {
                            unsafe {
                                let _ = controller.Close();
                            }
                        }
                        return Ok(());
                    };
                    let Some(controller) = result.ok().and(controller) else {
                        state.fail();
                        state.close();
                        return Ok(());
                    };
                    if state.attach(controller).is_err() {
                        state.fail();
                        state.close();
                    }
                    Ok(())
                },
            ));
            if unsafe { environment.CreateCoreWebView2Controller(state.native.hwnd.get(), &done) }
                .is_err()
            {
                state.fail();
                state.close();
            }
        });
        Ok(state)
    }

    fn attach(self: &Rc<Self>, controller: ICoreWebView2Controller) -> windows::core::Result<()> {
        *self.controller.borrow_mut() = Some(controller.clone());
        let web = unsafe { controller.CoreWebView2()? };
        let settings = unsafe { web.Settings()? };
        unsafe {
            settings.SetIsScriptEnabled(true)?;
            settings.SetIsWebMessageEnabled(true)?;
            settings.SetIsStatusBarEnabled(false)?;
            // Native WebView2 dialogs provide alert, confirm and prompt without
            // running a reentrant modal loop inside an application event handler.
            settings.SetAreDefaultScriptDialogsEnabled(true)?;
            settings.SetAreDevToolsEnabled(cfg!(debug_assertions))?;
        }
        if let Ok(settings) = settings.cast::<ICoreWebView2Settings2>() {
            let page = self.page.borrow();
            let version = page
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or_default();
            unsafe {
                let mut original = PWSTR::null();
                settings.UserAgent(&mut original)?;
                let original = take_pwstr(original);
                settings.SetUserAgent(&HSTRING::from(format!("{original} Mixin/{version}")))?;
            }
        }
        *self.web.borrow_mut() = Some(web.clone());
        super::focus::browser(self, &controller)?;
        events::install(self, &web)?;
        self.resize();
        self.presentation();
        Ok(())
    }

    pub fn url(&self) -> Option<String> {
        let web = self.web.borrow().clone()?;
        let mut source = PWSTR::null();
        unsafe {
            web.Source(&mut source).ok()?;
        }
        Some(take_pwstr(source))
    }

    pub fn title(&self) -> String {
        let Some(web) = self.web.borrow().clone() else {
            return String::new();
        };
        let mut title = PWSTR::null();
        if unsafe { web.DocumentTitle(&mut title) }.is_ok() {
            take_pwstr(title)
        } else {
            String::new()
        }
    }

    pub fn emit(&self, name: &str, value: Value) {
        if !self.closed.get() {
            self.context.emit(name, value);
        }
    }
    pub fn fail(&self) {
        self.emit("failed", Value::Null);
    }

    pub fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        self.pending.borrow_mut().clear();
        let dialog = self.saving.borrow_mut().take();
        if let Some(dialog) = dialog {
            unsafe {
                let _ = dialog.Close(windows::Win32::Foundation::E_ABORT);
            }
        }
        let downloads = self.downloads.borrow_mut().drain(..).collect::<Vec<_>>();
        for operation in downloads {
            unsafe {
                let _ = operation.Cancel();
            }
        }
        for remove in self.removers.borrow_mut().drain(..).collect::<Vec<_>>() {
            remove();
        }
        if let Some(web) = self.web.borrow_mut().take() {
            unsafe {
                let _ = web.Stop();
            }
        }
        if let Some(controller) = self.controller.borrow_mut().take() {
            unsafe {
                let _ = controller.Close();
            }
        }
        chrome::clear_icon(self);
        self.chrome.borrow_mut().take();
        self.native.close();
        self.context.emit("closed", Value::Null);
    }

    pub fn resize(&self) {
        let mut rect = self.native.bounds();
        let header = chrome::layout(self, &rect);
        rect.top += header;
        if let Some(controller) = self.controller.borrow().as_ref() {
            unsafe {
                let _ = controller.SetBounds(rect);
            }
        }
        chrome::raise(self);
    }

    pub fn presentation(self: &Rc<Self>) {
        chrome::update(self);
        let Some(web) = self.web.borrow().clone() else {
            return;
        };
        let page = self.page.borrow();
        let script = page
            .get("script")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let locale = page
            .get("locale")
            .and_then(Value::as_str)
            .unwrap_or_default();
        // Main-frame-only injection is defense in depth. Message source and
        // generation are still taken from native WebView2 events below.
        let script = format!("if (window === window.top) {{\n{script}\n}}");
        let revision = self.script_revision.get().wrapping_add(1);
        self.script_revision.set(revision);
        if let Some(id) = self.script.borrow_mut().take() {
            unsafe {
                let _ = web.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id));
            }
        }
        let weak = Rc::downgrade(self);
        let done = AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(
            move |result, id| {
                let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) else {
                    return Ok(());
                };
                if revision != state.script_revision.get() {
                    if let Some(web) = state.web.borrow().as_ref() {
                        unsafe {
                            let _ = web.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id));
                        }
                    }
                    return Ok(());
                }
                if result.is_err() || id.is_empty() {
                    state.fail();
                    return Ok(());
                }
                *state.script.borrow_mut() = Some(id);
                if !state.navigated.replace(true) {
                    let url = state
                        .page
                        .borrow()
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    state.navigate(&url);
                }
                Ok(())
            },
        ));
        if unsafe { web.AddScriptToExecuteOnDocumentCreated(&HSTRING::from(script), &done) }
            .is_err()
        {
            self.fail();
        }
        let locale_script = format!("window.__mixinBotSetLocale?.({});", json!(locale));
        unsafe {
            let _ = web.ExecuteScript(&HSTRING::from(locale_script), None);
        }
        let focus = page
            .get("focus")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        if self.focus.replace(focus) != focus {
            self.native.focus();
        }
        if let Ok(profile) = unsafe { web.cast::<ICoreWebView2_13>().and_then(|web| web.Profile()) }
        {
            unsafe {
                let _ = profile.SetPreferredColorScheme(if self.dark.get() {
                    COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK
                } else {
                    COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT
                });
            }
        }
    }

    pub fn navigate(&self, raw: &str) {
        if !self.closed.get()
            && is_web(raw)
            && let Some(web) = self.web.borrow().as_ref()
            && unsafe { web.Navigate(&HSTRING::from(raw)) }.is_err()
        {
            self.fail();
        }
    }

    pub fn reply(&self, reply: &Value) {
        if self.closed.get()
            || reply.get("generation").and_then(Value::as_u64) != Some(self.generation.get())
            || reply.get("page_url").and_then(Value::as_str) != self.url().as_deref()
        {
            return;
        }
        let (Some(id), Some(args)) = (
            reply.get("id").and_then(Value::as_str),
            reply.get("args").and_then(Value::as_array),
        ) else {
            return;
        };
        if id.len() > 256 {
            return;
        }
        self.pending.borrow_mut().remove(id);
        let script = format!("window.__mixinBotReply?.({}, {});", json!(id), json!(args));
        if let Some(web) = self.web.borrow().as_ref() {
            unsafe {
                let _ = web.ExecuteScript(&HSTRING::from(script), None);
            }
        }
    }
}

pub(super) fn is_web(raw: &str) -> bool {
    url::Url::parse(raw).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

pub(super) struct Host {
    context: ComponentContext,
    session: String,
    pending: Option<Value>,
    state: Option<Rc<State>>,
    dark: bool,
}
impl Host {
    pub fn new(context: ComponentContext) -> Self {
        Self {
            context,
            session: String::new(),
            pending: None,
            state: None,
            dark: false,
        }
    }
    fn page(&mut self, page: &Value) {
        if self.session.is_empty() {
            self.pending = Some(page.clone());
            return;
        }
        if let Some(state) = &self.state {
            *state.page.borrow_mut() = page.clone();
            state.presentation();
            return;
        }
        let Some(entry) = session::get(&self.session) else {
            self.context.emit("failed", Value::Null);
            return;
        };
        match State::open(self.context.clone(), page.clone(), entry, self.dark) {
            Ok(state) => self.state = Some(state),
            Err(_) => self.context.emit("failed", Value::Null),
        }
    }
}
impl NativeComponent for Host {
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        match name {
            "session" => {
                self.session = value.and_then(Value::as_str).unwrap_or_default().to_owned();
                if let Some(page) = self.pending.take() {
                    self.page(&page);
                }
            }
            "dark" => {
                self.dark = value.and_then(Value::as_bool).unwrap_or_default();
                if let Some(state) = &self.state {
                    state.dark.set(self.dark);
                    state.presentation();
                }
            }
            "page" => {
                if let Some(page) = value {
                    self.page(page);
                }
            }
            _ => {}
        }
    }
    fn command(&mut self, name: &str, args: &Value) {
        if name == "reply"
            && let Some(state) = &self.state
        {
            state.reply(args);
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            state.close();
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.close();
    }
}
