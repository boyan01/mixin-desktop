#[cfg(target_os = "macos")]
pub fn is_available() -> bool {
    use objc2_local_authentication::{LAContext, LAPolicy};

    let context = unsafe { LAContext::new() };
    unsafe {
        context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_ok()
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_available() -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn authenticate() -> bool {
    use std::sync::{mpsc, Mutex};

    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAPolicy};

    let context = unsafe { LAContext::new() };
    if unsafe {
        context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_err()
    } {
        return false;
    }

    let (sender, receiver) = mpsc::sync_channel(1);
    let sender = Mutex::new(Some(sender));
    let reply = RcBlock::new(move |success: Bool, _: *mut NSError| {
        if let Some(sender) = sender.lock().ok().and_then(|mut sender| sender.take()) {
            let _ = sender.send(success.as_bool());
        }
    });
    let reason = NSString::from_str("Unlock Mixin Messenger");
    unsafe {
        context.evaluatePolicy_localizedReason_reply(
            LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
            &reason,
            &reply,
        );
    }
    receiver.recv().unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
pub fn authenticate() -> bool {
    false
}
