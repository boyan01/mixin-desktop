//! Shared native resources let the account session close every WebKit view before purging.
use super::Delegate;
use nsh_platform_macos::objc2_app_kit::NSWindow;
use objc2::{
    msg_send,
    rc::{Retained, Weak},
    runtime::AnyObject,
};

pub(super) struct Resources {
    pub window: Option<Retained<NSWindow>>,
    pub web: Option<Retained<AnyObject>>,
    pub delegate: Weak<Delegate>,
}

impl Resources {
    pub(super) fn close(&mut self) {
        if let Some(delegate) = self.delegate.load() {
            delegate.detach();
        }
        if let Some(web) = self.web.take() {
            // SAFETY: each object was created as a WKWebView in Host::open.
            unsafe {
                let _: () = msg_send![&web, stopLoading];
                let _: () = msg_send![&web, setNavigationDelegate: std::ptr::null::<AnyObject>()];
                let _: () = msg_send![&web, setUIDelegate: std::ptr::null::<AnyObject>()];
                let config: Retained<AnyObject> = msg_send![&web, configuration];
                let content: Retained<AnyObject> = msg_send![&config, userContentController];
                let _: () = msg_send![&content, removeScriptMessageHandlerForName: &*objc2_foundation::NSString::from_str("mixinBot")];
                let _: () = msg_send![&content, removeAllUserScripts];
            }
            if let Some(window) = &self.window {
                window.setContentView(None);
            }
            drop(web);
        }
        if let Some(window) = self.window.take() {
            unsafe {
                let _: () = msg_send![&window, setDelegate: std::ptr::null::<AnyObject>()];
            }
            window.close();
        }
    }
}
