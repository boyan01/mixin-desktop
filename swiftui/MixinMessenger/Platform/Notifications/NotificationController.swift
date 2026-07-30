import AppKit
@preconcurrency import UserNotifications

@MainActor
final class NotificationController: NSObject, UNUserNotificationCenterDelegate {
    private weak var navigation: HomeNavigationModel?
    private weak var preferences: SettingsPreferencesModel?
    private var subscription: SwiftNotificationSubscription?
    private var task: Task<Void, Never>?

    func start(
        account: SwiftAccountHandle,
        navigation: HomeNavigationModel,
        preferences: SettingsPreferencesModel
    ) {
        stop()
        self.navigation = navigation
        self.preferences = preferences
        let center = UNUserNotificationCenter.current()
        center.delegate = self

        let subscription = account.notificationEvents()
        self.subscription = subscription
        task = Task {
            while !Task.isCancelled {
                do {
                    guard let event = try await subscription.next() else {
                        return
                    }
                    await handle(event, center: center)
                } catch {
                    NSLog("Mixin notification stream failed: %@", error.localizedDescription)
                    return
                }
            }
        }
    }

    func stop() {
        subscription?.cancel()
        subscription = nil
        task?.cancel()
        task = nil
        navigation = nil
        preferences = nil
        if UNUserNotificationCenter.current().delegate === self {
            UNUserNotificationCenter.current().delegate = nil
        }
    }

    func dismiss(conversationID: String) {
        let center = UNUserNotificationCenter.current()
        center.getDeliveredNotifications { notifications in
            let identifiers = notifications.compactMap { notification in
                notification.request.content.userInfo["conversation_id"] as? String
                    == conversationID
                    ? notification.request.identifier
                    : nil
            }
            center.removeDeliveredNotifications(withIdentifiers: identifiers)
        }
        center.getPendingNotificationRequests { requests in
            let identifiers = requests.compactMap { request in
                request.content.userInfo["conversation_id"] as? String == conversationID
                    ? request.identifier
                    : nil
            }
            center.removePendingNotificationRequests(withIdentifiers: identifiers)
        }
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        let info = response.notification.request.content.userInfo
        let conversationID = info["conversation_id"] as? String
        let conversationName = info["conversation_name"] as? String
        let messageID = info["message_id"] as? String
        Task { @MainActor [weak self] in
            NSApplication.shared.activate(ignoringOtherApps: true)
            if let conversationID, let messageID {
                self?.navigation?.locateMessage(
                    conversationID: conversationID,
                    messageID: messageID,
                    conversationName: conversationName
                )
            } else if let conversationID {
                self?.navigation?.selectConversation(
                    conversationID,
                    name: conversationName
                )
            }
            completionHandler()
        }
    }

    private func handle(
        _ event: NotificationEvent,
        center: UNUserNotificationCenter
    ) async {
        if let dismissMessageID = event.dismissMessageId {
            center.removeDeliveredNotifications(withIdentifiers: [dismissMessageID])
            center.removePendingNotificationRequests(withIdentifiers: [dismissMessageID])
            return
        }

        let createdAt = Date(
            timeIntervalSince1970: Double(event.createdAtMicros) / 1_000_000
        )
        guard createdAt >= Date().addingTimeInterval(-120) else {
            return
        }
        if NSApplication.shared.isActive,
           navigation?.selectedConversationID == event.conversationId
        {
            return
        }

        let content = UNMutableNotificationContent()
        content.title = event.conversationName
        content.body = preferences?.messagePreview == false
            ? "A message"
            : notificationPreview(event)
        content.sound = .default
        content.userInfo = [
            "conversation_id": event.conversationId,
            "conversation_name": event.conversationName,
            "message_id": event.messageId,
        ]
        do {
            try await center.add(
                UNNotificationRequest(
                    identifier: event.messageId,
                    content: content,
                    trigger: nil
                )
            )
        } catch {
            NSLog("Mixin notification delivery failed: %@", error.localizedDescription)
        }
    }

    private func notificationPreview(_ event: NotificationEvent) -> String {
        if event.category == "MESSAGE_PIN" {
            let pinned = pinnedContentPreview(event.content)
            return "\(event.senderName) pinned \(pinned)"
        }
        let content = contentPreview(category: event.category, content: event.content)
        return event.conversationCategory == "GROUP"
            ? "\(event.senderName): \(content)"
            : content
    }

    private func contentPreview(category: String, content: String) -> String {
        if category.contains("TEXT") {
            return content.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if category.contains("SNAPSHOT") { return "[Transfer]" }
        if category.contains("STICKER") { return "[Sticker]" }
        if category.contains("IMAGE") { return "[Image]" }
        if category.contains("VIDEO") { return "[Video]" }
        if category.contains("LIVE") { return "[Live]" }
        if category.contains("DATA") { return "[File]" }
        if category.contains("POST") {
            let value = content.trimmingCharacters(in: .whitespacesAndNewlines)
            return value.isEmpty ? "Post" : value
        }
        if category.contains("LOCATION") { return "[Location]" }
        if category.contains("AUDIO") { return "[Audio]" }
        if category.contains("CONTACT") { return "[Contact]" }
        if category.contains("TRANSCRIPT") { return "[Transcript]" }
        if category.contains("INSCRIPTION") { return "[Collectible]" }
        if category == "APP_BUTTON_GROUP" {
            guard let data = content.data(using: .utf8),
                  let buttons = try? JSONSerialization.jsonObject(
                    with: data
                  ) as? [[String: Any]]
            else {
                return ""
            }
            return buttons.map {
                "[\($0["label"] as? String ?? "")]"
            }.joined()
        }
        if category == "APP_CARD",
           let data = content.data(using: .utf8),
           let value = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        {
            return "[\(value["title"] as? String ?? "Card")]"
        }
        return "Unsupported message"
    }

    private func pinnedContentPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let value = try? JSONSerialization.jsonObject(
                with: data
              ) as? [String: Any],
              let category = value["category"] as? String
        else {
            return "A message"
        }
        return contentPreview(
            category: category,
            content: value["content"] as? String ?? ""
        )
    }
}
