//! The title row drags like Flutter's native composer; buttons retain normal input.
use nsh_platform_macos::objc2_app_kit::{NSButton, NSEvent, NSView};
use objc2::{
    ClassType, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::Bool,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSString, NSUserDefaults};

define_class!(
    // SAFETY: AppKit calls all view entry points on its main thread.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[name = "MixinBotHeaderView"]
    struct Header;
    unsafe impl NSObjectProtocol for Header {}
    impl Header {
        #[unsafe(method(mouseDownCanMoveWindow))]
        fn movable(&self) -> Bool { Bool::NO }

        #[unsafe(method(hitTest:))]
        fn hit(&self, point: NSPoint) -> *mut NSView {
            let hit: Option<Retained<NSView>> = unsafe { msg_send![super(self), hitTest: point] };
            let Some(hit) = hit else { return std::ptr::null_mut(); };
            let button: Bool = unsafe { msg_send![&hit, isKindOfClass: NSButton::class()] };
            if button.as_bool() { Retained::as_ptr(&hit).cast_mut() }
            else { self as *const Header as *mut NSView }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse(&self, event: &NSEvent) {
            let Some(window) = self.window() else { return; };
            match event.clickCount() {
                1 => window.performWindowDragWithEvent(event),
                2 => match NSUserDefaults::standardUserDefaults().stringForKey(&NSString::from_str("AppleActionOnDoubleClick")).map(|value| value.to_string()).as_deref() {
                    Some("Minimize") => window.miniaturize(None),
                    Some("None") => {},
                    _ => window.zoom(None),
                },
                _ => {},
            }
        }
    }
);

pub(super) fn create(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSView> {
    let allocated = Header::alloc(mtm).set_ivars(());
    // SAFETY: NSView's designated initializer initializes this subclass once.
    let view: Retained<Header> = unsafe { msg_send![super(allocated), initWithFrame: frame] };
    Retained::into_super(view)
}
