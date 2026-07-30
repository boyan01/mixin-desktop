import Foundation

extension ConversationListData: Identifiable {
    public var id: String {
        conversationId
    }

    var initial: String {
        name.first.map { String($0).uppercased() } ?? "?"
    }

    var preview: String {
        lastMessage
    }

    var hasDraft: Bool {
        status != 3 && !draft.isEmpty
    }

    var activeMembershipPlan: String? {
        guard let membership,
              let data = membership.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let plan = object["plan"] as? String,
              ["advance", "elite", "prosperity"].contains(plan),
              let expiration = object["expired_at"] as? String,
              let expirationDate = ISO8601DateFormatter.mixin.date(from: expiration),
              expirationDate > Date()
        else {
            return nil
        }
        return plan
    }

    var previewSymbol: String? {
        guard lastMessageStatus != "FAILED", let category = lastMessageCategory else {
            return nil
        }
        if category == "SYSTEM_SAFE_INSCRIPTION" {
            return "arrow.left.arrow.right"
        }
        if category.contains("TRANSCRIPT") || category.contains("DATA") {
            return "doc.fill"
        }
        if category.contains("IMAGE") {
            return "photo.fill"
        }
        if category.contains("VIDEO") || category.contains("LIVE") {
            return "video.fill"
        }
        if category.contains("AUDIO") || category.hasPrefix("WEBRTC_") {
            return "waveform"
        }
        if category.contains("LOCATION") {
            return "location.fill"
        }
        if category.contains("CONTACT") {
            return "person.crop.circle.fill"
        }
        if category.contains("STICKER") {
            return "face.smiling.fill"
        }
        if category.contains("SNAPSHOT") {
            return "arrow.left.arrow.right"
        }
        return nil
    }

    var statusSymbol: String? {
        switch lastMessageStatus?.uppercased() {
        case "FAILED":
            "exclamationmark.circle.fill"
        case "READ":
            "checkmark.circle.fill"
        case "DELIVERED":
            "checkmark.circle"
        case "SENT":
            "checkmark"
        case "SENDING", "PENDING":
            "clock"
        default:
            nil
        }
    }

    var statusAssetName: String? {
        switch lastMessageStatus?.uppercased() {
        case "READ":
            "ConversationRead"
        case "DELIVERED":
            "ConversationDelivered"
        case "SENT":
            "ConversationSent"
        default:
            nil
        }
    }

    func showsOutgoingStatus(for currentUserID: String) -> Bool {
        guard lastMessageSenderId == currentUserID, let category = lastMessageCategory else {
            return false
        }
        return !Self.statusUnsupportedCategories.contains(category)
    }

    func rowPreview(currentUserID: String) -> String {
        if hasDraft {
            return draft
        }
        guard let category = lastMessageCategory else {
            return "No messages yet"
        }
        let text: String
        if lastMessageStatus == "FAILED" {
            text = "Waiting for this message"
        } else if lastMessageStatus == "UNKNOWN" {
            text = "Message not supported"
        } else {
            text = categoryPreview(category: category, currentUserID: currentUserID)
        }
        if category == "SYSTEM_CONVERSATION" || category == "MESSAGE_PIN" {
            return text
        }
        let showSender = category == "GROUP" || self.category == "GROUP"
            || lastMessageSenderId != ownerId
        guard showSender, !text.isEmpty else {
            return text
        }
        let sender = lastMessageSenderId == currentUserID
            ? "You"
            : lastMessageSenderName ?? ""
        return sender.isEmpty ? text : "\(sender): \(text)"
    }

    private func categoryPreview(category: String, currentUserID: String) -> String {
        if category == "SYSTEM_CONVERSATION" {
            return systemConversationPreview(currentUserID: currentUserID)
        }
        if category == "MESSAGE_PIN" {
            let sender = lastMessageSenderName ?? ""
            return "\(sender) pinned \(Self.pinPreview(lastMessage))"
                .trimmingCharacters(in: .whitespaces)
        }
        if category.contains("TEXT") {
            return lastMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if category.contains("SNAPSHOT") {
            return "[Transfer]"
        }
        if category.contains("STICKER") {
            return "[Sticker]"
        }
        if category.contains("IMAGE") {
            return "[Image]"
        }
        if category.contains("VIDEO") {
            return "[Video]"
        }
        if category.contains("LIVE") {
            return "[Live]"
        }
        if category.contains("DATA") {
            return "[File]"
        }
        if category.contains("POST") {
            let content = lastMessage.trimmingCharacters(in: .whitespacesAndNewlines)
            return content.isEmpty ? "Post" : content
        }
        if category.contains("LOCATION") {
            return "[Location]"
        }
        if category.contains("AUDIO") {
            return "[Audio]"
        }
        if category == "APP_BUTTON_GROUP" {
            return Self.appButtonPreview(lastMessage)
        }
        if category == "APP_CARD" {
            return Self.appCardPreview(lastMessage)
        }
        if category.contains("CONTACT") {
            return "[Contact]"
        }
        if category.hasPrefix("WEBRTC_") || category.hasPrefix("KRAKEN_") {
            return "Voice call"
        }
        if category.contains("RECALL") {
            return lastMessageSenderId == currentUserID
                ? "[You deleted this message]"
                : "[This message was deleted]"
        }
        if category.contains("TRANSCRIPT") {
            return "[Transcript]"
        }
        if category.contains("INSCRIPTION") {
            return "[Collectible]"
        }
        return "Message not supported"
    }

    private func systemConversationPreview(currentUserID: String) -> String {
        let participant = lastMessageParticipantId == currentUserID
            ? "You"
            : lastMessageParticipantName ?? ""
        let sender = lastMessageSenderId == currentUserID ? "You" : lastMessageSenderName ?? ""
        switch lastMessageAction?.uppercased() {
        case "JOIN":
            return "\(participant) joined the group"
        case "EXIT":
            return "\(participant) left the group"
        case "ADD":
            return "\(sender) added \(participant)"
        case "REMOVE":
            return "\(sender) removed \(participant)"
        case "CREATE":
            return "\(sender) created this group"
        case "ROLE":
            return "\(participant) is now an admin"
        case "EXPIRE":
            guard let seconds = Int(lastMessage) else {
                return "\(sender) changed disappearing message settings"
            }
            if seconds <= 0 {
                return "\(sender) disabled disappearing messages"
            }
            return "\(sender) set disappearing messages to \(Self.durationText(seconds))"
        default:
            return "Message not supported"
        }
    }

    private static func durationText(_ seconds: Int) -> String {
        if seconds < 60 {
            return "\(seconds) sec"
        }
        if seconds < 3_600 {
            return "\(seconds / 60) min"
        }
        if seconds < 86_400 {
            return "\(seconds / 3_600) hr"
        }
        if seconds < 604_800 {
            return "\(seconds / 86_400) day"
        }
        return "\(seconds / 604_800) wk"
    }

    private static func appButtonPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let items = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]]
        else {
            return ""
        }
        return items.compactMap { $0["label"] as? String }
            .map { "[\($0)]" }
            .joined()
    }

    private static func appCardPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let card = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let title = card["title"] as? String
        else {
            return "[Card]"
        }
        return "[\(title)]"
    }

    private static func pinPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let item = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let category = item["category"] as? String
        else {
            return "a message"
        }
        let nestedContent = item["content"] as? String ?? ""
        if category.contains("TEXT") {
            return nestedContent.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if category.contains("IMAGE") {
            return "[Image]"
        }
        if category.contains("VIDEO") {
            return "[Video]"
        }
        if category.contains("AUDIO") {
            return "[Audio]"
        }
        if category.contains("DATA") {
            return "[File]"
        }
        if category.contains("STICKER") {
            return "[Sticker]"
        }
        if category.contains("CONTACT") {
            return "[Contact]"
        }
        if category.contains("LOCATION") {
            return "[Location]"
        }
        return "a message"
    }

    private static let statusUnsupportedCategories: Set<String> = [
        "SYSTEM_CONVERSATION",
        "SYSTEM_ACCOUNT_SNAPSHOT",
        "MESSAGE_RECALL",
        "MESSAGE_PIN",
        "WEBRTC_AUDIO_CANCEL",
        "WEBRTC_AUDIO_DECLINE",
        "WEBRTC_AUDIO_END",
        "WEBRTC_AUDIO_BUSY",
        "WEBRTC_AUDIO_FAILED",
        "KRAKEN_END",
        "KRAKEN_DECLINE",
        "KRAKEN_CANCEL",
        "KRAKEN_INVITE",
    ]

    var formattedTime: String {
        let date = Date(timeIntervalSince1970: TimeInterval(updatedAtMillis) / 1_000)
        if Calendar.current.isDateInToday(date) {
            return date.formatted(date: .omitted, time: .shortened)
        }
        return date.formatted(.dateTime.month(.twoDigits).day(.twoDigits))
    }
}

private extension ISO8601DateFormatter {
    static let mixin: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()
}
