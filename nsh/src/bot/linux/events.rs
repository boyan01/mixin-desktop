//! Web-process messages are accepted only after a native document challenge.
use super::{
    chrome,
    window::{State, is_web},
};
use nsh_platform_linux::gtk4 as gtk;
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};
use webkit6::{
    LoadEvent, NavigationPolicyDecision, PolicyDecisionType, ResponsePolicyDecision, UserMessage,
    prelude::*,
};

thread_local! { static GENERATION: Cell<u64> = const { Cell::new(0) }; }

pub(super) fn next_generation() -> u64 {
    GENERATION.with(|next| {
        let n = next.get().wrapping_add(1);
        next.set(n);
        n
    })
}

fn probe(state: &State) {
    let message = UserMessage::new(
        "mixin-bot-probe",
        Some(&state.gate.generation.get().to_variant()),
    );
    state
        .web
        .send_message_to_page(&message, gtk::gio::Cancellable::NONE, |_| {});
}

pub(super) fn install(state: &Rc<State>) {
    macro_rules! connect {
        ($method:ident, |$s:ident $(, $arg:ident)*| $body:block) => {{
            let weak = Rc::downgrade(state);
            let id = state.web.$method(move |_ $(, $arg)*| {
                if let Some($s) = weak.upgrade().filter(|s| !s.closed.get()) $body
            });
            state.handlers.borrow_mut().push(id);
        }};
    }
    connect!(connect_load_changed, |s, event| {
        match event {
            LoadEvent::Started => {
                s.gate.begin_navigation(next_generation());
                s.arm_timeout();
            }
            LoadEvent::Committed => {
                s.gate.committed.set(true);
                probe(&s);
            }
            LoadEvent::Finished if s.gate.is_ready() => {
                s.stop_timeout();
            }
            _ => {}
        }
        chrome::update(&s);
    });
    connect!(connect_web_process_terminated, |s, _reason| {
        s.stop_timeout();
        let reusable = s.gate.terminate(next_generation());
        s.fail();
        if !reusable {
            s.close();
        }
    });
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_load_failed(move |_, _, _, error| {
            if let Some(s) = weak.upgrade().filter(|s| !s.closed.get())
                && !error.matches(webkit6::NetworkError::Cancelled)
            {
                s.stop_timeout();
                s.fail();
            }
            false
        }));
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_user_message_received(move |_, message| {
            let Some(s) = weak.upgrade().filter(|s| !s.closed.get()) else {
                return true;
            };
            let Some(parameters) = message.parameters() else {
                return false;
            };
            match message.name().as_deref() {
                Some("mixin-bot-document") => {
                    if let Some((epoch, token)) = parameters.get::<(String, u64)>()
                        && s.gate.announce(&epoch, token)
                        && s.gate.committed.get()
                    {
                        probe(&s);
                    }
                }
                Some("mixin-bot-ready") => {
                    if let Some((epoch, token, uri, generation)) =
                        parameters.get::<(String, u64, String, u64)>()
                        && is_web(&uri)
                        && s.gate
                            .ready(&epoch, token, &uri, generation, s.web.uri().as_deref())
                    {
                        s.stop_timeout();
                    }
                }
                Some("mixin-bot-message") => {
                    let Some((epoch, token, uri, generation, body)) =
                        parameters.get::<(String, u64, String, u64, String)>()
                    else {
                        return true;
                    };
                    if !is_web(&uri)
                        || !s
                            .gate
                            .accepts(&epoch, token, &uri, generation, s.web.uri().as_deref())
                        || body.len() > 1024 * 1024
                    {
                        return true;
                    }
                    let Ok(envelope) = serde_json::from_str::<Value>(&body) else {
                        return true;
                    };
                    let data = envelope.get("data").cloned().unwrap_or(Value::Null);
                    match envelope.get("name").and_then(Value::as_str) {
                        Some("close") => s.close(),
                        Some("openInBrowser") if data.as_str().is_some_and(is_web) => {
                            s.emit("browser", data)
                        }
                        Some("signBotSignature") => {
                            let Some(mut data) = data.as_object().cloned() else {
                                return true;
                            };
                            let Some(id) = data
                                .get("id")
                                .and_then(Value::as_str)
                                .filter(|id| !id.is_empty() && id.len() <= 256)
                                .map(str::to_owned)
                            else {
                                return true;
                            };
                            if s.gate.pending.borrow().contains(&id) {
                                return true;
                            }
                            if s.gate.pending.borrow().len() >= 32 {
                                s.evaluate(&format!(
                                    "window.__mixinBotReply?.({},[\"[]\"]);",
                                    json!(id)
                                ));
                                return true;
                            }
                            s.gate.pending.borrow_mut().insert(id);
                            data.insert("page_url".into(), json!(uri));
                            data.insert("generation".into(), json!(generation));
                            s.emit("signature", Value::Object(data));
                        }
                        Some("theme") => chrome::theme(&s, data.get("color")),
                        _ => {}
                    }
                }
                _ => return false,
            }
            true
        }));
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_decide_policy(move |_, decision, kind| {
            let Some(s) = weak.upgrade().filter(|s| !s.closed.get()) else {
                decision.ignore();
                return true;
            };
            if kind == PolicyDecisionType::Response {
                if let Some(response) = decision.downcast_ref::<ResponsePolicyDecision>()
                    && !response.is_mime_type_supported()
                {
                    decision.download();
                    return true;
                }
                return false;
            }
            let Some(action) = decision
                .downcast_ref::<NavigationPolicyDecision>()
                .and_then(|d| d.navigation_action())
            else {
                return false;
            };
            let Some(uri) = action.request().and_then(|r| r.uri()) else {
                decision.ignore();
                return true;
            };
            if let Ok(url) = url::Url::parse(&uri) {
                if url.scheme() == "mixin"
                    || matches!(url.scheme(), "http" | "https")
                        && matches!(url.host_str(), Some("mixin.one" | "www.mixin.one"))
                {
                    decision.ignore();
                    s.emit("navigate", json!(uri.as_str()));
                    return true;
                }
                if is_web(&uri) || uri == "about:blank" {
                    return false;
                }
                if !matches!(url.scheme(), "javascript" | "file" | "data" | "blob") {
                    s.emit("external", json!(uri.as_str()));
                }
            }
            decision.ignore();
            true
        }));
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_create(move |_, action| {
            if let Some(s) = weak.upgrade().filter(|s| !s.closed.get())
                && let Some(uri) = action.request().and_then(|request| request.uri())
            {
                // Flutter's desktop plugin navigates target=_blank in place.
                // load_uri still passes the normal native navigation policy.
                s.web.load_uri(&uri);
            }
            None
        }));
    connect!(connect_close, |s| {
        s.close();
    });
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_script_dialog(move |_, dialog| {
            if let Some(s) = weak.upgrade().filter(|s| !s.closed.get()) {
                s.dialogs.script(&s, dialog);
            } else {
                dialog.close();
            }
            true
        }));
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_run_file_chooser(move |_, request| {
            if let Some(s) = weak.upgrade().filter(|s| !s.closed.get()) {
                s.dialogs.files(&s, request);
            } else {
                request.cancel();
            }
            true
        }));
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_permission_request(|_, request| {
            request.deny();
            true
        }));
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_uri_notify(move |_| {
            if let Some(s) = weak
                .upgrade()
                .filter(|s| !s.closed.get() && s.gate.committed.get())
            {
                probe(&s);
                chrome::update(&s);
            }
        }));
    let weak = Rc::downgrade(state);
    state
        .handlers
        .borrow_mut()
        .push(state.web.connect_title_notify(move |_| {
            if let Some(s) = weak.upgrade().filter(|s| !s.closed.get()) {
                chrome::update(&s);
            }
        }));
}
