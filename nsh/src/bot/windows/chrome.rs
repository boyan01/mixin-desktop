//! Native keyboard-accessible browser commands; labels come from the shared UI.
use std::rc::Rc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use windows::{
    Win32::{
        Foundation::{HANDLE, HWND, LPARAM, RECT, WPARAM},
        Graphics::Dwm::{
            DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
        },
        System::{
            DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
            SystemServices::SS_ICON,
        },
        UI::{
            HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, w},
};

use super::window::{State, is_web};

const BACK: usize = 101;
const MORE: usize = 103;
const CLOSE: usize = 104;
const REFRESH: usize = 201;
const SHARE: usize = 202;
const COPY: usize = 203;
const BROWSER: usize = 204;

#[derive(Clone, Copy)]
pub(super) struct Chrome {
    back: HWND,
    more: HWND,
    close: HWND,
    title: HWND,
    icon: HWND,
}

pub(super) struct Icon {
    data: String,
    handle: HICON,
}
impl Drop for Icon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.handle);
        }
    }
}

pub(super) fn install(state: &Rc<State>) -> windows::core::Result<()> {
    let parent = state.native.hwnd.get();
    let button = |text: &str, id: usize| unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            &HSTRING::from(text),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            0,
            0,
            32,
            32,
            Some(parent),
            Some(HMENU(id as *mut _)),
            None,
            None,
        )
    };
    let chrome = Chrome {
        back: button("‹", BACK)?,
        more: button("•••", MORE)?,
        close: button("×", CLOSE)?,
        title: unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                1,
                1,
                Some(parent),
                None,
                None,
                None,
            )?
        },
        icon: unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_ICON.0),
                0,
                0,
                24,
                24,
                Some(parent),
                None,
                None,
                None,
            )?
        },
    };
    *state.chrome.borrow_mut() = Some(chrome);
    for button in buttons(state) {
        super::focus::button(state, button)?;
    }
    let weak = Rc::downgrade(state);
    *state.native.command.borrow_mut() = Some(Box::new(move |id| {
        if let Some(state) = weak.upgrade().filter(|state| !state.closed.get()) {
            command(&state, id);
        }
    }));
    update(state);
    Ok(())
}

pub(super) fn layout(state: &State, bounds: &RECT) -> i32 {
    let scale = unsafe { GetDpiForWindow(state.native.hwnd.get()) }.max(96) as f32 / 96.0;
    let immersive = state
        .page
        .borrow()
        .get("immersive")
        .and_then(Value::as_bool)
        .unwrap_or_default();
    let height = if immersive {
        0
    } else {
        (48.0 * scale).round() as i32
    };
    if let Some(chrome) = state.chrome.borrow().as_ref() {
        let width = bounds.right - bounds.left;
        let unit = |value: f32| (value * scale).round() as i32;
        for (window, left, top, width, height) in [
            (chrome.back, unit(8.0), unit(8.0), unit(28.0), unit(32.0)),
            (chrome.icon, unit(12.0), unit(12.0), unit(24.0), unit(24.0)),
            (
                chrome.title,
                unit(44.0),
                unit(14.0),
                (width - unit(140.0)).max(0),
                unit(24.0),
            ),
            (
                chrome.more,
                width - unit(88.0),
                unit(8.0),
                unit(40.0),
                unit(32.0),
            ),
            (
                chrome.close,
                width - unit(46.0),
                unit(8.0),
                unit(38.0),
                unit(32.0),
            ),
        ] {
            unsafe {
                let _ = SetWindowPos(
                    window,
                    None,
                    left,
                    top,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
    height
}

pub(super) fn update(state: &State) {
    let Some(chrome) = state.chrome.borrow().as_ref().copied() else {
        return;
    };
    let page = state.page.borrow();
    icon(state, &page, chrome);
    for (window, key) in [
        (chrome.back, "Back"),
        (chrome.more, "More"),
        (chrome.close, "Close"),
    ] {
        super::focus::name(window, label(&page, key));
    }
    let title = if page.get("app_id").and_then(Value::as_str).is_none() {
        let title = state.title();
        if title.is_empty() {
            label(&page, "Webpage").to_owned()
        } else {
            title
        }
    } else {
        page.get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut back = windows::core::BOOL(0);
    if let Some(web) = state.web.borrow().as_ref() {
        unsafe {
            let _ = web.CanGoBack(&mut back);
        }
    }
    unsafe {
        let _ = SetWindowTextW(state.native.hwnd.get(), &HSTRING::from(&title));
        let _ = SetWindowTextW(chrome.title, &HSTRING::from(title));
        let _ = EnableWindow(chrome.back, back.as_bool());
        let _ = ShowWindow(
            chrome.back,
            if back.as_bool()
                && !page
                    .get("immersive")
                    .and_then(Value::as_bool)
                    .unwrap_or_default()
            {
                SW_SHOW
            } else {
                SW_HIDE
            },
        );
        let _ = ShowWindow(
            chrome.icon,
            if !back.as_bool()
                && state.icon.borrow().is_some()
                && !page
                    .get("immersive")
                    .and_then(Value::as_bool)
                    .unwrap_or_default()
            {
                SW_SHOW
            } else {
                SW_HIDE
            },
        );
        let _ = ShowWindow(
            chrome.title,
            if page
                .get("immersive")
                .and_then(Value::as_bool)
                .unwrap_or_default()
            {
                SW_HIDE
            } else {
                SW_SHOW
            },
        );
        let dark = i32::from(state.dark.get());
        let _ = DwmSetWindowAttribute(
            state.native.hwnd.get(),
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark as *const i32).cast(),
            std::mem::size_of_val(&dark) as u32,
        );
    }
}

fn icon(state: &State, page: &Value, chrome: Chrome) {
    let source = page.get("icon_data").and_then(Value::as_str);
    if state.icon.borrow().as_ref().map(|icon| icon.data.as_str()) == source {
        return;
    }
    clear_icon(state);
    let Some(source) = source.filter(|source| source.len() <= 1_400_000) else {
        return;
    };
    let Some(data) = STANDARD.decode(source).ok() else {
        return;
    };
    if !data.starts_with(b"\x89PNG\r\n\x1a\n") || data.len() > 1024 * 1024 {
        return;
    }
    // PNG-compressed RT_ICON data is supported on every targeted Windows version.
    let Ok(handle) =
        (unsafe { CreateIconFromResourceEx(&data, true, 0x0003_0000, 24, 24, LR_DEFAULTCOLOR) })
    else {
        return;
    };
    unsafe {
        SendMessageW(
            state.native.hwnd.get(),
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(handle.0 as isize)),
        );
        SendMessageW(
            chrome.icon,
            STM_SETIMAGE,
            Some(WPARAM(IMAGE_ICON.0 as usize)),
            Some(LPARAM(handle.0 as isize)),
        );
    }
    *state.icon.borrow_mut() = Some(Icon {
        data: source.to_owned(),
        handle,
    });
}

pub(super) fn clear_icon(state: &State) {
    unsafe {
        SendMessageW(
            state.native.hwnd.get(),
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(0)),
        );
        if let Some(chrome) = state.chrome.borrow().as_ref() {
            SendMessageW(
                chrome.icon,
                STM_SETIMAGE,
                Some(WPARAM(IMAGE_ICON.0 as usize)),
                Some(LPARAM(0)),
            );
        }
    }
    state.icon.borrow_mut().take();
}

pub(super) fn raise(state: &State) {
    for button in buttons(state) {
        unsafe {
            let _ = SetWindowPos(
                button,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

pub(super) fn buttons(state: &State) -> Vec<HWND> {
    state
        .chrome
        .borrow()
        .as_ref()
        .map(|chrome| vec![chrome.back, chrome.more, chrome.close])
        .unwrap_or_default()
}

fn command(state: &Rc<State>, id: usize) {
    if id == MORE {
        menu(state);
        return;
    }
    if id == CLOSE {
        state.close();
        return;
    }
    let Some(web) = state.web.borrow().clone() else {
        return;
    };
    match id {
        BACK => unsafe {
            let _ = web.GoBack();
        },
        REFRESH => unsafe {
            if web.Reload().is_err() {
                state.fail();
            }
        },
        SHARE => {
            if state
                .page
                .borrow()
                .get("shareable")
                .and_then(Value::as_bool)
                .unwrap_or(true)
                && let Some(url) = state.url().filter(|url| is_web(url))
            {
                state.emit("share", json!({"url":url,"title":state.title()}));
            }
        }
        COPY => {
            if let Some(url) = state.url().filter(|url| is_web(url))
                && copy(state.native.hwnd.get(), &url).is_err()
            {
                state.fail();
            }
        }
        BROWSER => {
            if let Some(url) = state.url().filter(|url| is_web(url)) {
                state.emit("browser", Value::String(url));
            }
        }
        _ => {}
    }
}

fn menu(state: &Rc<State>) {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        state.fail();
        return;
    };
    {
        let page = state.page.borrow();
        for (id, name) in [
            (REFRESH, "Refresh"),
            (SHARE, "Forward…"),
            (COPY, "Copy link"),
            (BROWSER, "Open in browser"),
        ] {
            if id == SHARE
                && !page
                    .get("shareable")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            {
                continue;
            }
            unsafe {
                let _ = AppendMenuW(menu, MF_STRING, id, &HSTRING::from(label(&page, name)));
            }
        }
    }
    let mut rect = RECT::default();
    let command_id = unsafe {
        let _ = GetWindowRect(state.native.hwnd.get(), &mut rect);
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTALIGN,
            rect.right - 16,
            rect.top + 76,
            None,
            state.native.hwnd.get(),
            None,
        )
        .0
    };
    unsafe {
        let _ = DestroyMenu(menu);
    }
    if command_id > 0 && !state.closed.get() {
        command(state, command_id as usize);
    }
}

fn label<'a>(page: &'a Value, key: &'a str) -> &'a str {
    page.get("labels")
        .and_then(|labels| labels.get(key))
        .and_then(Value::as_str)
        .unwrap_or(key)
}

fn copy(window: HWND, text: &str) -> windows::core::Result<()> {
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    unsafe {
        OpenClipboard(Some(window))?;
        let result = (|| {
            EmptyClipboard()?;
            let memory = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2)?;
            let pointer = GlobalLock(memory);
            if pointer.is_null() {
                let _ = windows::Win32::Foundation::GlobalFree(Some(memory));
                return Err(windows::core::Error::from_thread());
            }
            std::ptr::copy_nonoverlapping(text.as_ptr(), pointer.cast::<u16>(), text.len());
            let _ = GlobalUnlock(memory);
            if let Err(error) = SetClipboardData(13, Some(HANDLE(memory.0))) {
                let _ = windows::Win32::Foundation::GlobalFree(Some(memory));
                return Err(error);
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}

pub(super) fn theme(state: &State, color: Option<&Value>) {
    let color = color
        .and_then(Value::as_array)
        .filter(|value| value.len() == 3)
        .and_then(|value| {
            Some((
                u8::try_from(value[0].as_u64()?).ok()?,
                u8::try_from(value[1].as_u64()?).ok()?,
                u8::try_from(value[2].as_u64()?).ok()?,
            ))
        });
    // This is webpage data, not an app palette literal. None restores the OS color.
    let color = color.map_or(u32::MAX, |(r, g, b)| {
        u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16
    });
    unsafe {
        let _ = DwmSetWindowAttribute(
            state.native.hwnd.get(),
            DWMWA_CAPTION_COLOR,
            (&color as *const u32).cast(),
            std::mem::size_of_val(&color) as u32,
        );
    }
}
