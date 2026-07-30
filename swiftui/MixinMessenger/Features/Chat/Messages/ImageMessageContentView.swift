import AppKit
import SwiftUI

struct ImageMessageContentView: View {
    @Environment(SettingsPreferencesModel.self) private var preferences
    let message: MessageItem
    let mentionNames: [String: String]
    let outgoing: Bool
    let progress: () -> Double
    let onAttachmentAction: () -> Void
    let onOpen: () -> Void

    var body: some View {
        ImageMessageLayout(
            mediaWidth: message.mediaWidth,
            mediaHeight: message.mediaHeight
        ) {
            ZStack {
                MessageMediaImage(
                    source: message.mediaStatus.isComplete ? message.mediaUrl : nil,
                    thumbnail: message.thumbImage,
                    contentMode: .fill
                )
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
                    baseFontSize: 16 + preferences.chatFontSizeDelta,
                    mentionNames: mentionNames
                )
                .padding(.horizontal, 12)
                .padding(.vertical, 10)
            }
        }
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

private struct ImageMessageLayout: Layout {
    let mediaWidth: Int32?
    let mediaHeight: Int32?

    func makeCache(subviews _: Subviews) -> CGSize? {
        nil
    }

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout CGSize?
    ) -> CGSize {
        let mediaSize = resolvedMediaSize(availableWidth: proposal.width)
        guard subviews.count > 1 else {
            return mediaSize
        }
        let captionSize = subviews[1].sizeThatFits(
            ProposedViewSize(width: mediaSize.width, height: nil)
        )
        return CGSize(width: mediaSize.width, height: mediaSize.height + captionSize.height)
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout CGSize?
    ) {
        guard let image = subviews.first else {
            return
        }
        let mediaSize = resolvedMediaSize(availableWidth: proposal.width ?? bounds.width)
        image.place(
            at: bounds.origin,
            anchor: .topLeading,
            proposal: ProposedViewSize(width: mediaSize.width, height: mediaSize.height)
        )
        guard subviews.count > 1 else {
            return
        }
        subviews[1].place(
            at: CGPoint(x: bounds.minX, y: bounds.minY + mediaSize.height),
            anchor: .topLeading,
            proposal: ProposedViewSize(width: mediaSize.width, height: nil)
        )
    }

    private func resolvedMediaSize(availableWidth: CGFloat?) -> CGSize {
        let backingScale = NSScreen.main?.backingScaleFactor ?? 1
        let sourceWidth = max(1, CGFloat(mediaWidth ?? 1) / backingScale)
        let sourceHeight = max(1, CGFloat(mediaHeight ?? 1) / backingScale)
        let constraintWidth = availableWidth ?? 500
        let maximumWidth = min(constraintWidth * 0.6, 300)
        let minimumWidth = max(constraintWidth * 0.2, 200)
        let width = max(min(sourceWidth, maximumWidth), minimumWidth)
        let screenHeight = NSScreen.main?.visibleFrame.height ?? 900
        return CGSize(
            width: width,
            height: min(width / (sourceWidth / sourceHeight), screenHeight * 2 / 3)
        )
    }
}
