//! AppleEvent URL delivery, installed before the native run loop starts.
use super::super::Event;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSObject, NSObjectProtocol};
use std::{
    cell::RefCell,
    io,
    rc::{Rc, Weak},
};
use tokio::sync::mpsc;

const GET_URL: u32 = u32::from_be_bytes(*b"GURL");
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

struct UrlIvars {
    events: mpsc::Sender<Event>,
}

define_class!(
    // SAFETY: all Objective-C entry points run on the application's main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "MixinNshUrlHandler"]
    #[ivars = UrlIvars]
    struct UrlHandler;
    unsafe impl NSObjectProtocol for UrlHandler {}
    impl UrlHandler {
        #[unsafe(method(handleGetUrl:withReplyEvent:))]
        fn receive(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            // SAFETY: NSAppleEventDescriptor's documented keyword getter takes AEKeyword (u32).
            let direct: Option<Retained<NSAppleEventDescriptor>> = unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
            let Some(url) = direct.and_then(|value| value.stringValue()).map(|value| value.to_string()) else { return };
            if super::super::accepted_url(&url) && self.ivars().events.try_send(Event::Open(url)).is_err() {
                log::warn!("The native application link queue is full");
            }
        }
    }
);

struct CaptureInner {
    manager: Retained<NSAppleEventManager>,
    _handler: Retained<UrlHandler>,
}
impl Drop for CaptureInner {
    fn drop(&mut self) {
        // SAFETY: the manager's removal method takes the same AE class/id registered below.
        unsafe {
            let _: () = msg_send![&self.manager, removeEventHandlerForEventClass: GET_URL, andEventID: GET_URL];
        }
    }
}
thread_local! { static CAPTURE: RefCell<Weak<CaptureInner>> = const { RefCell::new(Weak::new()) }; }

pub(in crate::shell) struct UrlCapture {
    _inner: Rc<CaptureInner>,
}
impl UrlCapture {
    pub(in crate::shell) fn new(events: mpsc::Sender<Event>) -> io::Result<Self> {
        if let Some(inner) = CAPTURE.with(|capture| capture.borrow().upgrade()) {
            return Ok(Self { _inner: inner });
        }
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| io::Error::other("Native link capture must start on the main thread"))?;
        let allocated = UrlHandler::alloc(mtm).set_ivars(UrlIvars { events });
        // SAFETY: NSObject initialization and the exact AppleEvent handler selector signature above.
        let handler: Retained<UrlHandler> = unsafe { msg_send![super(allocated), init] };
        let manager = NSAppleEventManager::sharedAppleEventManager();
        unsafe {
            let _: () = msg_send![&manager, setEventHandler: &*handler, andSelector: sel!(handleGetUrl:withReplyEvent:), forEventClass: GET_URL, andEventID: GET_URL];
        }
        let inner = Rc::new(CaptureInner {
            manager,
            _handler: handler,
        });
        CAPTURE.with(|capture| *capture.borrow_mut() = Rc::downgrade(&inner));
        Ok(Self { _inner: inner })
    }
}
