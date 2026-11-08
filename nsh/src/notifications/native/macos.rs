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
    delegate: Retained<Delegate>,
}

impl NativeNotifications {
    pub(super) fn new() -> Result<(Self, mpsc::UnboundedReceiver<Event>), Error> {
        let availability = availability();
        if availability != Availability::Available {
            return Err(Error::Unavailable(availability));
        }
        // Calling this for an unbundled command-line executable raises an
        // Objective-C exception. The bundle check must precede this call.
        let center = UNUserNotificationCenter::currentNotificationCenter();
        if center.delegate().is_some() {
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
        Ok((Self { center, delegate }, receiver))
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
        content.setSound(Some(&UNNotificationSound::defaultSound()));
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
            self.center.setDelegate(None);
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
