//! Package identity and process-local COM activation; installation owns registration.
use std::{ffi::c_void, sync::Arc};

use windows::{
    Win32::{
        Foundation::{
            CLASS_E_NOAGGREGATION, E_ABORT, E_ACCESSDENIED, E_INVALIDARG, E_POINTER,
            ERROR_INSUFFICIENT_BUFFER,
        },
        Storage::Packaging::Appx::GetCurrentApplicationUserModelId,
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, CoRegisterClassObject, CoRevokeClassObject, IClassFactory,
                IClassFactory_Impl, REGCLS_MULTIPLEUSE,
            },
            WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        },
        UI::Notifications::{
            INotificationActivationCallback, INotificationActivationCallback_Impl,
            NOTIFICATION_USER_INPUT_DATA,
        },
    },
    core::{BOOL, GUID, IUnknown, Interface, PCWSTR, PWSTR, Ref, implement},
};

use super::Payload;

pub(super) const FLAG: &str = "--mixin-toast-activation";
pub(super) const CLSID: GUID = GUID::from_u128(0xc7dfb9df_1f4c_48d8_a3d5_8bda631a17ec);

pub(super) fn identity() -> Option<String> {
    let mut length = 0;
    // SAFETY: The first call queries the required UTF-16 buffer length.
    if unsafe { GetCurrentApplicationUserModelId(&mut length, None) } != ERROR_INSUFFICIENT_BUFFER
        || !(2..=1024).contains(&length)
    {
        return None;
    }
    let mut value = vec![0_u16; length as usize];
    // SAFETY: The buffer has the requested capacity and remains valid during the call.
    unsafe { GetCurrentApplicationUserModelId(&mut length, Some(PWSTR(value.as_mut_ptr()))) }
        .ok()
        .ok()?;
    let value = String::from_utf16(value.get(..length.checked_sub(1)? as usize)?).ok()?;
    // Never clear history or register an activation object for an unrelated package.
    (value.starts_with("one.mixin.messenger.nsh_") && value.ends_with("!MixinMessenger"))
        .then_some(value)
}

pub(super) struct Apartment;

impl Apartment {
    pub(super) fn new() -> windows::core::Result<Self> {
        // SAFETY: Each notification worker owns a fresh thread and balances this initialization.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: Runs on the same thread as its successful RoInitialize.
        unsafe { RoUninitialize() };
    }
}

type Handler = Arc<dyn Fn(Payload) -> bool + Send + Sync>;

#[implement(INotificationActivationCallback)]
struct Callback {
    app_id: String,
    handler: Handler,
}

impl INotificationActivationCallback_Impl for Callback_Impl {
    fn Activate(
        &self,
        app_id: &PCWSTR,
        arguments: &PCWSTR,
        _data: *const NOTIFICATION_USER_INPUT_DATA,
        _count: u32,
    ) -> windows::core::Result<()> {
        // SAFETY: COM guarantees null-terminated strings for the callback duration.
        // Bounds also reject oversized payloads before allocating or parsing them.
        let app_id = unsafe { bounded_string(*app_id, 1024) };
        let arguments = unsafe { bounded_string(*arguments, 4096) };
        if app_id.as_deref() != Some(&self.app_id) {
            return Err(E_ACCESSDENIED.into());
        }
        let payload = arguments
            .and_then(|value| Payload::decode(&value))
            .ok_or_else(|| windows::core::Error::from(E_INVALIDARG))?;
        if (self.handler)(payload) {
            Ok(())
        } else {
            // A timed-out bootstrap no longer owns a receiver. Report that
            // failure to COM instead of acknowledging an activation it lost.
            Err(E_ABORT.into())
        }
    }
}

#[implement(IClassFactory)]
struct Factory {
    app_id: String,
    handler: Handler,
}

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        result: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if iid.is_null() || result.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: COM supplied a writable output pointer, checked above.
        unsafe { *result = std::ptr::null_mut() };
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let callback: INotificationActivationCallback = Callback {
            app_id: self.app_id.clone(),
            handler: self.handler.clone(),
        }
        .into();
        // SAFETY: QueryInterface writes the requested interface and retains it for COM.
        unsafe { callback.query(iid, result).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> windows::core::Result<()> {
        // The owning notification worker or bounded activation bootstrap retains the factory.
        Ok(())
    }
}

pub(super) struct Registration(u32);

impl Registration {
    pub(super) fn new(app_id: String, handler: Handler) -> windows::core::Result<Self> {
        let factory: IClassFactory = Factory { app_id, handler }.into();
        // SAFETY: The registered class factory implements the documented COM interface.
        // This changes only the process COM class table, never system registration.
        let cookie = unsafe {
            CoRegisterClassObject(&CLSID, &factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE)?
        };
        Ok(Self(cookie))
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        // SAFETY: This cookie was returned by the corresponding registration.
        let _ = unsafe { CoRevokeClassObject(self.0) };
    }
}

unsafe fn bounded_string(pointer: PCWSTR, limit: usize) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    for length in 0..=limit {
        // SAFETY: COM owns a null-terminated string; each read stays within the accepted bound.
        if unsafe { *pointer.0.add(length) } == 0 {
            // SAFETY: All preceding UTF-16 units were readable in the loop above.
            return String::from_utf16(unsafe { std::slice::from_raw_parts(pointer.0, length) })
                .ok();
        }
    }
    None
}
