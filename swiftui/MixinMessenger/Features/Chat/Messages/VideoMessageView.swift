import AppKit
import SwiftUI

struct VideoMessageView: View {
    let message: MessageItem
    let outgoing: Bool
    let progress: () -> Double
    let onAttachmentAction: () -> Void
    let onOpen: () -> Void

    var body: some View {
        VideoMessageLayout(
            mediaWidth: message.mediaWidth,
            mediaHeight: message.mediaHeight
        ) {
        ZStack {
            MessageMediaImage(
                source: message.thumbUrl,
                thumbnail: message.thumbImage,
                contentMode: .fill
            )
            .clipped()

            AttachmentStatusOverlay(
                message: message,
                outgoing: outgoing,
                progress: progress,
                completedSymbol: "play.fill"
            )

            if !message.mediaDuration.isEmpty {
                Text(AudioMessageView.format(Int64(message.mediaDuration) ?? 0))
                    .font(.system(size: 10))
                    .padding(4)
                    .background(.black.opacity(0.3), in: RoundedRectangle(cornerRadius: 5))
                    .foregroundStyle(.white)
                    .frame(
                        maxWidth: .infinity,
                        maxHeight: .infinity,
                        alignment: .topLeading
                    )
                    .padding(.top, 6)
                    .padding(.leading, outgoing ? 6 : 14)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture(perform: mediaTap)
        }
    }

    private func mediaTap() {
        switch message.mediaStatus.uppercased() {
        case "DONE", "READ":
            onOpen()
        case "CANCELED", "PENDING":
            onAttachmentAction()
        default:
            if message.category.hasSuffix("_LIVE"),
               let rawURL = message.mediaUrl,
               let url = URL(string: rawURL),
               ["http", "https"].contains(url.scheme?.lowercased() ?? "") {
                NSWorkspace.shared.open(url)
            }
        }
    }
}

private struct VideoMessageLayout: Layout {
    let mediaWidth: Int32?
    let mediaHeight: Int32?

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews _: Subviews,
        cache _: inout ()
    ) -> CGSize {
        resolvedMediaSize(availableWidth: proposal.width)
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache _: inout ()
    ) {
        let size = resolvedMediaSize(availableWidth: proposal.width ?? bounds.width)
        for subview in subviews {
            subview.place(
                at: bounds.origin,
                anchor: .topLeading,
                proposal: ProposedViewSize(width: size.width, height: size.height)
            )
        }
    }

    private func resolvedMediaSize(availableWidth: CGFloat?) -> CGSize {
        let fallback: CGFloat = 200
        let sourceWidth = max(1, CGFloat(mediaWidth ?? Int32(fallback)))
        let sourceHeight = max(1, CGFloat(mediaHeight ?? Int32(fallback)))
        let maximumWidth = min((availableWidth ?? 340) * 0.6, fallback)
        let width = min(sourceWidth, maximumWidth)
        return CGSize(width: width, height: width / (sourceWidth / sourceHeight))
    }
}
