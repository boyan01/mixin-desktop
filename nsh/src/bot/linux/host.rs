//! The NSH anchor owns a native window without participating in its layout.
use super::{session, window::State};
use gtk::prelude::*;
use nsh_platform_linux::{ComponentContext, NativeComponent, gtk4 as gtk};
use serde_json::Value;
use std::rc::Rc;

pub(super) struct Host {
    holder: gtk::Box,
    context: ComponentContext,
    session: String,
    pending: Option<Value>,
    state: Option<Rc<State>>,
}

impl Host {
    pub fn new(context: ComponentContext) -> Self {
        Self {
            holder: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            context,
            session: String::new(),
            pending: None,
            state: None,
        }
    }
    fn page(&mut self, page: &Value) {
        if self.session.is_empty() {
            self.pending = Some(page.clone());
            return;
        }
        if let Some(state) = &self.state {
            *state.page.borrow_mut() = page.clone();
            state.presentation();
            return;
        }
        let Some(entry) = session::get(&self.session) else {
            self.context.emit("failed", Value::Null);
            return;
        };
        match State::open(self.context.clone(), page.clone(), entry) {
            Ok(state) => self.state = Some(state),
            Err(_) => self.context.emit("failed", Value::Null),
        }
    }
}

impl NativeComponent for Host {
    fn widget(&self) -> gtk::Widget {
        self.holder.clone().upcast()
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        match name {
            "session" => {
                let key = value.and_then(Value::as_str).unwrap_or_default();
                if self.session != key
                    && let Some(state) = self.state.take()
                {
                    state.close();
                }
                self.session = key.into();
                if let Some(page) = self.pending.take() {
                    self.page(&page);
                }
            }
            "page" => {
                if let Some(page) = value {
                    self.page(page);
                }
            }
            // NSH's process appearance already drives GTK and WebKit color scheme.
            "dark" => {}
            _ => {}
        }
    }
    fn command(&mut self, name: &str, args: &Value) {
        if name == "reply"
            && let Some(state) = &self.state
        {
            state.reply(args);
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            state.close();
        }
    }
}
