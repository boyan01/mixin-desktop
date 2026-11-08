//! Windows shell integration, attached to NSH's actual window through an adopted child.
use std::{
    cell::{Cell, RefCell},
    mem::size_of,
    rc::Rc,
};

use nsh_platform_windows::{ComponentContext, Components, NativeComponent};
use serde_json::{Value, json};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
            MonitorFromWindow,
        },
        UI::{
            HiDpi::GetDpiForWindow,
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
    core::w,
};

mod tray;

const SUBCLASS: usize = 0x4d_58_53;
const TRAY_MESSAGE: u32 = WM_APP + 0x345;

struct State {
    context: ComponentContext,
    window: Cell<HWND>,
    restoring: Cell<bool>,
    bounds_loaded: Cell<bool>,
    moving: Cell<bool>,
    quitting: Cell<bool>,
    last_bounds: RefCell<Option<Value>>,
    labels: RefCell<(String, String)>,
    tooltip: RefCell<String>,
    tray: tray::Tray,
    taskbar_created: u32,
}

impl State {
    fn attach(&self, anchor: HWND) {
        if !self.window.get().is_invalid() {
            return;
        }
        // NSH adopts NativeComponent::window() before delivering properties. The
        // anchor's root is therefore this component's window, never an enumerated one.
        let window = unsafe { GetAncestor(anchor, GA_ROOT) };
        if window.is_invalid() || window == anchor {
            return;
        }
        // SAFETY: this state is owned by Host until its Drop removes the subclass.
        if unsafe {
            SetWindowSubclass(
                window,
                Some(window_proc),
                SUBCLASS,
                self as *const Self as usize,
            )
        }
        .as_bool()
        {
            self.window.set(window);
            let tooltip = self.tooltip.borrow().clone();
            self.tray.add(window, &tooltip);
        }
    }

    fn show(&self) {
        let window = self.window.get();
        if window.is_invalid() {
            return;
        }
        // SAFETY: the bound window belongs to NSH's current UI thread.
        unsafe {
            let _ = ShowWindow(
                window,
                if IsIconic(window).as_bool() {
                    SW_RESTORE
                } else {
                    SW_SHOW
                },
            );
            let _ = SetForegroundWindow(window);
        }
    }

    fn hide(&self) {
        self.emit_bounds();
        if !self.window.get().is_invalid() {
            unsafe {
                let _ = ShowWindow(
                    self.window.get(),
                    if self.tray.active() {
                        SW_HIDE
                    } else {
                        SW_MINIMIZE
                    },
                );
            }
        }
    }

    fn quit(&self) {
        self.emit_bounds();
        if !self.window.get().is_invalid() {
            // DestroyWindow reaches NSH's WM_DESTROY handler and returns its event
            // loop, so the normal Rust runtime shutdown can complete.
            unsafe {
                let _ = DestroyWindow(self.window.get());
            }
        }
    }

    fn request_quit(&self) {
        if self.quitting.replace(true) {
            return;
        }
        self.emit_bounds();
        self.context.emit(
            "quit",
            self.last_bounds.borrow().clone().unwrap_or(Value::Null),
        );
    }

    fn emit_bounds(&self) {
        let window = self.window.get();
        if window.is_invalid() || self.restoring.get() || !self.bounds_loaded.get() {
            return;
        }
        let mut placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        if unsafe { GetWindowPlacement(window, &mut placement) }.is_err() {
            return;
        }
        let mut rect = placement.rcNormalPosition;
        // WINDOWPLACEMENT uses workspace coordinates for ordinary top-level
        // windows; our public bounds contract uses physical screen coordinates.
        let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            let dx = info.rcWork.left - info.rcMonitor.left;
            let dy = info.rcWork.top - info.rcMonitor.top;
            rect.left += dx;
            rect.right += dx;
            rect.top += dy;
            rect.bottom += dy;
        }
        let bounds = json!({
            "x": rect.left, "y": rect.top,
            "width": rect.right - rect.left, "height": rect.bottom - rect.top,
            "maximized": placement.showCmd == SW_SHOWMAXIMIZED.0 as u32
                || (placement.showCmd == SW_SHOWMINIMIZED.0 as u32 && placement.flags.contains(WPF_RESTORETOMAXIMIZED)),
        });
        if self.last_bounds.borrow().as_ref() == Some(&bounds) {
            return;
        }
        *self.last_bounds.borrow_mut() = Some(bounds.clone());
        self.context.emit("bounds", bounds);
    }

    fn restore(&self, value: &Value) {
        if self.bounds_loaded.get() || !value.is_object() || self.window.get().is_invalid() {
            return;
        }
        let number = |name: &str| {
            value
                .get(name)
                .and_then(Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
        };
        let (Some(x), Some(y), Some(width), Some(height)) =
            (number("x"), number("y"), number("width"), number("height"))
        else {
            return;
        };
        if width <= 0 || height <= 0 {
            return;
        }
        self.bounds_loaded.set(true);
        let window = self.window.get();
        let rect = RECT {
            left: x,
            top: y,
            right: x.saturating_add(width),
            bottom: y.saturating_add(height),
        };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !unsafe { GetMonitorInfoW(MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST), &mut info) }
            .as_bool()
        {
            return;
        }
        let dpi = unsafe { GetDpiForWindow(window) }.max(96) as i32;
        let min_width = 384 * dpi / 96;
        let min_height = 480 * dpi / 96;
        let work = info.rcWork;
        let width = width.max(min_width).min((work.right - work.left).max(1));
        let height = height.max(min_height).min((work.bottom - work.top).max(1));
        let x = x.clamp(work.left, work.right - width);
        let y = y.clamp(work.top, work.bottom - height);
        self.restoring.set(true);
        unsafe {
            let _ = SetWindowPos(
                window,
                None,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
            if value
                .get("maximized")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let _ = ShowWindow(window, SW_MAXIMIZE);
            }
        }
        self.restoring.set(false);
    }

    fn tray_menu(&self) {
        let labels = self.labels.borrow().clone();
        if labels.0.is_empty() || labels.1.is_empty() {
            return;
        }
        match tray::menu(self.window.get(), &labels.0, &labels.1) {
            1 => self.show(),
            2 => self.request_quit(),
            _ => {}
        }
    }
}

// SAFETY: installed and removed on the window's UI thread. Every callback takes
// a temporary strong reference because DestroyWindow can synchronously drop Host.
unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let pointer = data as *const State;
    unsafe {
        Rc::increment_strong_count(pointer);
    }
    let state = unsafe { Rc::from_raw(pointer) };
    if message == WM_NCDESTROY {
        state.window.set(HWND::default());
        state.tray.remove(window);
        unsafe {
            let _ = RemoveWindowSubclass(window, Some(window_proc), SUBCLASS);
        }
    } else if message == WM_CLOSE {
        if state.tray.active() {
            state.hide();
        } else {
            state.request_quit();
        }
        return LRESULT(0);
    } else if message == WM_GETMINMAXINFO {
        let result = unsafe { DefSubclassProc(window, message, wparam, lparam) };
        if lparam.0 != 0 {
            let dpi = unsafe { GetDpiForWindow(window) }.max(96) as i32;
            let limits = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
            limits.ptMinTrackSize.x = 384 * dpi / 96;
            limits.ptMinTrackSize.y = 480 * dpi / 96;
        }
        return result;
    } else if message == WM_ENTERSIZEMOVE {
        state.moving.set(true);
        state.bounds_loaded.set(true);
    } else if message == WM_EXITSIZEMOVE {
        state.moving.set(false);
        state.emit_bounds();
    } else if message == WM_SIZE && !state.moving.get() && wparam.0 != SIZE_MINIMIZED as usize {
        state.emit_bounds();
    } else if message == TRAY_MESSAGE {
        match lparam.0 as u32 {
            WM_LBUTTONUP => state.show(),
            WM_RBUTTONUP | WM_CONTEXTMENU => state.tray_menu(),
            _ => {}
        }
        return LRESULT(0);
    } else if state.taskbar_created != 0 && message == state.taskbar_created {
        let tooltip = state.tooltip.borrow().clone();
        state.tray.add(window, &tooltip);
    }
    unsafe { DefSubclassProc(window, message, wparam, lparam) }
}

struct Host {
    anchor: HWND,
    state: Rc<State>,
}

impl Host {
    fn new(context: ComponentContext) -> Self {
        // STATIC is a system class. This hidden anchor is adopted by NSH and never
        // participates in keyboard navigation or draws any application content.
        let anchor = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap_or_default();
        Self {
            anchor,
            state: Rc::new(State {
                context,
                window: Cell::new(HWND::default()),
                restoring: Cell::new(false),
                bounds_loaded: Cell::new(false),
                moving: Cell::new(false),
                quitting: Cell::new(false),
                last_bounds: RefCell::new(None),
                labels: RefCell::new((String::new(), String::new())),
                tooltip: RefCell::new(String::new()),
                tray: tray::Tray::new(),
                taskbar_created: unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
            }),
        }
    }
}

impl NativeComponent for Host {
    fn window(&self) -> Option<HWND> {
        (!self.anchor.is_invalid()).then_some(self.anchor)
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        self.state.attach(self.anchor);
        match name {
            "labels" => {
                if let Some(value) = value {
                    *self.state.labels.borrow_mut() = (
                        value
                            .get("show")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .into(),
                        value
                            .get("quit")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .into(),
                    );
                }
            }
            "tooltip" => {
                let tooltip = value.and_then(Value::as_str).unwrap_or_default().to_owned();
                *self.state.tooltip.borrow_mut() = tooltip.clone();
                self.state.tray.update(self.state.window.get(), &tooltip);
            }
            "bounds" => {
                if let Some(value) = value {
                    self.state.restore(value);
                }
                // Root delivers null after loading an absent preference. Do not
                // save startup geometry before that asynchronous load completes.
                self.state.bounds_loaded.set(true);
            }
            _ => {}
        }
    }
    fn command(&mut self, name: &str, _: &Value) {
        self.state.attach(self.anchor);
        match name {
            "show" => self.state.show(),
            "hide" => self.state.hide(),
            "request-quit" => self.state.request_quit(),
            "quit" => self.state.quit(),
            _ => {}
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let window = self.state.window.replace(HWND::default());
        if !window.is_invalid() {
            unsafe {
                let _ = RemoveWindowSubclass(window, Some(window_proc), SUBCLASS);
            }
            self.state.tray.remove(window);
        }
        if unsafe { IsWindow(Some(self.anchor)) }.as_bool() {
            unsafe {
                let _ = DestroyWindow(self.anchor);
            }
        }
    }
}

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinShell",
        Rc::new(|context| Box::new(Host::new(context))),
    );
}
