//! Native window ownership and bounded deferred work on WebView2's STA thread.
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{COLOR_WINDOW, GetSysColorBrush},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
    core::{HSTRING, PCWSTR, Result, w},
};

const RUN: u32 = WM_APP + 91;
const DEADLINE: usize = 91;
type Task = Box<dyn FnOnce()>;
type Command = Box<dyn Fn(usize)>;

pub(super) struct Window {
    pub hwnd: Cell<HWND>,
    tasks: RefCell<VecDeque<Task>>,
    deadline: RefCell<Option<Task>>,
    pub resize: RefCell<Option<Box<dyn Fn()>>>,
    pub command: RefCell<Option<Command>>,
    pub closing: RefCell<Option<Box<dyn Fn()>>>,
    raw_held: Cell<bool>,
}

impl Window {
    pub fn new(title: &str, visible: bool) -> Result<Rc<Self>> {
        let window = Rc::new(Self {
            hwnd: Cell::new(HWND::default()),
            tasks: RefCell::default(),
            deadline: RefCell::default(),
            resize: RefCell::default(),
            command: RefCell::default(),
            closing: RefCell::default(),
            raw_held: Cell::new(true),
        });
        // SAFETY: registration and window creation run on the platform UI thread.
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance.into(),
                lpszClassName: w!("MixinNshBotWindow"),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                ..Default::default()
            };
            // Re-registering the same process-owned class is harmless.
            RegisterClassW(&class);
            let raw = Rc::into_raw(window.clone());
            let created = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class.lpszClassName,
                &HSTRING::from(title),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                396,
                790,
                None,
                None,
                Some(instance.into()),
                Some(raw.cast()),
            );
            match created {
                Ok(hwnd) => {
                    window.hwnd.set(hwnd);
                    if visible {
                        let _ = ShowWindow(hwnd, SW_SHOW);
                    }
                }
                Err(error) => {
                    // WM_NCCREATE did not retain this failed window.
                    if window.raw_held.replace(false) {
                        drop(Rc::from_raw(raw));
                    }
                    return Err(error);
                }
            }
        }
        Ok(window)
    }

    pub fn later(&self, task: impl FnOnce() + 'static) -> bool {
        if self.hwnd.get().is_invalid() || self.tasks.borrow().len() >= 32 {
            return false;
        }
        self.tasks.borrow_mut().push_back(Box::new(task));
        // SAFETY: this is our live process-owned HWND; no pointer crosses threads.
        if unsafe { PostMessageW(Some(self.hwnd.get()), RUN, WPARAM(0), LPARAM(0)) }.is_err() {
            self.tasks.borrow_mut().pop_back();
            return false;
        }
        true
    }

    pub fn timeout(&self, task: impl FnOnce() + 'static) {
        *self.deadline.borrow_mut() = Some(Box::new(task));
        // SAFETY: the timer is owned and removed with this window.
        if unsafe { SetTimer(Some(self.hwnd.get()), DEADLINE, 15_000, None) } == 0 {
            let task = self.deadline.borrow_mut().take();
            if let Some(task) = task {
                task();
            }
        }
    }

    pub fn clear_timeout(&self) {
        self.deadline.borrow_mut().take();
        unsafe {
            let _ = KillTimer(Some(self.hwnd.get()), DEADLINE);
        }
    }

    pub fn bounds(&self) -> RECT {
        let mut rect = RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd.get(), &mut rect);
        }
        rect
    }

    pub fn focus(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd.get(), SW_RESTORE);
            let _ = SetForegroundWindow(self.hwnd.get());
        }
    }

    pub fn close(&self) {
        self.clear_timeout();
        self.tasks.borrow_mut().clear();
        let hwnd = self.hwnd.replace(HWND::default());
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: WM_NCCREATE receives the pointer passed to CreateWindowExW.
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Window };
    if !raw.is_null() {
        // Keep state alive when a callback synchronously destroys the window.
        unsafe {
            Rc::increment_strong_count(raw);
        }
        let state = unsafe { Rc::from_raw(raw) };
        match message {
            RUN => {
                let task = state.tasks.borrow_mut().pop_front();
                if let Some(task) = task {
                    task();
                }
                return LRESULT(0);
            }
            WM_TIMER if wparam.0 == DEADLINE => {
                let task = state.deadline.borrow_mut().take();
                state.clear_timeout();
                if let Some(task) = task {
                    task();
                }
                return LRESULT(0);
            }
            WM_SIZE => {
                if let Some(resize) = state.resize.borrow().as_ref() {
                    resize();
                }
            }
            WM_COMMAND => {
                if let Some(command) = state.command.borrow().as_ref() {
                    command(wparam.0 & 0xffff);
                }
                return LRESULT(0);
            }
            WM_CLOSE => {
                if let Some(close) = state.closing.borrow().as_ref() {
                    close();
                } else {
                    state.close();
                }
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                state.hwnd.set(HWND::default());
                if state.raw_held.replace(false) {
                    drop(unsafe { Rc::from_raw(raw) });
                }
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

pub(super) fn null() -> PCWSTR {
    PCWSTR::null()
}
