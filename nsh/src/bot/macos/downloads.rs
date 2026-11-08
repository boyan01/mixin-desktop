//! WebKit owns transport; selected files are replaced only after a complete download.
use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use block2::DynBlock;
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_foundation::{NSString, NSURL, NSUUID};

use super::dialogs::Dialogs;
use nsh_platform_macos::objc2_app_kit::NSWindow;

struct Download {
    native: Retained<AnyObject>,
    destination: Option<(PathBuf, PathBuf)>,
}

#[derive(Default)]
pub(super) struct Downloads {
    active: Rc<RefCell<HashMap<usize, Download>>>,
}

fn id(download: &AnyObject) -> usize {
    download as *const AnyObject as usize
}

fn path(url: &NSURL) -> Option<PathBuf> {
    url.absoluteString()
        .and_then(|url| url::Url::parse(&url.to_string()).ok())
        .and_then(|url| url.to_file_path().ok())
}

impl Downloads {
    pub(super) fn begin(&self, native: &AnyObject, delegate: &AnyObject) {
        let native = unsafe {
            Retained::retain(native as *const AnyObject as *mut AnyObject)
                .expect("download is alive")
        };
        if self.active.borrow().len() >= 16 {
            unsafe {
                let _: () = msg_send![&native, cancel: std::ptr::null::<AnyObject>()];
            }
            return;
        }
        self.active.borrow_mut().insert(
            id(&native),
            Download {
                native: native.clone(),
                destination: None,
            },
        );
        unsafe {
            let _: () = msg_send![&native, setDelegate: delegate];
        }
    }

    pub(super) fn destination(
        &self,
        native: &AnyObject,
        window: Option<&NSWindow>,
        dialogs: &Dialogs,
        suggested: &NSString,
        done: &DynBlock<dyn Fn(*mut NSURL)>,
    ) {
        let key = id(native);
        if !self.active.borrow().contains_key(&key) {
            done.call((std::ptr::null_mut(),));
            return;
        }
        let active = Rc::downgrade(&self.active);
        let done = done.copy();
        dialogs.save(window, &suggested.to_string(), move |url| {
            let Some(active) = active.upgrade() else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            let Some(url) = url else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            let Some(destination) = path(&url) else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            let Some(directory) = destination.parent() else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            let temporary = directory.join(format!(
                ".mixin-download-{}.part",
                NSUUID::UUID().UUIDString()
            ));
            let Ok(temporary_url) = url::Url::from_file_path(&temporary) else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            let Some(temporary_url) =
                NSURL::URLWithString(&NSString::from_str(temporary_url.as_str()))
            else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            let mut active = active.borrow_mut();
            let Some(download) = active.get_mut(&key) else {
                done.call((std::ptr::null_mut(),));
                return;
            };
            download.destination = Some((temporary, destination));
            drop(active);
            done.call((Retained::as_ptr(&temporary_url).cast_mut(),));
        });
    }

    pub(super) fn finish(&self, native: &AnyObject) -> Result<(), ()> {
        let Some(download) = self.active.borrow_mut().remove(&id(native)) else {
            return Ok(());
        };
        unsafe {
            let _: () = msg_send![&download.native, setDelegate: std::ptr::null::<AnyObject>()];
        }
        let Some((temporary, destination)) = download.destination else {
            return Err(());
        };
        if std::fs::rename(&temporary, &destination).is_err() {
            let _ = std::fs::remove_file(temporary);
            return Err(());
        }
        Ok(())
    }

    pub(super) fn failed(&self, native: &AnyObject) {
        if let Some(download) = self.active.borrow_mut().remove(&id(native)) {
            if let Some((temporary, _)) = download.destination {
                let _ = std::fs::remove_file(temporary);
            }
            unsafe {
                let _: () = msg_send![&download.native, setDelegate: std::ptr::null::<AnyObject>()];
            }
        }
    }

    pub(super) fn cancel_all(&self) {
        let active = std::mem::take(&mut *self.active.borrow_mut());
        for (_, download) in active {
            unsafe {
                let _: () = msg_send![&download.native, setDelegate: std::ptr::null::<AnyObject>()];
                let _: () = msg_send![&download.native, cancel: std::ptr::null::<AnyObject>()];
            }
            if let Some((temporary, _)) = download.destination {
                let _ = std::fs::remove_file(temporary);
            }
        }
    }
}
