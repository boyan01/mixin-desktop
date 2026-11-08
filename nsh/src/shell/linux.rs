//! GTK shell attached to this native component's actual containing window.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use gtk::{glib, prelude::*};
use nsh_platform_linux::{ComponentContext, Components, NativeComponent, gtk4 as gtk};
use serde_json::{Value, json};

struct Binding {
    window: glib::WeakRef<gtk::Window>,
    handlers: Vec<glib::SignalHandlerId>,
}

impl Drop for Binding {
    fn drop(&mut self) {
        if let Some(window) = self.window.upgrade() {
            for handler in self.handlers.drain(..) {
                window.disconnect(handler);
            }
        }
    }
}

struct State {
    context: ComponentContext,
    binding: RefCell<Option<Binding>>,
    initial: RefCell<Option<Value>>,
    loaded: Cell<bool>,
    restored: Cell<bool>,
    restoring: Cell<bool>,
    quitting: Cell<bool>,
    last: RefCell<Option<Value>>,
    pending: RefCell<Option<glib::SourceId>>,
}

impl State {
    fn window(&self) -> Option<gtk::Window> {
        self.binding.borrow().as_ref()?.window.upgrade()
    }

    fn attach(self: &Rc<Self>, anchor: &gtk::Box) {
        let window = anchor.root().and_downcast::<gtk::Window>();
        if self.window() == window {
            return;
        }
        self.cancel_pending();
        self.binding.borrow_mut().take();
        let Some(window) = window else {
            return;
        };
        window.set_size_request(384, 480);
        let mut handlers = Vec::new();
        let weak = Rc::downgrade(self);
        handlers.push(window.connect_default_width_notify(move |_| {
            if let Some(state) = weak.upgrade() {
                state.schedule_bounds();
            }
        }));
        let weak = Rc::downgrade(self);
        handlers.push(window.connect_default_height_notify(move |_| {
            if let Some(state) = weak.upgrade() {
                state.schedule_bounds();
            }
        }));
        let weak = Rc::downgrade(self);
        handlers.push(window.connect_maximized_notify(move |_| {
            if let Some(state) = weak.upgrade() {
                state.schedule_bounds();
            }
        }));
        let weak = Rc::downgrade(self);
        handlers.push(window.connect_close_request(move |_| {
            if let Some(state) = weak.upgrade() {
                state.request_quit();
            }
            // Root completes its final preference write before sending quit.
            glib::Propagation::Stop
        }));
        *self.binding.borrow_mut() = Some(Binding {
            window: window.downgrade(),
            handlers,
        });
        self.restore();
    }

    fn restore(&self) {
        if !self.loaded.get() || self.restored.get() {
            return;
        }
        let Some(window) = self.window() else {
            return;
        };
        self.restored.set(true);
        let initial = self.initial.borrow();
        let Some(value) = initial.as_ref() else {
            return;
        };
        let dimension = |key| {
            value
                .get(key)?
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .filter(|n| *n > 0)
        };
        let (Some(width), Some(height), Some(maximized)) = (
            dimension("width"),
            dimension("height"),
            value.get("maximized").and_then(Value::as_bool),
        ) else {
            return;
        };
        self.restoring.set(true);
        // GTK keeps the normal default size while maximized/fullscreen. Using
        // allocations here would overwrite it with the maximized dimensions.
        window.set_default_size(width.max(384), height.max(480));
        window.set_maximized(maximized);
        self.restoring.set(false);
    }

    fn cancel_pending(&self) {
        if let Some(source) = self.pending.borrow_mut().take() {
            source.remove();
        }
    }

    fn schedule_bounds(self: &Rc<Self>) {
        if !self.loaded.get() || self.restoring.get() {
            return;
        }
        self.cancel_pending();
        let weak = Rc::downgrade(self);
        *self.pending.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(150),
            move || {
                if let Some(state) = weak.upgrade() {
                    state.pending.borrow_mut().take();
                    state.emit_bounds();
                }
            },
        ));
    }

    fn emit_bounds(&self) {
        if !self.loaded.get() || self.restoring.get() {
            return;
        }
        let Some(window) = self.window() else {
            return;
        };
        let (width, height) = window.default_size();
        if width <= 0 || height <= 0 {
            return;
        }
        let value = json!({"width": width, "height": height, "maximized": window.is_maximized()});
        if self.last.borrow().as_ref() != Some(&value) {
            *self.last.borrow_mut() = Some(value.clone());
            self.context.emit("bounds", value);
        }
    }

    fn flush_bounds(&self) {
        self.cancel_pending();
        self.emit_bounds();
    }

    fn request_quit(&self) {
        if self.quitting.replace(true) {
            return;
        }
        self.flush_bounds();
        self.context
            .emit("quit", self.last.borrow().clone().unwrap_or(Value::Null));
    }

    fn command(&self, command: &str) {
        let Some(window) = self.window() else {
            return;
        };
        match command {
            "show" => {
                window.unminimize();
                window.present();
            }
            "hide" => {
                self.flush_bounds();
                // Linux has no tray in the reference app. Minimizing keeps a
                // taskbar entry instead of leaving a hidden, unreachable process.
                window.minimize();
            }
            "quit" => {
                self.flush_bounds();
                let application = window.application();
                // Destroy owns NSH engine cleanup. Defer past this native
                // command's host borrow before disposing its owner.
                glib::idle_add_local_once(move || {
                    window.destroy();
                    if let Some(application) = application {
                        application.quit();
                    }
                });
            }
            "request-quit" => self.request_quit(),
            _ => {}
        }
    }
}

struct Host {
    anchor: gtk::Box,
    state: Rc<State>,
    root_handler: Option<glib::SignalHandlerId>,
}

impl Host {
    fn new(context: ComponentContext) -> Self {
        let anchor = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let state = Rc::new(State {
            context,
            binding: RefCell::default(),
            initial: RefCell::default(),
            loaded: Cell::new(false),
            restored: Cell::new(false),
            restoring: Cell::new(false),
            quitting: Cell::new(false),
            last: RefCell::default(),
            pending: RefCell::default(),
        });
        let weak = Rc::downgrade(&state);
        let root_handler = anchor.connect_root_notify(move |anchor| {
            if let Some(state) = weak.upgrade() {
                state.attach(anchor);
            }
        });
        Self {
            anchor,
            state,
            root_handler: Some(root_handler),
        }
    }
}

impl NativeComponent for Host {
    fn widget(&self) -> gtk::Widget {
        self.anchor.clone().upcast()
    }

    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        self.state.attach(&self.anchor);
        if name == "bounds" && !self.state.loaded.replace(true) {
            *self.state.initial.borrow_mut() = value.filter(|v| !v.is_null()).cloned();
            self.state.restore();
        }
    }

    fn command(&mut self, name: &str, _: &Value) {
        self.state.attach(&self.anchor);
        self.state.command(name);
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if let Some(handler) = self.root_handler.take() {
            self.anchor.disconnect(handler);
        }
        self.state.cancel_pending();
        self.state.binding.borrow_mut().take();
    }
}

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinShell",
        Rc::new(|context| Box::new(Host::new(context))),
    );
}
