//! Native save confirmation is deferred out of WebView2's event callback.
use std::rc::Rc;

use webview2_com::{DownloadStartingEventHandler, Microsoft::Web::WebView2::Win32::*, take_pwstr};
use windows::{
    Win32::{
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
        UI::Shell::{FileSaveDialog, IFileSaveDialog, SIGDN_FILESYSPATH},
    },
    core::{HSTRING, PWSTR},
};

use super::window::State;

pub(super) fn install(state: &Rc<State>, web: &ICoreWebView2_4) -> windows::core::Result<()> {
    let weak = Rc::downgrade(state);
    let starting = DownloadStartingEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };
        // Any failure or dropped deferral must remain canceled until the user
        // has chosen a destination and this window is still alive.
        unsafe {
            args.SetCancel(true)?;
        }
        let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) else {
            unsafe {
                args.SetCancel(true)?;
            }
            return Ok(());
        };
        state.downloads.borrow_mut().retain(|operation| {
            let mut status = COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS;
            (unsafe { operation.State(&mut status).is_err() })
                || status == COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS
        });
        if state.downloads.borrow().len() >= 32 || state.saving.borrow().is_some() {
            unsafe {
                args.SetCancel(true)?;
            }
            return Ok(());
        }
        state
            .downloads
            .borrow_mut()
            .push(unsafe { args.DownloadOperation()? });
        state.download_started.set(true);
        let deferral = unsafe { args.GetDeferral()? };
        let callback_args = args.clone();
        let callback_deferral = deferral.clone();
        let weak = Rc::downgrade(&state);
        if !state.native.later(move || {
            let saved = weak
                .upgrade()
                .filter(|state| !state.closed.get())
                .is_some_and(|state| choose(&state, &callback_args).is_ok());
            if !saved {
                unsafe {
                    let _ = callback_args.SetCancel(true);
                }
            }
            unsafe {
                let _ = callback_deferral.Complete();
            }
        }) {
            unsafe {
                args.SetCancel(true)?;
                deferral.Complete()?;
            }
        }
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        web.add_DownloadStarting(&starting, &mut token)?;
    }
    let live = web.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = live.remove_DownloadStarting(token);
    }));
    Ok(())
}

fn choose(
    state: &State,
    args: &ICoreWebView2DownloadStartingEventArgs,
) -> windows::core::Result<()> {
    let mut proposed = PWSTR::null();
    unsafe {
        args.ResultFilePath(&mut proposed)?;
    }
    let proposed = take_pwstr(proposed);
    let name = proposed
        .rsplit(['\\', '/'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("download");
    let dialog: IFileSaveDialog =
        unsafe { CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)? };
    unsafe {
        dialog.SetFileName(&HSTRING::from(name))?;
        if let Some((_, extension)) = name.rsplit_once('.') {
            dialog.SetDefaultExtension(&HSTRING::from(extension))?;
        }
        if state.saving.borrow().is_some() {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_ABORT,
            ));
        }
        *state.saving.borrow_mut() = Some(dialog.clone());
        let shown = dialog.Show(Some(state.native.hwnd.get()));
        state.saving.borrow_mut().take();
        shown?;
        if state.closed.get() {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_ABORT,
            ));
        }
        let result = dialog.GetResult()?;
        let path = result.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = take_pwstr(path);
        args.SetResultFilePath(&HSTRING::from(path))?;
        args.SetCancel(false)?;
        // WebView2 keeps its native download progress and failure UI.
        args.SetHandled(false)?;
    }
    Ok(())
}
