import SwiftUI

struct ConversationContentMessageRow: View {
    let message: SwiftMessageItem
    let imageMessages: [SwiftMessageItem]
    let account: SwiftAccountHandle
    let currentUserID: String

    @State private var operationError: String?

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            MixinRemoteImage(url: URL(string: message.senderAvatarUrl)) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Image(systemName: "person.crop.circle.fill")
                    .resizable()
                    .foregroundStyle(.secondary)
            }
            .frame(width: 34, height: 34)
            .clipShape(Circle())

            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text(message.senderName)
                        .font(.headline)
                        .lineLimit(1)
                    Spacer()
                    Text(message.conversationContentTimestamp)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                if message.conversationContentIsRich {
                    RichMessageContent(
                        message: message,
                        outgoing: message.senderId == currentUserID,
                        imageMessages: imageMessages,
                        loadImageWindow: { messageID in
                            try await account.imageMessagesAround(
                                conversationId: message.conversationId,
                                targetMessageId: messageID,
                                before: 40,
                                after: 40
                            )
                        },
                        progress: {
                            account.attachmentProgress(
                                messageId: message.messageId
                            )
                        },
                        onAttachmentAction: {
                            Task {
                                await performAttachmentAction()
                            }
                        },
                        loadTranscript: {
                            try await account.transcriptMessages(
                                transcriptId: message.messageId
                            )
                        },
                        onTranscriptAttachmentAction: { item in
                            await performTranscriptAttachmentAction(item)
                        }
                    )
                } else {
                    MessageRichText(
                        content: message.conversationContentDisplayText,
                        baseFontSize: 14
                    )
                }
            }
        }
        .padding(.vertical, 6)
        .alert(
            "Attachment action failed",
            isPresented: Binding(
                get: { operationError != nil },
                set: { if !$0 { operationError = nil } }
            )
        ) {
            Button("OK") {
                operationError = nil
            }
        } message: {
            Text(operationError ?? "")
        }
    }

    private func performAttachmentAction(_ item: SwiftMessageItem? = nil) async {
        let target = item ?? message
        do {
            switch target.mediaStatus.uppercased() {
            case "CANCELED":
                if target.senderId == currentUserID {
                    try await account.retryAttachment(
                        messageId: target.messageId
                    )
                } else {
                    try await account.downloadAttachment(
                        messageId: target.messageId
                    )
                }
            case "PENDING":
                try await account.cancelAttachment(
                    messageId: target.messageId
                )
            default:
                break
            }
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func performTranscriptAttachmentAction(
        _ item: SwiftMessageItem
    ) async {
        do {
            switch item.mediaStatus.uppercased() {
            case "CANCELED":
                if message.senderId == currentUserID {
                    try await account.retryTranscriptAttachment(
                        transcriptId: message.messageId
                    )
                } else {
                    try await account.downloadTranscriptAttachment(
                        transcriptId: message.messageId,
                        messageId: item.messageId
                    )
                }
            case "PENDING":
                try await account.cancelTranscriptAttachment(
                    transcriptId: message.messageId,
                    messageId: item.messageId
                )
            default:
                break
            }
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }
}

extension SwiftMessageItem {
    var conversationContentDate: Date {
        Date(timeIntervalSince1970: Double(createdAtMicros) / 1_000_000)
    }

    var conversationContentTimestamp: String {
        conversationContentDate.formatted(date: .abbreviated, time: .shortened)
    }

    var conversationContentDisplayText: String {
        let text = content.trimmingCharacters(in: .whitespacesAndNewlines)
        if !text.isEmpty {
            return text
        }
        if let caption = caption?.trimmingCharacters(in: .whitespacesAndNewlines),
           !caption.isEmpty
        {
            return caption
        }
        if let mediaName, !mediaName.isEmpty {
            return mediaName
        }
        return category.replacingOccurrences(of: "_", with: " ").capitalized
    }

    var conversationContentIsRich: Bool {
        category.hasSuffix("_IMAGE")
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_DATA")
            || category.hasSuffix("_TRANSCRIPT")
            || category.contains("POST")
            || category.hasSuffix("_LIVE")
    }
}
