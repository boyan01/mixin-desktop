//! macOS shell integration without replacing NSH's application/window delegates.
use std::{cell::Cell, rc::Rc};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use nsh_platform_macos::{
    ComponentContext, Components, NativeComponent,
    objc2_app_kit::{
        NSApplication, NSModalResponseCancel, NSView, NSWindowWillBeginSheetNotification,
    },
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::AnyObject, sel,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSRect, NSSize, NSString,
};
use serde_json::Value;

mod urls;
pub(super) use urls::UrlCapture;

struct ViewIvars {
    context: ComponentContext,
    blocked: Cell<bool>,
    configured: Cell<bool>,
    frame_name: String,
}

define_class!(
    // SAFETY: this NSView and all its ivars are main-thread confined.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[name = "MixinNshShellView"]
    #[ivars = ViewIvars]
    struct ShellView;
    unsafe impl NSObjectProtocol for ShellView {}
    impl ShellView {
        #[unsafe(method(sheetWillBegin:))]
        fn sheet_will_begin(&self, notification: &NSNotification) {
            if !self.ivars().blocked.get() { return; }
            if let (Some(window), Some(object)) = (self.window(), notification.object())
                && Retained::as_ptr(&window).cast::<AnyObject>() == Retained::as_ptr(&object)
            {
                // The notification precedes attachment. Route a deferred command
                // through the engine, so the sheet has attached before cancellation.
                self.ivars().context.emit("sheet", Value::Null);
            }
        }
        #[unsafe(method(viewDidMoveToWindow))]
        fn moved(&self) {
            // SAFETY: NSView implements viewDidMoveToWindow, with no arguments or return.
            unsafe { let _: () = msg_send![super(self), viewDidMoveToWindow]; }
            if self.ivars().configured.get() { return; }
            if let Some(window) = self.window() {
                window.setMinSize(NSSize::new(384.0, 480.0));
                let name = NSString::from_str(&self.ivars().frame_name);
                window.setFrameUsingName(&name);
                window.setFrameAutosaveName(&name);
                self.ivars().configured.set(true);
            }
        }
    }
);

struct Host {
    view: Retained<ShellView>,
    _urls: UrlCapture,
}
impl Host {
    fn new(context: ComponentContext) -> Self {
        let mtm =
            MainThreadMarker::new().expect("Native components are created on AppKit's thread");
        let namespace = mixin_desktop_api::data_directory()
            .ok()
            .and_then(|path| std::fs::canonicalize(path).ok())
            .map(|path| URL_SAFE_NO_PAD.encode(path.as_os_str().as_encoded_bytes()))
            .unwrap_or_else(|| "default".into());
        let view = ShellView::alloc(mtm).set_ivars(ViewIvars {
            context,
            blocked: Cell::new(false),
            configured: Cell::new(false),
            frame_name: format!("mixin_messenger_nsh_{namespace}"),
        });
        // SAFETY: NSView's designated initializer, with the expected CGRect layout.
        let view: Retained<ShellView> =
            unsafe { msg_send![super(view), initWithFrame: NSRect::ZERO] };
        // SAFETY: the notification is delivered synchronously on AppKit's thread,
        // and the retained view implements the selector. Drop removes this observer.
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &view,
                sel!(sheetWillBegin:),
                Some(NSWindowWillBeginSheetNotification),
                None,
            );
        }
        let urls = UrlCapture::new(super::events().0.clone())
            .expect("Native components run on AppKit's main thread");
        Self { view, _urls: urls }
    }
}
impl NativeComponent for Host {
    fn view(&self) -> Retained<NSView> {
        self.view.clone().into_super()
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        if name == "blocked" {
            let blocked = value.and_then(Value::as_bool).unwrap_or(false);
            let previous = self.view.ivars().blocked.replace(blocked);
            if blocked && !previous {
                cancel_sheet(&self.view);
            }
        }
        if name == "badge" {
            let count = value.and_then(Value::as_i64).unwrap_or(0).max(0);
            set_badge(count);
        }
    }
    fn command(&mut self, name: &str, _args: &Value) {
        if name == "cancel-sheet" && self.view.ivars().blocked.get() {
            cancel_sheet(&self.view);
        }
        if let Some(window) = self.view.window() {
            match name {
                "hide" => window.orderOut(None),
                "show" => {
                    if window.isMiniaturized() {
                        window.deminiaturize(None);
                    }
                    window.makeKeyAndOrderFront(None);
                    // SAFETY: activating the already-running application is main-thread confined.
                    #[allow(deprecated)]
                    NSApplication::sharedApplication(
                        MainThreadMarker::new().expect("AppKit thread"),
                    )
                    .activateIgnoringOtherApps(true);
                }
                _ => {}
            }
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        // SAFETY: this is the exact retained notification observer registered above.
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(&self.view);
        }
        cancel_sheet(&self.view);
        set_badge(0);
    }
}

fn cancel_sheet(view: &ShellView) {
    if let Some(window) = view.window()
        && let Some(sheet) = window.attachedSheet()
    {
        window.endSheet_returnCode(&sheet, NSModalResponseCancel);
    }
}

fn set_badge(count: i64) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let label = (count > 0).then(|| NSString::from_str(&count.to_string()));
    // SAFETY: NSApplication.dockTile returns NSDockTile; its documented setter
    // accepts a nullable NSString. These APIs exist on all supported macOS versions.
    unsafe {
        let tile: Retained<AnyObject> = msg_send![&NSApplication::sharedApplication(mtm), dockTile];
        let _: () = msg_send![&tile, setBadgeLabel: label.as_deref()];
    }
}

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinShell",
        Rc::new(|context| Box::new(Host::new(context))),
    );
}
