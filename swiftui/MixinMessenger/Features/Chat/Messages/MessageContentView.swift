import SwiftUI

struct MessageContentView: View {
    @Environment(AccountSession.self) private var session
    @Environment(SettingsPreferencesModel.self) private var preferences

    let message: SwiftMessageItem
    let mentionNames: [String: String]
    let audioPlaylist: [SwiftMessageItem]
    let mediaDirectory: URL?
    let conversationName: String?
    let outgoing: Bool
    let recalledText: String?
    let imageMessages: [SwiftMessageItem]
    let loadImageWindow: (String) async throws -> [SwiftImageMessageItem]
    let attachmentProgress: Double
    let onAttachmentAction: () -> Void
    let loadTranscript: () async throws -> [SwiftMessageItem]
    let onTranscriptAttachmentAction: (SwiftMessageItem) async -> Void
    let onReedit: (String) -> Void
    let onMarkAudioRead: (String) -> Void
    let onShowStickerDetail: (String) -> Void
    let onAppAction: (String, String) -> Void
    let onStrangerAction: (String) async -> Bool

    @ViewBuilder
    var body: some View {
        if message.status.uppercased() == "FAILED" {
            WaitingMessageView(
                subject: outgoing ? "your linked device" : message.senderName
            )
        } else if message.status.uppercased() == "UNKNOWN"
            || message.hasInvalidSpecialPayload
            || message.hasInvalidImagePayload
        {
            UnsupportedMessageView(category: message.category)
        } else {
            supportedContent
        }
    }

    @ViewBuilder
    private var supportedContent: some View {
        let category = message.category.uppercased()
        if category == "SYSTEM_CONVERSATION" {
            SystemConversationMessageView(
                text: message.systemConversationText(
                    currentUserID: session.profile.userId
                ),
                fontSize: 14 + preferences.chatFontSizeDelta
            )
        } else if category == "MESSAGE_RECALL" {
            RecallMessageView(
                outgoing: outgoing,
                recalledText: recalledText,
                onReedit: onReedit
            )
        } else if message.isStandaloneSpecial {
            SpecialMessageContentView(
                message: message,
                mentionNames: mentionNames,
                onStrangerAction: onStrangerAction
            )
        } else if category == "APP_CARD" || category == "APP_BUTTON_GROUP" {
            AppMessageView(message: message, onAction: onAppAction)
        } else if message.isRichContent {
            RichMessageContent(
                message: message,
                mentionNames: mentionNames,
                outgoing: outgoing,
                imageMessages: imageMessages,
                loadImageWindow: loadImageWindow,
                progress: { attachmentProgress },
                onAttachmentAction: onAttachmentAction,
                loadTranscript: loadTranscript,
                onTranscriptAttachmentAction: onTranscriptAttachmentAction,
                onAppAction: onAppAction
            )
        } else {
            simpleContent
        }
    }

    @ViewBuilder
    private var simpleContent: some View {
        switch message.presentationKind {
        case .sticker:
            StickerMessageView(
                message: message,
                onShowDetail: onShowStickerDetail
            )
        case .image, .video, .file:
            AttachmentFallbackMessageView(message: message)
        case .audio:
            AudioMessageView(
                message: message,
                playlist: audioPlaylist,
                mediaDirectory: mediaDirectory,
                conversationName: conversationName,
                outgoing: outgoing,
                onDownload: onAttachmentAction,
                onCancelTransfer: onAttachmentAction,
                onRetryTransfer: onAttachmentAction,
                onMarkRead: onMarkAudioRead
            )
        case .contact:
            ContactMessageView(message: message)
        case .special:
            SpecialSnapshotMessageCard(message: message)
        case .transfer:
            TransferFallbackMessageView(message: message)
        case .system:
            SystemFallbackMessageView(message: message)
        case .text, .unknown:
            TextMessageView(
                message: message,
                mentionNames: mentionNames,
                fontSize: 16 + preferences.chatFontSizeDelta
            )
        }
    }
}
