//! Header controls and browser content form one native keyboard focus cycle.
use std::rc::{Rc, Weak};

use webview2_com::{Microsoft::Web::WebView2::Win32::*, MoveFocusRequestedEventHandler};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
        UI::{
            Accessibility::{CAccPropServices, IAccPropServices, PROPID_ACC_NAME},
            Input::KeyboardAndMouse::{GetKeyState, IsWindowEnabled, SetFocus, VK_SHIFT},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, Result},
};

use super::window::State;

pub(super) fn button(state: &Rc<State>, window: HWND) -> Result<()> {
    let raw = Box::into_raw(Box::new(Rc::downgrade(state)));
    if !unsafe { SetWindowSubclass(window, Some(procedure), 1, raw as usize) }.as_bool() {
        drop(unsafe { Box::from_raw(raw) });
        return Err(windows::core::Error::from_thread());
    }
    Ok(())
}

pub(super) fn name(window: HWND, name: &str) {
    // Standard Win32 button captions are glyphs; expose the actual action name.
    let service: Result<IAccPropServices> =
        unsafe { CoCreateInstance(&CAccPropServices, None, CLSCTX_INPROC_SERVER) };
    if let Ok(service) = service {
        unsafe {
            let _ = service.SetHwndPropStr(
                window,
                OBJID_CLIENT.0 as u32,
                0,
                PROPID_ACC_NAME,
                &HSTRING::from(name),
            );
        }
    }
}

pub(super) fn browser(state: &Rc<State>, controller: &ICoreWebView2Controller) -> Result<()> {
    let weak = Rc::downgrade(state);
    let moving = MoveFocusRequestedEventHandler::create(Box::new(move |_, args| {
        let (Some(state), Some(args)) = (weak.upgrade().filter(|state| !state.closed.get()), args)
        else {
            return Ok(());
        };
        let mut reason = COREWEBVIEW2_MOVE_FOCUS_REASON_NEXT;
        unsafe {
            args.Reason(&mut reason)?;
        }
        let buttons = super::chrome::buttons(&state);
        let target = if reason == COREWEBVIEW2_MOVE_FOCUS_REASON_PREVIOUS {
            buttons.into_iter().rev().find(|button| {
                unsafe { IsWindowEnabled(*button) }.as_bool()
                    && unsafe { IsWindowVisible(*button) }.as_bool()
            })
        } else {
            buttons.into_iter().find(|button| {
                unsafe { IsWindowEnabled(*button) }.as_bool()
                    && unsafe { IsWindowVisible(*button) }.as_bool()
            })
        };
        if let Some(target) = target {
            unsafe {
                let _ = SetFocus(Some(target));
                args.SetHandled(true)?;
            }
        }
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        controller.add_MoveFocusRequested(&moving, &mut token)?;
    }
    let controller = controller.clone();
    state.removers.borrow_mut().push(Box::new(move || unsafe {
        let _ = controller.remove_MoveFocusRequested(token);
    }));
    Ok(())
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    let weak = unsafe { &*(data as *const Weak<State>) };
    if message == WM_NCDESTROY {
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(procedure), id);
        }
        drop(unsafe { Box::from_raw(data as *mut Weak<State>) });
    } else if let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) {
        if message == WM_KEYDOWN && wparam.0 == 9 {
            let backwards = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
            let buttons = super::chrome::buttons(&state);
            if let Some(index) = buttons.iter().position(|button| *button == hwnd) {
                let target = if backwards {
                    buttons[..index].iter().rev().find(|button| {
                        unsafe { IsWindowEnabled(**button) }.as_bool()
                            && unsafe { IsWindowVisible(**button) }.as_bool()
                    })
                } else {
                    buttons[index + 1..].iter().find(|button| {
                        unsafe { IsWindowEnabled(**button) }.as_bool()
                            && unsafe { IsWindowVisible(**button) }.as_bool()
                    })
                };
                if let Some(target) = target {
                    unsafe {
                        let _ = SetFocus(Some(*target));
                    }
                } else if let Some(controller) = state.controller.borrow().as_ref() {
                    unsafe {
                        let _ = controller.MoveFocus(if backwards {
                            COREWEBVIEW2_MOVE_FOCUS_REASON_PREVIOUS
                        } else {
                            COREWEBVIEW2_MOVE_FOCUS_REASON_NEXT
                        });
                    }
                }
            }
            return LRESULT(0);
        }
        if message == WM_CHAR && wparam.0 == 9 {
            return LRESULT(0);
        }
        if message == WM_KEYDOWN && wparam.0 == 13 {
            let id = unsafe { GetDlgCtrlID(hwnd) };
            unsafe {
                SendMessageW(
                    state.native.hwnd.get(),
                    WM_COMMAND,
                    Some(WPARAM(id as usize)),
                    Some(LPARAM(hwnd.0 as isize)),
                );
            }
            return LRESULT(0);
        }
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}
