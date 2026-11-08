//! WebKit completion handlers belong to their browser window and answer exactly once.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use block2::{DynBlock, RcBlock};
use nsh_platform_macos::objc2_app_kit::{NSAlert, NSOpenPanel, NSSavePanel, NSTextField, NSWindow};
use objc2::{
    MainThreadMarker, MainThreadOnly, msg_send,
    rc::{Retained, Weak},
    runtime::{AnyObject, Bool},
    sel,
};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString, NSURL};
use serde_json::Value;

enum Question<'a> {
    Alert,
    Confirm,
    Prompt(&'a NSString),
}

struct Pending {
    id: u64,
    sheet: Retained<NSWindow>,
    cancel: Box<dyn Fn()>,
}

#[derive(Default)]
pub(super) struct Dialogs {
    pending: Rc<RefCell<Vec<Pending>>>,
    next: Cell<u64>,
}

impl Dialogs {
    fn id(&self) -> Option<u64> {
        if self.pending.borrow().len() >= 16 {
            return None;
        }
        let id = self.next.get().wrapping_add(1);
        self.next.set(id);
        Some(id)
    }

    pub(super) fn cancel_all(&self, window: Option<&NSWindow>) {
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        for item in pending {
            // Resolve even sheets queued behind another sheet. AppKit may deliver
            // its callback later; the shared once flag prevents a second answer.
            (item.cancel)();
            if let Some(window) = window {
                window.endSheet_returnCode(&item.sheet, 0);
            }
            item.sheet.orderOut(None);
        }
    }

    fn alert(
        &self,
        window: &NSWindow,
        host: &str,
        message: &NSString,
        question: Question<'_>,
        labels: &Value,
        answer: impl Fn(isize, Option<String>) + 'static,
    ) {
        let Some(id) = self.id() else {
            answer(0, None);
            return;
        };
        let mtm = MainThreadMarker::new().expect("WebKit dialogs run on the main thread");
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(if host.is_empty() {
            super::label(labels, "Webpage")
        } else {
            host
        }));
        alert.setInformativeText(message);
        alert.addButtonWithTitle(&NSString::from_str(super::label(labels, "OK")));
        if !matches!(question, Question::Alert) {
            alert.addButtonWithTitle(&NSString::from_str(super::label(labels, "Cancel")));
        }
        let text = match question {
            Question::Prompt(value) => Some(value),
            _ => None,
        };
        let field = text.map(|value| {
            let field = NSTextField::initWithFrame(
                NSTextField::alloc(mtm),
                NSRect::new(NSPoint::ZERO, NSSize::new(280.0, 24.0)),
            );
            field.setStringValue(value);
            alert.setAccessoryView(Some(&field));
            field
        });
        let once = Rc::new(Cell::new(false));
        let answer = Rc::new(answer);
        let cancel_once = once.clone();
        let cancel_answer = answer.clone();
        self.pending.borrow_mut().push(Pending {
            id,
            sheet: alert.window(),
            cancel: Box::new(move || {
                if !cancel_once.replace(true) {
                    cancel_answer(0, None);
                }
            }),
        });
        let pending = Rc::downgrade(&self.pending);
        let handler = RcBlock::new(move |response: isize| {
            if let Some(pending) = pending.upgrade() {
                pending.borrow_mut().retain(|item| item.id != id);
            }
            if !once.replace(true) {
                answer(
                    response,
                    field.as_ref().map(|field| field.stringValue().to_string()),
                );
            }
        });
        alert.beginSheetModalForWindow_completionHandler(window, Some(&handler));
    }

    pub(super) fn javascript_alert(
        &self,
        window: Option<&NSWindow>,
        host: &str,
        message: &NSString,
        labels: &Value,
        done: &DynBlock<dyn Fn()>,
    ) {
        let Some(window) = window else {
            done.call(());
            return;
        };
        let done = done.copy();
        self.alert(
            window,
            host,
            message,
            Question::Alert,
            labels,
            move |_, _| done.call(()),
        );
    }

    pub(super) fn javascript_confirm(
        &self,
        window: Option<&NSWindow>,
        host: &str,
        message: &NSString,
        labels: &Value,
        done: &DynBlock<dyn Fn(Bool)>,
    ) {
        let Some(window) = window else {
            done.call((Bool::NO,));
            return;
        };
        let done = done.copy();
        self.alert(
            window,
            host,
            message,
            Question::Confirm,
            labels,
            move |response, _| done.call((Bool::new(response == 1000),)),
        );
    }

    pub(super) fn javascript_prompt(
        &self,
        window: Option<&NSWindow>,
        host: &str,
        message: &NSString,
        default: Option<&NSString>,
        labels: &Value,
        done: &DynBlock<dyn Fn(*mut NSString)>,
    ) {
        let Some(window) = window else {
            done.call((std::ptr::null_mut(),));
            return;
        };
        let done = done.copy();
        let default = default.map(|value| value.to_owned()).unwrap_or_default();
        self.alert(
            window,
            host,
            message,
            Question::Prompt(&default),
            labels,
            move |response, text| {
                let text = (response == 1000)
                    .then(|| NSString::from_str(text.as_deref().unwrap_or_default()));
                done.call((text.as_ref().map_or(std::ptr::null_mut(), |text| {
                    Retained::as_ptr(text).cast_mut()
                }),));
            },
        );
    }

    pub(super) fn open(
        &self,
        window: Option<&NSWindow>,
        parameters: &AnyObject,
        done: &DynBlock<dyn Fn(*mut NSArray<NSURL>)>,
    ) {
        let (Some(window), Some(id)) = (window, self.id()) else {
            done.call((std::ptr::null_mut(),));
            return;
        };
        let mtm = MainThreadMarker::new().expect("WebKit panels run on the main thread");
        let panel = NSOpenPanel::openPanel(mtm);
        panel.setCanChooseFiles(true);
        let multiple: Bool = unsafe { msg_send![parameters, allowsMultipleSelection] };
        let supports_directories: Bool =
            unsafe { msg_send![parameters, respondsToSelector: sel!(allowsDirectories)] };
        let directories: Bool = if supports_directories.as_bool() {
            unsafe { msg_send![parameters, allowsDirectories] }
        } else {
            Bool::NO
        };
        panel.setAllowsMultipleSelection(multiple.as_bool());
        panel.setCanChooseDirectories(directories.as_bool());
        let done = done.copy();
        let once = Rc::new(Cell::new(false));
        let cancel_once = once.clone();
        let cancel_done = done.clone();
        let sheet: Retained<NSWindow> = unsafe {
            Retained::retain((&*panel as *const NSOpenPanel).cast_mut().cast())
                .expect("panel window")
        };
        self.pending.borrow_mut().push(Pending {
            id,
            sheet,
            cancel: Box::new(move || {
                if !cancel_once.replace(true) {
                    cancel_done.call((std::ptr::null_mut(),));
                }
            }),
        });
        let selected_panel = Weak::new(&*panel);
        let pending = Rc::downgrade(&self.pending);
        let handler = RcBlock::new(move |response: isize| {
            let panel = selected_panel.load();
            if let Some(pending) = pending.upgrade() {
                pending.borrow_mut().retain(|item| item.id != id);
            }
            if once.replace(true) {
                return;
            }
            let urls = (response == 1)
                .then(|| panel.as_ref().map(|panel| panel.URLs()))
                .flatten();
            done.call((urls.as_ref().map_or(std::ptr::null_mut(), |urls| {
                Retained::as_ptr(urls).cast_mut()
            }),));
        });
        panel.beginSheetModalForWindow_completionHandler(window, &handler);
    }

    pub(super) fn save(
        &self,
        window: Option<&NSWindow>,
        name: &str,
        answer: impl Fn(Option<Retained<NSURL>>) + 'static,
    ) {
        let (Some(window), Some(id)) = (window, self.id()) else {
            answer(None);
            return;
        };
        let mtm = MainThreadMarker::new().expect("WebKit panels run on the main thread");
        let panel = NSSavePanel::savePanel(mtm);
        panel.setCanCreateDirectories(true);
        let name = std::path::Path::new(name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("download");
        panel.setNameFieldStringValue(&NSString::from_str(name));
        let once = Rc::new(Cell::new(false));
        let answer = Rc::new(answer);
        let cancel_once = once.clone();
        let cancel_answer = answer.clone();
        let sheet: Retained<NSWindow> = unsafe {
            Retained::retain((&*panel as *const NSSavePanel).cast_mut().cast())
                .expect("panel window")
        };
        self.pending.borrow_mut().push(Pending {
            id,
            sheet,
            cancel: Box::new(move || {
                if !cancel_once.replace(true) {
                    cancel_answer(None);
                }
            }),
        });
        let selected_panel = Weak::new(&*panel);
        let pending = Rc::downgrade(&self.pending);
        let handler = RcBlock::new(move |response: isize| {
            let panel = selected_panel.load();
            if let Some(pending) = pending.upgrade() {
                pending.borrow_mut().retain(|item| item.id != id);
            }
            if !once.replace(true) {
                answer(if response == 1 {
                    panel.as_ref().and_then(|panel| panel.URL())
                } else {
                    None
                });
            }
        });
        panel.beginSheetModalForWindow_completionHandler(window, &handler);
    }
}

pub(super) fn frame_host(frame: &AnyObject) -> String {
    // A blank document can inherit an origin without a host in request.URL.
    // WebKit supplies this origin; page JSON never provides it.
    let origin: Retained<AnyObject> = unsafe { msg_send![frame, securityOrigin] };
    let host: Retained<NSString> = unsafe { msg_send![&origin, host] };
    host.to_string()
}
