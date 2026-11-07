//! Browser events preserve native source identity, navigation and cancellation.
use std::rc::Rc;

use serde_json::{Value, json};
use webview2_com::{
    DocumentTitleChangedEventHandler, HistoryChangedEventHandler,
    Microsoft::Web::WebView2::Win32::*, NavigationCompletedEventHandler,
    NavigationStartingEventHandler, NewWindowRequestedEventHandler, WebMessageReceivedEventHandler,
    WindowCloseRequestedEventHandler, take_pwstr,
};
use windows::core::{BOOL, Interface, PWSTR};

use super::{
    chrome, downloads,
    window::{State, is_web},
};

pub(super) fn install(state: &Rc<State>, web: &ICoreWebView2) -> windows::core::Result<()> {
    let weak = Rc::downgrade(state);
    let starting = NavigationStartingEventHandler::create(Box::new(move |_, args| {
        let (Some(state), Some(args)) = (weak.upgrade().filter(|state| !state.closed.get()), args)
        else {
            return Ok(());
        };
        let mut uri = PWSTR::null();
        unsafe {
            args.Uri(&mut uri)?;
        }
        let raw = take_pwstr(uri);
        let allowed = match url::Url::parse(&raw) {
            Ok(url) if is_mixin_action(&url) => {
                state.emit("navigate", Value::String(raw.clone()));
                false
            }
            Ok(url) if matches!(url.scheme(), "http" | "https" | "about" | "data" | "blob") => true,
            Ok(url) => {
                if !matches!(url.scheme(), "javascript" | "file") {
                    state.emit("external", Value::String(raw.clone()));
                }
                false
            }
            Err(_) => false,
        };
        if !allowed {
            unsafe {
                args.SetCancel(true)?;
            }
            return Ok(());
        }
        state.generation.set(state.generation.get().wrapping_add(1));
        state.pending.borrow_mut().clear();
        state.download_started.set(false);
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        web.add_NavigationStarting(&starting, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_NavigationStarting(token);
    }));

    let weak = Rc::downgrade(state);
    let completed = NavigationCompletedEventHandler::create(Box::new(move |_, args| {
        let (Some(state), Some(args)) = (weak.upgrade().filter(|state| !state.closed.get()), args)
        else {
            return Ok(());
        };
        state.native.clear_timeout();
        let mut success = BOOL(0);
        let mut status = COREWEBVIEW2_WEB_ERROR_STATUS_UNKNOWN;
        unsafe {
            args.IsSuccess(&mut success)?;
            args.WebErrorStatus(&mut status)?;
        }
        if !success.as_bool()
            && status != COREWEBVIEW2_WEB_ERROR_STATUS_OPERATION_CANCELED
            && !state.download_started.replace(false)
        {
            state.fail();
        }
        chrome::update(&state);
        Ok(())
    }));
    unsafe {
        web.add_NavigationCompleted(&completed, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_NavigationCompleted(token);
    }));

    let weak = Rc::downgrade(state);
    let message = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
        let (Some(state), Some(args)) = (weak.upgrade().filter(|state| !state.closed.get()), args)
        else {
            return Ok(());
        };
        // This event is ICoreWebView2's top-level event, never a Frame event.
        let mut source = PWSTR::null();
        let mut body = PWSTR::null();
        unsafe {
            args.Source(&mut source)?;
        }
        let source = take_pwstr(source);
        if !is_web(&source) || state.url().as_deref() != Some(&source) {
            return Ok(());
        }
        if unsafe { args.TryGetWebMessageAsString(&mut body) }.is_err() {
            return Ok(());
        }
        let body = take_pwstr(body);
        if body.len() > 1024 * 1024 {
            return Ok(());
        }
        let Ok(envelope) = serde_json::from_str::<Value>(&body) else {
            return Ok(());
        };
        let data = envelope.get("data").cloned().unwrap_or(Value::Null);
        match envelope.get("name").and_then(Value::as_str) {
            Some("close") => state.close(),
            Some("openInBrowser") => {
                if data.as_str().is_some_and(is_web) {
                    state.emit("browser", data);
                }
            }
            Some("signBotSignature") => {
                if let Some(mut data) = data.as_object().cloned() {
                    let Some(id) = data
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|id| !id.is_empty() && id.len() <= 256)
                    else {
                        return Ok(());
                    };
                    if state.pending.borrow().contains(id) {
                        return Ok(());
                    }
                    if state.pending.borrow().len() >= 32 {
                        state.reply(&json!({"generation":state.generation.get(),"page_url":source,"id":id,"args":["[]"]}));
                        return Ok(());
                    }
                    state.pending.borrow_mut().insert(id.to_owned());
                    data.insert("page_url".into(), Value::String(source));
                    data.insert("generation".into(), json!(state.generation.get()));
                    state.emit("signature", Value::Object(data));
                }
            }
            Some("theme") => chrome::theme(&state, data.get("color")),
            _ => {}
        }
        Ok(())
    }));
    unsafe {
        web.add_WebMessageReceived(&message, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_WebMessageReceived(token);
    }));

    let weak = Rc::downgrade(state);
    let popup = NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            args.SetHandled(true)?;
        }
        let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) else {
            return Ok(());
        };
        let mut uri = PWSTR::null();
        unsafe {
            args.Uri(&mut uri)?;
        }
        let raw = take_pwstr(uri);
        if is_web(&raw) {
            state.navigate(&raw);
        } else if let Ok(url) = url::Url::parse(&raw) {
            if is_mixin_action(&url) {
                state.emit("navigate", Value::String(raw));
            } else if !matches!(url.scheme(), "javascript" | "file") {
                state.emit("external", Value::String(raw));
            }
        }
        Ok(())
    }));
    unsafe {
        web.add_NewWindowRequested(&popup, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_NewWindowRequested(token);
    }));

    let weak = Rc::downgrade(state);
    let closing = WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
        if let Some(state) = weak.upgrade() {
            state.close();
        }
        Ok(())
    }));
    unsafe {
        web.add_WindowCloseRequested(&closing, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_WindowCloseRequested(token);
    }));

    let weak = Rc::downgrade(state);
    let title = DocumentTitleChangedEventHandler::create(Box::new(move |_, _| {
        if let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) {
            chrome::update(&state);
        }
        Ok(())
    }));
    unsafe {
        web.add_DocumentTitleChanged(&title, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_DocumentTitleChanged(token);
    }));

    let weak = Rc::downgrade(state);
    let history = HistoryChangedEventHandler::create(Box::new(move |_, _| {
        if let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) {
            chrome::update(&state);
        }
        Ok(())
    }));
    unsafe {
        web.add_HistoryChanged(&history, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_HistoryChanged(token);
    }));

    downloads::install(state, &web.cast::<ICoreWebView2_4>()?)?;
    Ok(())
}

fn is_mixin_action(url: &url::Url) -> bool {
    url.scheme() == "mixin"
        || matches!(url.scheme(), "http" | "https")
            && matches!(url.host_str(), Some("mixin.one" | "www.mixin.one"))
}
