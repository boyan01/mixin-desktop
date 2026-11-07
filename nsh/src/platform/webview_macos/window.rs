//! Bot shortcuts belong to the native window that receives them.
use std::cell::RefCell;

use nsh_platform_macos::objc2_app_kit::{
    NSBackingStoreType, NSEvent, NSEventModifierFlags, NSWindow, NSWindowStyleMask,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::{Retained, Weak},
    runtime::Bool,
};
use objc2_foundation::{NSObjectProtocol, NSRect};

use super::Delegate;

struct Ivars {
    delegate: RefCell<Option<Weak<Delegate>>>,
}

define_class!(
    // SAFETY: NSWindow subclass state is used only by AppKit's main thread.
    #[unsafe(super = NSWindow)]
    #[thread_kind = MainThreadOnly]
    #[name = "MixinBotNativeWindow"]
    #[ivars = Ivars]
    struct Window;

    // SAFETY: NSObjectProtocol adds no requirements.
    unsafe impl NSObjectProtocol for Window {}

    impl Window {
        #[unsafe(method(performKeyEquivalent:))]
        fn key_equivalent(&self, event: &NSEvent) -> Bool {
            let modifiers = event.modifierFlags();
            if modifiers.contains(NSEventModifierFlags::Command)
                && !modifiers.intersects(NSEventModifierFlags::Control | NSEventModifierFlags::Shift)
                && let Some(delegate) = self.ivars().delegate.borrow().as_ref().and_then(Weak::load)
                && let Some(key) = event.charactersIgnoringModifiers()
            {
                let key = key.to_string();
                if !modifiers.contains(NSEventModifierFlags::Option) {
                    match key.as_str() {
                        "r" => { delegate.reload(); return Bool::YES; },
                        "w" => { self.close(); return Bool::YES; },
                        "[" if delegate.navigate(false) => return Bool::YES,
                        "]" if delegate.navigate(true) => return Bool::YES,
                        _ => {},
                    }
                }
            }
            // SAFETY: forward unhandled keys to NSWindow's implementation.
            unsafe { msg_send![super(self), performKeyEquivalent: event] }
        }
    }
);

pub(super) fn create(
    mtm: MainThreadMarker,
    frame: NSRect,
    style: NSWindowStyleMask,
    delegate: &Delegate,
) -> Retained<NSWindow> {
    let allocated = Window::alloc(mtm).set_ivars(Ivars {
        delegate: RefCell::new(Some(Weak::new(delegate))),
    });
    // SAFETY: NSWindow's designated initializer initializes this subclass once.
    let window: Retained<Window> = unsafe {
        msg_send![super(allocated), initWithContentRect: frame, styleMask: style, backing: NSBackingStoreType::Buffered, defer: false]
    };
    Retained::into_super(window)
}
