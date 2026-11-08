//! An app-owned WebKit host. Native frame provenance never comes from page JSON.
use std::{cell::RefCell, ffi::CStr, rc::Rc};

use nsh_platform_macos::{
    ComponentContext, Components, NativeComponent,
    objc2_app_kit::{
        NSAccessibility, NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua,
        NSAppearanceNameDarkAqua, NSApplication, NSAutoresizingMaskOptions as Mask, NSButton,
        NSFont, NSFontWeightMedium, NSImageView, NSLineBreakMode, NSTextField, NSView,
        NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility,
    },
};
use objc2::{
    MainThreadMarker, MainThreadOnly, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject},
    sel,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL};
use serde_json::Value;

mod chrome;
mod delegate;
mod dialogs;
mod downloads;
mod header;
use delegate::Delegate;
mod resources;
mod session;
mod window;

// Link the system framework before querying its Objective-C classes.
#[link(name = "WebKit", kind = "framework")]
unsafe extern "C" {}

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinBotSession",
        Rc::new(|context| Box::new(session::Session::new(context))),
    );
    components.register(
        "MixinBotWindow",
        Rc::new(|context| Box::new(Host::new(context))),
    );
}

fn class(name: &CStr) -> Result<&'static AnyClass, &'static str> {
    AnyClass::get(name).ok_or("WebKit runtime is unavailable")
}

struct Host {
    holder: Retained<NSView>,
    resources: Rc<RefCell<resources::Resources>>,
    delegate: Retained<Delegate>,
    focus: u64,
    dark: bool,
    session: String,
    pending_page: Option<Value>,
}

impl Host {
    fn new(context: ComponentContext) -> Self {
        let mtm = MainThreadMarker::new().expect("Native factories run on AppKit's main thread");
        let delegate = Delegate::new(mtm, context);
        Self {
            holder: NSView::initWithFrame(NSView::alloc(mtm), NSRect::ZERO),
            resources: Rc::new(RefCell::new(resources::Resources {
                window: None,
                web: None,
                delegate: objc2::rc::Weak::new(&delegate),
            })),
            delegate,
            focus: 0,
            dark: false,
            session: String::new(),
            pending_page: None,
        }
    }

    fn open(&mut self, page: &Value) -> Result<(), &'static str> {
        let mtm = MainThreadMarker::new().expect("Native props run on AppKit's main thread");
        let raw = page
            .get("url")
            .and_then(Value::as_str)
            .ok_or("Missing page URL")?;
        let parsed = url::Url::parse(raw).map_err(|_| "Invalid page URL")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("Unsupported page URL");
        }
        let store = session::store(&self.session).ok_or("Bot website session is not ready")?;
        let url = NSURL::URLWithString(&NSString::from_str(raw)).ok_or("Invalid page URL")?;
        let title = page
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let is_bot = page.get("app_id").and_then(Value::as_str).is_some();
        let immersive = page
            .get("immersive")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let labels = page.get("labels").unwrap_or(&Value::Null);
        self.delegate.set_labels(labels);
        // These dimensions reproduce Flutter's external native window geometry.
        let size = NSSize::new(380.0, 750.0);
        let header_height = if immersive { 28.0 } else { 48.0 };
        let window = window::create(
            mtm,
            NSRect::new(NSPoint::ZERO, size),
            NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::FullSizeContentView,
            &self.delegate,
        );
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(&NSString::from_str(title));
        window.setAppearance(
            NSAppearance::appearanceNamed(unsafe {
                if self.dark {
                    NSAppearanceNameDarkAqua
                } else {
                    NSAppearanceNameAqua
                }
            })
            .as_deref(),
        );
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        window.setTitlebarAppearsTransparent(true);
        window.setContentMinSize(size);
        window.setMovableByWindowBackground(true);
        for kind in [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ] {
            if let Some(button) = window.standardWindowButton(kind) {
                button.setHidden(true);
            }
        }
        let root = NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::ZERO, size));
        let config: Retained<AnyObject> =
            unsafe { msg_send![class(c"WKWebViewConfiguration")?, new] };
        unsafe {
            let _: () = msg_send![&config, setWebsiteDataStore: &*store];
        }
        let content: Retained<AnyObject> = unsafe { msg_send![&config, userContentController] };
        unsafe {
            let _: () = msg_send![&content, addScriptMessageHandler: &*self.delegate, name: &*NSString::from_str("mixinBot")];
            let _: () = msg_send![&config, setApplicationNameForUserAgent: &*NSString::from_str(&format!("Mixin/{}", page.get("version").and_then(Value::as_str).unwrap_or_default()))];
        }
        let web: Retained<AnyObject> = unsafe {
            let allocation: Allocated<AnyObject> = msg_send![class(c"WKWebView")?, alloc];
            msg_send![allocation, initWithFrame: NSRect::new(NSPoint::ZERO, NSSize::new(size.width, size.height - header_height)), configuration: &*config]
        };
        unsafe {
            let _: () = msg_send![&web, setNavigationDelegate: &*self.delegate];
            let _: () = msg_send![&web, setUIDelegate: &*self.delegate];
        }
        let web_view: &NSView = unsafe { &*(Retained::as_ptr(&web).cast::<NSView>()) };
        web_view.setAutoresizingMask(Mask::ViewWidthSizable | Mask::ViewHeightSizable);
        root.addSubview(web_view);
        let header = header::create(
            mtm,
            NSRect::new(
                NSPoint::new(0.0, size.height - header_height),
                NSSize::new(size.width, header_height),
            ),
        );
        header.setAutoresizingMask(Mask::ViewWidthSizable | Mask::ViewMinYMargin);
        let mut buttons = Vec::new();
        let mut title_label = None;
        let mut icon_view = None;
        if !immersive {
            let back = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str("‹"),
                    Some(&*self.delegate),
                    Some(sel!(back:)),
                    mtm,
                )
            };
            back.setFrame(NSRect::new(
                NSPoint::new(12.0, 12.0),
                NSSize::new(24.0, 24.0),
            ));
            back.setBordered(false);
            back.setHidden(true);
            back.setAccessibilityLabel(Some(&NSString::from_str(label(labels, "Back"))));
            header.addSubview(&back);
            buttons.push(back);
            let label = NSTextField::labelWithString(&NSString::from_str(title), mtm);
            label.setFrame(NSRect::new(
                NSPoint::new(if is_bot { 44.0 } else { 12.0 }, 12.0),
                NSSize::new(if is_bot { 234.0 } else { 266.0 }, 24.0),
            ));
            label.setFont(Some(&NSFont::systemFontOfSize_weight(16.0, unsafe {
                NSFontWeightMedium
            })));
            label.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
            label.setMaximumNumberOfLines(1);
            label.setAutoresizingMask(Mask::ViewWidthSizable);
            header.addSubview(&label);
            title_label = Some(label);
            if is_bot {
                let icon = NSImageView::initWithFrame(
                    NSImageView::alloc(mtm),
                    NSRect::new(NSPoint::new(12.0, 12.0), NSSize::new(24.0, 24.0)),
                );
                icon.setWantsLayer(true);
                if let Some(layer) = icon.layer() {
                    layer.setCornerRadius(12.0);
                    layer.setMasksToBounds(true);
                }
                header.addSubview(&icon);
                icon_view = Some(icon);
            }
        }
        root.addSubview(&header);
        let capsule_top = if immersive { 34.0 } else { 8.0 };
        let capsule = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(
                NSPoint::new(size.width - 94.0, size.height - capsule_top - 32.0),
                NSSize::new(88.0, 32.0),
            ),
        );
        capsule.setAutoresizingMask(Mask::ViewMinXMargin | Mask::ViewMinYMargin);
        capsule.setWantsLayer(true);
        if let Some(layer) = capsule.layer() {
            layer.setCornerRadius(16.0);
            layer.setBorderWidth(1.0);
            layer.setMasksToBounds(true);
        }
        let divider = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(43.5, 8.0), NSSize::new(1.0, 16.0)),
        );
        divider.setWantsLayer(true);
        capsule.addSubview(&divider);
        for (text, action, x, label) in [
            ("•••", sel!(more:), 0.0, label(labels, "More")),
            ("×", sel!(close:), 44.0, label(labels, "Close")),
        ] {
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(text),
                    Some(&*self.delegate),
                    Some(action),
                    mtm,
                )
            };
            button.setFrame(NSRect::new(NSPoint::new(x, 0.0), NSSize::new(44.0, 32.0)));
            button.setBordered(false);
            button.setAccessibilityLabel(Some(&NSString::from_str(label)));
            capsule.addSubview(&button);
            buttons.push(button);
        }
        root.addSubview(&capsule);
        self.delegate.attach_capsule(&capsule, &divider);
        window.setContentView(Some(&root));
        self.delegate.attach_chrome(
            &window,
            title_label.as_deref(),
            icon_view.as_deref(),
            &buttons,
            &self.session,
            page.get("app_id")
                .and_then(Value::as_str)
                .unwrap_or(parsed.origin().ascii_serialization().as_str()),
        );
        self.delegate.system_dark(self.dark);
        self.delegate
            .icon(page.get("icon_data").and_then(Value::as_str));
        self.delegate.attach(&web, &window);
        self.delegate.presentation(page)?;
        unsafe {
            let _: () = msg_send![&window, setDelegate: &*self.delegate];
            let request: Retained<AnyObject> =
                msg_send![class(c"NSURLRequest")?, requestWithURL: &*url];
            let _: Option<Retained<AnyObject>> = msg_send![&web, loadRequest: &*request];
        }
        window.center();
        window.makeKeyAndOrderFront(None);
        NSApplication::sharedApplication(mtm).activate();
        self.resources.borrow_mut().window = Some(window);
        self.resources.borrow_mut().web = Some(web);
        session::register_window(&self.session, &self.resources);
        Ok(())
    }
}

impl NativeComponent for Host {
    fn view(&self) -> Retained<NSView> {
        self.holder.clone()
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        if name == "session" {
            self.session = value.and_then(Value::as_str).unwrap_or_default().to_owned();
            if let Some(page) = self.pending_page.take() {
                self.set_prop("page", Some(&page));
            }
            return;
        }
        if name == "dark" {
            self.dark = value.and_then(Value::as_bool).unwrap_or_default();
            self.delegate.system_dark(self.dark);
            return;
        }
        if name != "page" {
            return;
        }
        let Some(page) = value else {
            return;
        };
        if self.session.is_empty() {
            self.pending_page = Some(page.clone());
            return;
        }
        if self.resources.borrow().window.is_none() {
            if let Err(error) = self.open(page) {
                self.delegate.failed(error);
            }
        } else {
            self.delegate
                .set_labels(page.get("labels").unwrap_or(&Value::Null));
            if let Err(error) = self.delegate.presentation(page) {
                self.delegate.failed(error);
            }
            self.delegate
                .icon(page.get("icon_data").and_then(Value::as_str));
            if let Some(focus) = page.get("focus").and_then(Value::as_u64)
                && focus != self.focus
            {
                self.focus = focus;
                if let Some(window) = &self.resources.borrow().window {
                    window.makeKeyAndOrderFront(None);
                }
            }
        }
    }
    fn command(&mut self, name: &str, args: &Value) {
        if name == "reply" {
            self.delegate.reply(args);
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.resources.borrow_mut().close();
    }
}

fn label<'a>(labels: &'a Value, key: &'a str) -> &'a str {
    labels.get(key).and_then(Value::as_str).unwrap_or(key)
}
