import SwiftUI

struct ImageMessageView: View {
    let message: SwiftMessageItem
    let mentionNames: [String: String]
    let outgoing: Bool
    let progress: () -> Double
    let onAttachmentAction: () -> Void
    let onOpen: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ZStack {
                MessageMediaImage(
                    source: message.mediaStatus.isComplete ? message.mediaUrl : nil,
                    thumbnail: message.thumbImage,
                    contentMode: .fill
                )
                .frame(width: mediaSize.width, height: mediaSize.height)
                .clipped()

                AttachmentStatusOverlay(
                    message: message,
                    outgoing: outgoing,
                    progress: progress,
                    completedSymbol: nil
                )
            }
            .contentShape(Rectangle())
            .onTapGesture(perform: mediaTap)

            if let caption = message.caption?.trimmingCharacters(
                in: .whitespacesAndNewlines
            ), !caption.isEmpty {
                MessageRichText(
                    content: caption,
                    baseFontSize: 13,
                    mentionNames: mentionNames
                )
            }
        }
    }

    private var mediaSize: CGSize {
        message.scaledMediaSize(
            maximum: CGSize(width: 320, height: 360),
            minimumWidth: 190
        )
    }

    private func mediaTap() {
        switch message.mediaStatus.uppercased() {
        case "DONE", "READ":
            onOpen()
        case "CANCELED", "PENDING":
            onAttachmentAction()
        default:
            break
        }
    }
}
