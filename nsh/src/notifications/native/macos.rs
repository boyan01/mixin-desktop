use std::{future::Future, ptr::NonNull, sync::Mutex};

use block2::{DynBlock, RcBlock};
use objc2::{
    AnyThread, DefinedClass, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, ProtocolObject},
};
use objc2_foundation::{
    NSArray, NSBundle, NSDictionary, NSError, NSObject, NSObjectProtocol, NSString, ns_string,
};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent, UNNotification,
    UNNotificationDefaultActionIdentifier, UNNotificationPresentationOptions,
    UNNotificationRequest, UNNotificationResponse, UNNotificationSettings, UNNotificationSound,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use tokio::sync::{mpsc, oneshot};

use super::{Availability, Error, Event, Notification, Permission};

pub(super) fn availability() -> Availability {
    let bundle = NSBundle::mainBundle();
    if bundle.bundleIdentifier().is_some_and(|id| !id.is_empty())
        && std::path::Path::new(&bundle.bundlePath().to_string())
            .extension()
            .is_some_and(|extension| extension == "app")
    {
        Availability::Available
    } else {
        Availability::MissingAppBundle
    }
}

struct DelegateIvars {
    events: mpsc::UnboundedSender<Event>,
}

define_class!(
    // SAFETY: NSObject has no additional subclass requirements. The ivars
    // contain a thread-safe channel, as delegate callbacks may run off-thread.
    #[unsafe(super = NSObject)]
    #[name = "MixinDesktopNotificationDelegate"]
    #[ivars = DelegateIvars]
    struct Delegate;

    // SAFETY: NSObjectProtocol imposes no additional requirements.
    unsafe impl NSObjectProtocol for Delegate {}

    // SAFETY: These signatures match the framework's generated protocol.
    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion.call((UNNotificationPresentationOptions::Banner
                | UNNotificationPresentationOptions::List
                | UNNotificationPresentationOptions::Sound,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn respond(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            // SAFETY: Apple's default action identifier is a process-lifetime
            // framework constant. Dismissals and unknown actions do not navigate.
            if &*response.actionIdentifier() == unsafe { UNNotificationDefaultActionIdentifier } {
                let info = response.notification().request().content().userInfo();
                let read = |key: &NSString| {
                    info.objectForKey(key)
                        .and_then(|object| object.downcast::<NSString>().ok())
                        .map(|text| text.to_string())
                };
                if let (Some(account_id), Some(conversation_id), Some(message_id)) = (
                    read(ns_string!("mixinAccountId")),
                    read(ns_string!("mixinConversationId")),
                    read(ns_string!("mixinMessageId")),
                ) {
                    let _ = self.ivars().events.send(Event::Clicked {
                        account_id,
                        conversation_id,
                        message_id,
                    });
                }
            }
            completion.call(());
        }
    }
);

impl Delegate {
    fn new(events: mpsc::UnboundedSender<Event>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars { events });
        // SAFETY: NSObject init initializes the allocated subclass once.
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) struct NativeNotifications {
    center: Retained<UNUserNotificationCenter>,
    sound_name: &'static str,
    delegate: Retained<Delegate>,
    previous_delegate: Option<Retained<ProtocolObject<dyn UNUserNotificationCenterDelegate>>>,
}

impl NativeNotifications {
    pub(super) fn new() -> Result<(Self, mpsc::UnboundedReceiver<Event>), Error> {
        let availability = availability();
        if availability != Availability::Available {
            return Err(Error::Unavailable(availability));
        }
        // Calling this for an unbundled command-line executable raises an
        // Objective-C exception. The bundle check must precede this call.
        let legacy_sound =
            std::path::PathBuf::from(NSBundle::mainBundle().bundlePath().to_string())
                .join("Contents/Resources/mixin.caf");
        let sound_name = if legacy_sound.is_file() {
            "mixin.caf"
        } else {
            install_sound().map_err(|error| {
                Error::Native(format!("Installing notification sound: {error}"))
            })?;
            "mixin-messenger.caf"
        };
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let previous_delegate = center.delegate();
        // NSH installs its default delegate before mounting the Rust app.
        // Mixin owns message delivery and click routing, so replace only that
        // default; another Mixin instance or an unrelated delegate is an error.
        if previous_delegate.as_deref().is_some_and(|current| {
            let current: &AnyObject = current.as_ref();
            current.class().name() != c"NshNotificationDelegate"
        }) {
            return Err(Error::Native(
                "Another notification delegate is already registered".into(),
            ));
        }
        let (events, receiver) = mpsc::unbounded_channel();
        let delegate = Delegate::new(events);
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        // Flutter clears prior-process notifications during initialization.
        center.removeAllPendingNotificationRequests();
        center.removeAllDeliveredNotifications();
        Ok((
            Self {
                center,
                sound_name,
                delegate,
                previous_delegate,
            },
            receiver,
        ))
    }

    pub(super) fn permission(
        &self,
    ) -> impl Future<Output = Result<Permission, Error>> + Send + 'static + use<> {
        let (sender, receiver) = oneshot::channel();
        let sender = Mutex::new(Some(sender));
        let completion = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
            // SAFETY: The framework keeps the settings object alive for this callback.
            let status = unsafe { settings.as_ref() }.authorizationStatus();
            if let Ok(mut sender) = sender.lock()
                && let Some(sender) = sender.take()
            {
                let _ = sender.send(permission(status));
            }
        });
        self.center
            .getNotificationSettingsWithCompletionHandler(&completion);
        async move {
            receiver
                .await
                .map_err(|_| Error::Native("Notification settings callback ended".into()))
        }
    }

    pub(super) fn request_permission(
        &self,
    ) -> impl Future<Output = Result<Permission, Error>> + Send + 'static + use<> {
        let (sender, receiver) = oneshot::channel();
        let sender = Mutex::new(Some(sender));
        let completion = RcBlock::new(move |granted: Bool, error: *mut NSError| {
            let result = if error.is_null() {
                Ok(if granted.as_bool() {
                    Permission::Authorized
                } else {
                    Permission::Denied
                })
            } else {
                // SAFETY: A non-null NSError is valid for the callback's duration.
                Err(Error::Native(
                    unsafe { &*error }.localizedDescription().to_string(),
                ))
            };
            if let Ok(mut sender) = sender.lock()
                && let Some(sender) = sender.take()
            {
                let _ = sender.send(result);
            }
        });
        self.center
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert
                    | UNAuthorizationOptions::Sound
                    | UNAuthorizationOptions::Badge,
                &completion,
            );
        async move {
            receiver
                .await
                .map_err(|_| Error::Native("Notification permission callback ended".into()))?
        }
    }

    pub(super) fn deliver(
        &self,
        notification: Notification,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'static + use<> {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&notification.title));
        content.setBody(&NSString::from_str(&notification.body));
        content.setThreadIdentifier(&NSString::from_str(&notification.conversation_id));
        // Match Flutter: let Notification Center play the bundled sound once.
        content.setSound(Some(&UNNotificationSound::soundNamed(&NSString::from_str(
            self.sound_name,
        ))));
        let account_id = NSString::from_str(&notification.account_id);
        let conversation_id = NSString::from_str(&notification.conversation_id);
        let message_id = NSString::from_str(&notification.message_id);
        let info = NSDictionary::<NSString, AnyObject>::from_slices(
            &[
                ns_string!("mixinAccountId"),
                ns_string!("mixinConversationId"),
                ns_string!("mixinMessageId"),
            ],
            &[&account_id, &conversation_id, &message_id],
        );
        // SAFETY: Erasing dictionary generics preserves its Objective-C class.
        // Keys and values are NSString property-list objects accepted by userInfo.
        let info: Retained<NSDictionary> = unsafe { Retained::cast_unchecked(info) };
        unsafe {
            content.setUserInfo(&info);
        }
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&notification.id),
            &content,
            None,
        );
        let (sender, receiver) = oneshot::channel();
        let sender = Mutex::new(Some(sender));
        let completion = RcBlock::new(move |error: *mut NSError| {
            let result = if error.is_null() {
                Ok(())
            } else {
                // SAFETY: A non-null NSError is valid for this callback.
                Err(Error::Native(
                    unsafe { &*error }.localizedDescription().to_string(),
                ))
            };
            if let Ok(mut sender) = sender.lock()
                && let Some(sender) = sender.take()
            {
                let _ = sender.send(result);
            }
        });
        self.center
            .addNotificationRequest_withCompletionHandler(&request, Some(&completion));
        async move {
            receiver
                .await
                .map_err(|_| Error::Native("Notification delivery callback ended".into()))?
        }
    }

    pub(super) fn dismiss(&self, ids: &[String]) {
        let identifiers = ids
            .iter()
            .map(|id| NSString::from_str(id))
            .collect::<Vec<_>>();
        let identifiers = NSArray::from_retained_slice(&identifiers);
        self.center
            .removeDeliveredNotificationsWithIdentifiers(&identifiers);
        self.center
            .removePendingNotificationRequestsWithIdentifiers(&identifiers);
    }

    pub(super) fn existing_ids(
        &self,
        account_id: String,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send + 'static + use<> {
        let prefix = format!("{account_id}:");
        let pending_prefix = prefix.clone();
        let (delivered_tx, delivered_rx) = oneshot::channel();
        let delivered_tx = Mutex::new(Some(delivered_tx));
        let delivered = RcBlock::new(move |notifications: NonNull<NSArray<UNNotification>>| {
            // SAFETY: Apple keeps this array and its elements valid throughout the callback.
            let ids = unsafe { notifications.as_ref() }
                .iter()
                .map(|notification| notification.request().identifier().to_string())
                .filter(|id| id.starts_with(&prefix))
                .collect::<Vec<_>>();
            if let Ok(mut sender) = delivered_tx.lock()
                && let Some(sender) = sender.take()
            {
                let _ = sender.send(ids);
            }
        });
        let (pending_tx, pending_rx) = oneshot::channel();
        let pending_tx = Mutex::new(Some(pending_tx));
        let pending = RcBlock::new(
            move |notifications: NonNull<NSArray<UNNotificationRequest>>| {
                // SAFETY: Apple keeps this array and its elements valid throughout the callback.
                let ids = unsafe { notifications.as_ref() }
                    .iter()
                    .map(|notification| notification.identifier().to_string())
                    .filter(|id| id.starts_with(&pending_prefix))
                    .collect::<Vec<_>>();
                if let Ok(mut sender) = pending_tx.lock()
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(ids);
                }
            },
        );
        self.center
            .getDeliveredNotificationsWithCompletionHandler(&delivered);
        self.center
            .getPendingNotificationRequestsWithCompletionHandler(&pending);
        async move {
            let (mut delivered, pending) = futures::future::try_join(delivered_rx, pending_rx)
                .await
                .map_err(|_| Error::Native("Notification listing callback ended".into()))?;
            delivered.extend(pending);
            delivered.sort();
            delivered.dedup();
            Ok(delivered)
        }
    }

    pub(super) fn shutdown(&self) {
        if self
            .center
            .delegate()
            .as_deref()
            .is_some_and(|current| std::ptr::eq(current, ProtocolObject::from_ref(&*self.delegate)))
        {
            self.center.setDelegate(self.previous_delegate.as_deref());
            self.center.removeAllPendingNotificationRequests();
            self.center.removeAllDeliveredNotifications();
        }
    }
}

fn permission(status: UNAuthorizationStatus) -> Permission {
    match status {
        UNAuthorizationStatus::NotDetermined => Permission::NotDetermined,
        UNAuthorizationStatus::Denied => Permission::Denied,
        UNAuthorizationStatus::Authorized => Permission::Authorized,
        UNAuthorizationStatus::Provisional | UNAuthorizationStatus::Ephemeral => {
            Permission::Provisional
        }
        _ => Permission::Unknown,
    }
}

/// Notification Center searches the sandbox's Library/Sounds as well as the
/// bundle root. NSH's nested assets therefore need no bundle postprocessing.
fn install_sound() -> std::io::Result<()> {
    use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};
    let directories = NSFileManager::defaultManager().URLsForDirectory_inDomains(
        NSSearchPathDirectory::LibraryDirectory,
        NSSearchPathDomainMask::UserDomainMask,
    );
    let library = directories
        .firstObject()
        .and_then(|url| url.path())
        .ok_or_else(|| std::io::Error::other("Missing application Library directory"))?;
    let directory = std::path::PathBuf::from(library.to_string()).join("Sounds");
    let bundle_id = NSBundle::mainBundle()
        .bundleIdentifier()
        .ok_or_else(|| std::io::Error::other("Missing bundle identity"))?
        .to_string();
    prepare_sound_directory(&directory, &bundle_id)?;
    let destination = directory.join("mixin-messenger.caf");
    let bytes = include_bytes!("../../../assets/mixin.caf");
    if std::fs::read(&destination).ok().as_deref() == Some(bytes.as_slice()) {
        return Ok(());
    }
    let temporary = directory.join(".mixin-messenger.caf.tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, destination)
}

fn prepare_sound_directory(directory: &std::path::Path, bundle_id: &str) -> std::io::Result<()> {
    // macOS seeds containers with a Sounds symlink to the user's shared
    // Library. Preserve that link, and install only in this app's private
    // container: never follow it to write shared user sound resources.
    if std::fs::symlink_metadata(directory).is_ok_and(|meta| meta.file_type().is_symlink()) {
        let container_library = directory
            .parent()
            .ok_or_else(|| std::io::Error::other("Missing Library"))?;
        let suffix = std::path::PathBuf::from("Containers")
            .join(bundle_id)
            .join("Data/Library");
        if !container_library.ends_with(suffix)
            || std::fs::read_link(directory)? != std::path::Path::new("../../../../Sounds")
        {
            return Err(std::io::Error::other(
                "Refusing to replace a nonstandard Sounds link",
            ));
        }
        let backup = container_library.join("MixinSharedSounds");
        if backup.symlink_metadata().is_ok() {
            return Err(std::io::Error::other("Sounds link backup already exists"));
        }
        std::fs::rename(directory, &backup)?;
        if let Err(error) = std::fs::create_dir(directory) {
            let _ = std::fs::rename(&backup, directory);
            return Err(error);
        }
    }
    std::fs::create_dir_all(directory)?;
    Ok(())
}

#[cfg(test)]
mod sound_directory_tests {
    use super::*;

    #[test]
    fn preparing_container_sounds_preserves_the_shared_link_and_user_files() {
        let root =
            std::env::temp_dir().join(format!("mixin-private-sounds-{}", std::process::id()));
        let library = root.join("Containers/test.mixin/Data/Library");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(root.join("Sounds")).unwrap();
        std::fs::write(root.join("Sounds/user.caf"), b"user sound").unwrap();
        let directory = library.join("Sounds");
        std::os::unix::fs::symlink("../../../../Sounds", &directory).unwrap();
        prepare_sound_directory(&directory, "test.mixin").unwrap();
        assert!(
            !directory
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            std::fs::read_link(library.join("MixinSharedSounds")).unwrap(),
            std::path::Path::new("../../../../Sounds")
        );
        assert_eq!(
            std::fs::read(root.join("Sounds/user.caf")).unwrap(),
            b"user sound"
        );
        prepare_sound_directory(&directory, "test.mixin").unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preparing_sounds_rejects_a_custom_link_without_changing_it() {
        let root = std::env::temp_dir().join(format!("mixin-custom-sounds-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let directory = root.join("Sounds");
        std::os::unix::fs::symlink("custom", &directory).unwrap();
        assert!(prepare_sound_directory(&directory, "test.mixin").is_err());
        assert_eq!(
            std::fs::read_link(&directory).unwrap(),
            std::path::Path::new("custom")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
