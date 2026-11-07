//! Downloads ask for a native destination and are canceled with their window.
use super::window::State;
use gtk::gio;
use nsh_platform_linux::gtk4 as gtk;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use webkit6::{Download, prelude::*};

#[derive(Default)]
pub(super) struct Downloads {
    next: Cell<u64>,
    active: Rc<RefCell<HashMap<u64, Download>>>,
    choosers: Rc<RefCell<HashMap<u64, gio::Cancellable>>>,
}

impl Downloads {
    pub fn close(&self) {
        let choosers = self
            .choosers
            .borrow_mut()
            .drain()
            .map(|(_, chooser)| chooser)
            .collect::<Vec<_>>();
        for chooser in choosers {
            chooser.cancel();
        }
        let downloads = self
            .active
            .borrow_mut()
            .drain()
            .map(|(_, download)| download)
            .collect::<Vec<_>>();
        for download in downloads {
            download.cancel();
        }
    }
    pub fn start(&self, owner: &Rc<State>, download: &Download) {
        if self.active.borrow().len() >= 16 {
            download.cancel();
            return;
        }
        let id = self.next.get().wrapping_add(1);
        self.next.set(id);
        self.active.borrow_mut().insert(id, download.clone());
        download.set_allow_overwrite(false);
        let weak = Rc::downgrade(owner);
        let active = Rc::downgrade(&self.active);
        let choosers = Rc::downgrade(&self.choosers);
        download.connect_decide_destination(move |download, suggested| {
            let (Some(owner), Some(active), Some(choosers)) = (
                weak.upgrade().filter(|s| !s.closed.get()),
                active.upgrade(),
                choosers.upgrade(),
            ) else {
                download.cancel();
                return true;
            };
            if !active.borrow().contains_key(&id) {
                download.cancel();
                return true;
            }
            let chooser = gtk::FileDialog::builder()
                .accept_label(owner.label("OK"))
                .modal(true)
                .build();
            let name = std::path::Path::new(suggested)
                .file_name()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty() && !matches!(*s, "." | ".."))
                .unwrap_or("download");
            chooser.set_initial_name(Some(name));
            let cancellation = gio::Cancellable::new();
            choosers.borrow_mut().insert(id, cancellation.clone());
            let download = download.clone();
            let pending = Rc::downgrade(&choosers);
            let active = Rc::downgrade(&active);
            chooser.save(Some(&owner.window), Some(&cancellation), move |result| {
                let Some(pending) = pending.upgrade() else {
                    download.cancel();
                    return;
                };
                if pending.borrow_mut().remove(&id).is_none() {
                    return;
                }
                let valid = active
                    .upgrade()
                    .is_some_and(|active| active.borrow().contains_key(&id));
                if valid
                    && let Ok(file) = result
                    && file.path().is_some()
                {
                    // The native save chooser has confirmed the selected target.
                    download.set_allow_overwrite(true);
                    download.set_destination(&file.uri());
                } else {
                    download.cancel();
                }
            });
            true
        });
        let weak = Rc::downgrade(owner);
        download.connect_failed(move |_, error| {
            if !error.matches(webkit6::DownloadError::CancelledByUser)
                && let Some(owner) = weak.upgrade().filter(|s| !s.closed.get())
            {
                owner.dialogs.error(&owner);
            }
        });
        let active = Rc::downgrade(&self.active);
        let choosers = Rc::downgrade(&self.choosers);
        download.connect_finished(move |_| {
            if let Some(active) = active.upgrade() {
                active.borrow_mut().remove(&id);
            }
            if let Some(choosers) = choosers.upgrade()
                && let Some(chooser) = choosers.borrow_mut().remove(&id)
            {
                chooser.cancel();
            }
        });
    }
}
