import Foundation

struct MessageActionPolicy: Equatable {
    let message: SwiftMessageItem
    let currentUserID: String
    let currentUserRole: String?
    let now: Date

    var canReply: Bool {
        message.category.canReply
    }

    var allowsMessageActions: Bool {
        !message.category.bypassesMessageActions
    }

    var canSelect: Bool {
        allowsMessageActions
    }

    var canPin: Bool {
        message.category.canReply
            && message.status.isCompletedMessageStatus
            && currentUserRole != nil
    }

    var canRecall: Bool {
        let baseAllowed = message.status.isCompletedMessageStatus
            && message.category.canRecall
            && now < message.createdAt.addingTimeInterval(30 * 24 * 60 * 60)
        guard baseAllowed else {
            return false
        }
        if message.senderId == currentUserID {
            return true
        }
        if message.conversationCategory?.uppercased() == "CONTACT" {
            return true
        }
        guard message.conversationCategory?.uppercased() == "GROUP" else {
            return false
        }
        if message.conversationOwnerId == currentUserID {
            return true
        }

        switch currentUserRole?.uppercased() {
        case "OWNER":
            return true
        case "ADMIN":
            return message.senderId != message.conversationOwnerId
                && message.senderParticipantId != nil
                && message.senderRole == nil
        default:
            return false
        }
    }

    var canDelete: Bool {
        allowsMessageActions
    }

    var copyableText: String? {
        if message.category.isPost || message.category.isText {
            return message.content
        }
        if message.category.isImage,
           let caption = message.caption?.trimmingCharacters(in: .whitespacesAndNewlines),
           !caption.isEmpty
        {
            return caption
        }
        return nil
    }
}

private extension String {
    var isCompletedMessageStatus: Bool {
        ["SENT", "DELIVERED", "READ"].contains(uppercased())
    }

    var isText: Bool {
        hasSuffix("_TEXT")
    }

    var isImage: Bool {
        hasSuffix("_IMAGE")
    }

    var isPost: Bool {
        hasSuffix("_POST")
    }

    var canReply: Bool {
        isText
            || isImage
            || hasSuffix("_VIDEO")
            || hasSuffix("_LIVE")
            || hasSuffix("_DATA")
            || isPost
            || hasSuffix("_LOCATION")
            || hasSuffix("_AUDIO")
            || hasSuffix("_STICKER")
            || hasSuffix("_CONTACT")
            || hasSuffix("_TRANSCRIPT")
            || self == "APP_CARD"
            || self == "APP_BUTTON_GROUP"
    }

    var canRecall: Bool {
        canReply && self != "APP_BUTTON_GROUP" && !hasPrefix("SYSTEM_")
    }

    var bypassesMessageActions: Bool {
        [
            "SYSTEM_CONVERSATION",
            "MESSAGE_PIN",
            "SECRET",
            "STRANGER",
        ].contains(uppercased())
    }
}

private extension SwiftMessageItem {
    var createdAt: Date {
        Date(timeIntervalSince1970: Double(createdAtMicros) / 1_000_000)
    }
}
