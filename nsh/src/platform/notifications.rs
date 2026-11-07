use std::{fmt, future::Future};

use tokio::sync::mpsc::UnboundedReceiver;

#[cfg(target_os = "macos")]
#[path = "notifications_macos.rs"]
mod macos;

#[cfg(target_os = "linux")]
#[path = "notifications_linux.rs"]
mod linux;

#[cfg(target_os = "windows")]
#[path = "notifications_windows.rs"]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn finish_shutdown() {
    platform::finish_shutdown();
}

#[cfg(target_os = "windows")]
pub(crate) fn prepare_activation() -> Result<Option<Event>, Error> {
    windows::run_activation_server()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Availability {
    Available,
    MissingAppBundle,
    #[cfg_attr(
        any(target_os = "macos", target_os = "linux", target_os = "windows"),
        allow(
            dead_code,
            reason = "Used by targets without native notification delivery"
        )
    )]
    UnsupportedPlatform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    any(target_os = "linux", target_os = "windows"),
    allow(
        dead_code,
        reason = "Shared permission states include macOS-only authorization outcomes"
    )
)]
pub(crate) enum Permission {
    NotDetermined,
    Denied,
    Authorized,
    Provisional,
    Unknown,
    #[cfg(target_os = "linux")]
    NotRequired,
}

impl Permission {
    pub(crate) fn permits_delivery(self) -> bool {
        #[cfg(target_os = "linux")]
        {
            matches!(
                self,
                Self::Authorized | Self::Provisional | Self::NotRequired
            )
        }
        #[cfg(not(target_os = "linux"))]
        {
            matches!(self, Self::Authorized | Self::Provisional)
        }
    }
}

pub(crate) struct Notification {
    pub id: String,
    pub account_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug)]
pub(crate) enum Event {
    Clicked {
        account_id: String,
        conversation_id: String,
        message_id: String,
    },
}

#[derive(Debug)]
pub(crate) enum Error {
    #[cfg_attr(
        target_os = "linux",
        allow(
            dead_code,
            reason = "Linux has no package-identity notification preflight"
        )
    )]
    Unavailable(Availability),
    Native(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(Availability::MissingAppBundle) => {
                formatter.write_str("Notifications require launching the packaged Mixin app")
            }
            Self::Unavailable(_) => {
                formatter.write_str("Notifications are unavailable on this platform")
            }
            Self::Native(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) fn availability() -> Availability {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        platform::availability()
    }
    #[cfg(target_os = "linux")]
    {
        Availability::Available
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Availability::UnsupportedPlatform
    }
}

/// Native notification delivery. Call methods on the UI thread; completion
/// futures contain no Objective-C objects and may be awaited independently.
pub(crate) struct NativeNotifications {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    native: platform::NativeNotifications,
}

impl NativeNotifications {
    pub(crate) fn new() -> Result<(Self, UnboundedReceiver<Event>), Error> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            let (native, events) = platform::NativeNotifications::new()?;
            Ok((Self { native }, events))
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(Error::Unavailable(Availability::UnsupportedPlatform))
        }
    }

    pub(crate) fn permission(
        &self,
    ) -> impl Future<Output = Result<Permission, Error>> + Send + 'static + use<> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.native.permission()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            async { Err(Error::Unavailable(Availability::UnsupportedPlatform)) }
        }
    }

    pub(crate) fn request_permission(
        &self,
    ) -> impl Future<Output = Result<Permission, Error>> + Send + 'static + use<> {
        #[cfg(target_os = "macos")]
        {
            self.native.request_permission()
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            self.native.permission()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            async { Err(Error::Unavailable(Availability::UnsupportedPlatform)) }
        }
    }

    /// Success means the system accepted the request, not that a banner was seen.
    pub(crate) fn deliver(
        &self,
        notification: Notification,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'static + use<> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.native.deliver(notification)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = notification;
            async { Err(Error::Unavailable(Availability::UnsupportedPlatform)) }
        }
    }

    pub(crate) fn dismiss(&self, ids: &[String]) {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        self.native.dismiss(ids);
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let _ = ids;
    }

    pub(crate) fn existing_ids(
        &self,
        account_id: String,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send + 'static + use<> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.native.existing_ids(account_id)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = account_id;
            async { Err(Error::Unavailable(Availability::UnsupportedPlatform)) }
        }
    }

    pub(crate) fn shutdown(&self) {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        self.native.shutdown();
    }
}

impl Drop for NativeNotifications {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn unbundled_process_rejects_notifications_without_opening_the_system_center() {
        assert_eq!(availability(), Availability::MissingAppBundle);
        assert!(matches!(
            NativeNotifications::new(),
            Err(Error::Unavailable(Availability::MissingAppBundle))
        ));
    }
}
