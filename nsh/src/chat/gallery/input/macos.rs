//! A transparent, canvas-sized AppKit input view. Events stay in this view's
//! window and are delivered through NSH's engine queue, never to UI signals here.
use std::{cell::Cell, rc::Rc};

use nsh_platform_macos::{
    ComponentContext, Components, NativeComponent,
    objc2_app_kit::{NSEvent, NSView},
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::Bool,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect};
use serde_json::{Value, json};

struct State {
    context: ComponentContext,
    enabled: Cell<bool>,
    drag: Cell<Option<NSPoint>>,
}

define_class!(
    // SAFETY: AppKit creates, uses and disposes the view on the main thread.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[name = "MixinImageInputView"]
    #[ivars = State]
    struct InputView;
    unsafe impl NSObjectProtocol for InputView {}
    impl InputView {
        #[unsafe(method(isFlipped))]
        fn flipped(&self) -> Bool { Bool::YES }

        #[unsafe(method(mouseDownCanMoveWindow))]
        fn movable(&self) -> Bool { Bool::NO }

        #[unsafe(method(acceptsFirstMouse:))]
        fn first_mouse(&self, _event: Option<&NSEvent>) -> Bool { Bool::YES }

        #[unsafe(method(mouseDown:))]
        fn down(&self, event: &NSEvent) {
            if !self.ivars().enabled.get() { return; }
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            if event.clickCount() == 2 {
                self.ivars().drag.set(None);
                self.ivars().context.emit("toggle", json!({"x":point.x,"y":point.y}));
            } else {
                self.ivars().drag.set(Some(point));
            }
        }

        #[unsafe(method(mouseDragged:))]
        fn dragged(&self, event: &NSEvent) {
            if !self.ivars().enabled.get() { return; }
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            if let Some(previous) = self.ivars().drag.replace(Some(point)) {
                self.ivars().context.emit("pan", json!({"x":point.x-previous.x,"y":point.y-previous.y}));
            }
        }

        #[unsafe(method(mouseUp:))]
        fn up(&self, _event: &NSEvent) { self.ivars().drag.set(None); }

        #[unsafe(method(scrollWheel:))]
        fn wheel(&self, event: &NSEvent) {
            if !self.ivars().enabled.get() { return; }
            if event.hasPreciseScrollingDeltas() && (!event.phase().is_empty() || !event.momentumPhase().is_empty()) {
                self.ivars().context.emit("pan", json!({"x":event.scrollingDeltaX(),"y":event.scrollingDeltaY()}));
            } else {
                let delta = event.scrollingDeltaY();
                // Mouse wheels report lines, not the trackpad's point deltas.
                let sensitivity = if event.hasPreciseScrollingDeltas() { 0.004 } else { 0.12 };
                self.zoom_at_pointer((delta * sensitivity).clamp(-2.0, 2.0).exp(), event);
            }
        }

        #[unsafe(method(magnifyWithEvent:))]
        fn magnify(&self, event: &NSEvent) {
            if self.ivars().enabled.get() {
                self.zoom_at_pointer((1.0 + event.magnification()).max(0.01), event);
            }
        }
    }
);

impl InputView {
    fn zoom_at_pointer(&self, factor: f64, event: &NSEvent) {
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        let bounds = self.bounds();
        if point.x >= 0.0
            && point.y >= 0.0
            && point.x <= bounds.size.width
            && point.y <= bounds.size.height
        {
            self.ivars()
                .context
                .emit("zoom", json!({"factor":factor,"x":point.x,"y":point.y}));
        }
    }
}

struct Host {
    view: Retained<InputView>,
}

impl NativeComponent for Host {
    fn view(&self) -> Retained<NSView> {
        self.view.clone().into_super()
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        if name == "enabled" {
            self.view
                .ivars()
                .enabled
                .set(value.and_then(Value::as_bool).unwrap_or(false));
            self.view.ivars().drag.set(None);
        }
    }
}

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinImageInput",
        Rc::new(|context| {
            let mtm = MainThreadMarker::new().expect("Native views run on AppKit's thread");
            let allocated = InputView::alloc(mtm).set_ivars(State {
                context,
                enabled: Cell::new(false),
                drag: Cell::new(None),
            });
            // SAFETY: the superclass designated initializer initializes this NSView subclass.
            let view = unsafe { msg_send![super(allocated), initWithFrame: NSRect::ZERO] };
            Box::new(Host { view })
        }),
    );
}
