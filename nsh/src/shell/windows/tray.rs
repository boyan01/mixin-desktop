//! Owned notification-area icon and standard Windows context menu.
use std::{cell::Cell, mem::size_of};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, POINT, WPARAM},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::PCWSTR,
};

pub(super) struct Tray {
    icon: HICON,
    active: Cell<bool>,
}

impl Tray {
    pub(super) fn new() -> Self {
        let bytes = include_bytes!("../../../assets/notify_icon.ico");
        // ICO directory entries describe independent icon resources. Prefer 32px,
        // which retains the original brand artwork without drawing a new badge.
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        let resource = (0..count)
            .filter_map(|index| {
                let entry = bytes.get(6 + index * 16..6 + (index + 1) * 16)?;
                let size = u32::from_le_bytes(entry[8..12].try_into().ok()?) as usize;
                let offset = u32::from_le_bytes(entry[12..16].try_into().ok()?) as usize;
                let data = bytes.get(offset..offset.checked_add(size)?)?;
                let width = if entry[0] == 0 { 256 } else { entry[0] as u32 };
                Some((width.abs_diff(32), data))
            })
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, data)| data);
        let icon = resource
            .and_then(|data| {
                unsafe { CreateIconFromResourceEx(data, true, 0x30000, 32, 32, LR_DEFAULTCOLOR) }
                    .ok()
            })
            .unwrap_or_default();
        Self {
            icon,
            active: Cell::new(false),
        }
    }

    fn data(&self, window: HWND, tooltip: &str) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: window,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: super::TRAY_MESSAGE,
            hIcon: self.icon,
            ..Default::default()
        };
        // Leave one NUL terminator, and avoid splitting a UTF-16 surrogate pair.
        let mut tip: Vec<u16> = tooltip.encode_utf16().take(data.szTip.len() - 1).collect();
        if tip
            .last()
            .is_some_and(|unit| (0xd800..=0xdbff).contains(unit))
        {
            tip.pop();
        }
        data.szTip[..tip.len()].copy_from_slice(&tip);
        data
    }

    pub(super) fn add(&self, window: HWND, tooltip: &str) {
        if window.is_invalid() || self.icon.is_invalid() {
            return;
        }
        // Legacy callback messages preserve mouse-up behavior used by Flutter's
        // Windows tray. Explorer restart invalidates its old registration.
        self.active
            .set(unsafe { Shell_NotifyIconW(NIM_ADD, &self.data(window, tooltip)) }.as_bool());
    }
    pub(super) fn update(&self, window: HWND, tooltip: &str) {
        if self.active.get() {
            let success =
                unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.data(window, tooltip)) }.as_bool();
            if !success {
                self.active.set(false);
            }
        }
        if !self.active.get() {
            self.add(window, tooltip);
        }
    }
    pub(super) fn remove(&self, window: HWND) {
        if self.active.replace(false) {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data(window, ""));
            }
        }
    }
    pub(super) fn active(&self) -> bool {
        self.active.get()
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        if !self.icon.is_invalid() {
            unsafe {
                let _ = DestroyIcon(self.icon);
            }
        }
    }
}

pub(super) fn menu(window: HWND, show: &str, quit: &str) -> u32 {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return 0;
    };
    let show: Vec<u16> = show.encode_utf16().chain(Some(0)).collect();
    let quit: Vec<u16> = quit.encode_utf16().chain(Some(0)).collect();
    // SAFETY: strings and menu remain live throughout the synchronous menu loop.
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, 1, PCWSTR(show.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, 2, PCWSTR(quit.as_ptr()));
        let _ = SetMenuDefaultItem(menu, 1, 0);
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        let _ = SetForegroundWindow(window);
        let choice = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            window,
            None,
        )
        .0 as u32;
        // Microsoft requires a benign message after a notification-icon popup so
        // dismissing it does not leave the next popup stuck open.
        let _ = PostMessageW(Some(window), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        choice
    }
}
