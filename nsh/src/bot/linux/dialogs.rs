//! Native dialogs retain each WebKit request until a response or owner closure.
use super::window::State;
use gtk::{gio, prelude::*};
use nsh_platform_linux::gtk4 as gtk;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use webkit6::{FileChooserRequest, ScriptDialog, ScriptDialogType};

type Cleanup = Box<dyn FnOnce()>;
#[derive(Default)]
pub(super) struct Dialogs {
    next: Cell<u64>,
    pending: Rc<RefCell<HashMap<u64, Cleanup>>>,
}

impl Dialogs {
    fn insert(&self, cleanup: Cleanup) -> u64 {
        let id = self.next.get().wrapping_add(1);
        self.next.set(id);
        self.pending.borrow_mut().insert(id, cleanup);
        id
    }
    pub fn close(&self) {
        let pending = self
            .pending
            .borrow_mut()
            .drain()
            .map(|(_, cleanup)| cleanup)
            .collect::<Vec<_>>();
        for cleanup in pending {
            cleanup();
        }
    }
    pub fn script(&self, owner: &Rc<State>, request: &ScriptDialog) {
        if self.pending.borrow().len() >= 16 {
            request.close();
            return;
        }
        let kind = request.dialog_type();
        let dialog = gtk::Window::builder()
            .transient_for(&owner.window)
            .modal(true)
            .title(owner.label("Webpage"))
            .build();
        let area = gtk::Box::new(gtk::Orientation::Vertical, 8);
        area.set_margin_top(16);
        area.set_margin_bottom(16);
        area.set_margin_start(16);
        area.set_margin_end(16);
        let label = gtk::Label::new(request.message().as_deref());
        label.set_wrap(true);
        label.set_selectable(true);
        label.set_max_width_chars(60);
        area.append(&label);
        let entry = gtk::Entry::new();
        entry.set_activates_default(true);
        if kind == ScriptDialogType::Prompt {
            entry.set_text(
                request
                    .prompt_get_default_text()
                    .as_deref()
                    .unwrap_or_default(),
            );
            area.append(&entry);
        }
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let request = request.clone();
        let cancel = request.clone();
        let window = dialog.clone();
        let id = self.insert(Box::new(move || {
            cancel.close();
            window.destroy();
        }));
        for accepted in [false, true] {
            if !accepted && kind == ScriptDialogType::Alert {
                continue;
            }
            let button =
                gtk::Button::with_label(&owner.label(if accepted { "OK" } else { "Cancel" }));
            let pending = Rc::downgrade(&self.pending);
            let request = request.clone();
            let entry = entry.clone();
            if accepted {
                dialog.set_default_widget(Some(&button));
            }
            let dialog = dialog.downgrade();
            button.connect_clicked(move |_| {
                let Some(pending) = pending.upgrade() else {
                    return;
                };
                if pending.borrow_mut().remove(&id).is_none() {
                    return;
                }
                match kind {
                    ScriptDialogType::Prompt if accepted => request.prompt_set_text(&entry.text()),
                    ScriptDialogType::Confirm | ScriptDialogType::BeforeUnloadConfirm => {
                        request.confirm_set_confirmed(accepted)
                    }
                    _ => {}
                }
                request.close();
                if let Some(dialog) = dialog.upgrade() {
                    dialog.destroy();
                }
            });
            actions.append(&button);
        }
        area.append(&actions);
        dialog.set_child(Some(&area));
        let pending = Rc::downgrade(&self.pending);
        dialog.connect_close_request(move |_| {
            if let Some(pending) = pending.upgrade()
                && let Some(cleanup) = pending.borrow_mut().remove(&id)
            {
                cleanup();
            }
            gtk::glib::Propagation::Stop
        });
        dialog.present();
    }
    pub fn files(&self, owner: &Rc<State>, request: &FileChooserRequest) {
        if self.pending.borrow().len() >= 16 {
            request.cancel();
            return;
        }
        let dialog = gtk::FileDialog::builder()
            .title(owner.label("Webpage"))
            .accept_label(owner.label("OK"))
            .modal(true)
            .build();
        if let Some(filter) = request.mime_types_filter() {
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            dialog.set_default_filter(Some(&filter));
        }
        let request = request.clone();
        let cancel = request.clone();
        let cancellation = gio::Cancellable::new();
        let stop = cancellation.clone();
        let id = self.insert(Box::new(move || {
            stop.cancel();
            cancel.cancel();
        }));
        let pending = Rc::downgrade(&self.pending);
        let multiple = request.selects_multiple();
        let complete = move |result: Result<Vec<gio::File>, gtk::glib::Error>| {
            let Some(pending) = pending.upgrade() else {
                request.cancel();
                return;
            };
            if pending.borrow_mut().remove(&id).is_none() {
                return;
            }
            let paths = result
                .unwrap_or_default()
                .iter()
                .filter_map(|file| file.path())
                .filter_map(|path| path.to_str().map(str::to_owned))
                .collect::<Vec<_>>();
            if paths.is_empty() {
                request.cancel();
            } else {
                request.select_files(&paths.iter().map(String::as_str).collect::<Vec<_>>());
            }
        };
        if multiple {
            dialog.open_multiple(Some(&owner.window), Some(&cancellation), move |result| {
                complete(
                    result.map(|files| files.iter::<gio::File>().filter_map(Result::ok).collect()),
                )
            });
        } else {
            dialog.open(Some(&owner.window), Some(&cancellation), move |result| {
                complete(result.map(|file| vec![file]))
            });
        }
    }
    pub fn error(&self, owner: &Rc<State>) {
        if self.pending.borrow().len() >= 16 {
            return;
        }
        let dialog = gtk::Window::builder()
            .transient_for(&owner.window)
            .modal(true)
            .title(owner.label("Webpage"))
            .build();
        let area = gtk::Box::new(gtk::Orientation::Vertical, 16);
        area.set_margin_top(16);
        area.set_margin_bottom(16);
        area.set_margin_start(16);
        area.set_margin_end(16);
        let label = gtk::Label::new(Some(
            &owner.label("Couldn’t download this file. Try again."),
        ));
        label.set_wrap(true);
        area.append(&label);
        let button = gtk::Button::with_label(&owner.label("OK"));
        area.append(&button);
        dialog.set_child(Some(&area));
        let window = dialog.clone();
        let id = self.insert(Box::new(move || window.destroy()));
        let pending = Rc::downgrade(&self.pending);
        button.connect_clicked(move |_| {
            if let Some(pending) = pending.upgrade() {
                let cleanup = pending.borrow_mut().remove(&id);
                if let Some(cleanup) = cleanup {
                    cleanup();
                }
            }
        });
        let pending = Rc::downgrade(&self.pending);
        dialog.connect_close_request(move |_| {
            if let Some(pending) = pending.upgrade() {
                let cleanup = pending.borrow_mut().remove(&id);
                if let Some(cleanup) = cleanup {
                    cleanup();
                }
            }
            gtk::glib::Propagation::Stop
        });
        dialog.present();
    }
}
