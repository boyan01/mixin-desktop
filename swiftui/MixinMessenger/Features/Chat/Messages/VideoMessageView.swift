import AppKit
import SwiftUI

struct VideoMessageView: View {
    let message: SwiftMessageItem
    let outgoing: Bool
    let progress: () -> Double
    let onAttachmentAction: () -> Void
    let onOpen: () -> Void

    var body: some View {
        ZStack {
            MessageMediaImage(
                source: message.thumbUrl,
                thumbnail: message.thumbImage,
                contentMode: .fill
            )
            .frame(width: mediaSize.width, height: mediaSize.height)
            .clipped()

            AttachmentStatusOverlay(
                message: message,
                outgoing: outgoing,
                progress: progress,
                completedSymbol: "play.fill"
            )

            if !message.mediaDuration.isEmpty {
                Text(message.mediaDuration)
                    .font(.caption2)
                    .padding(4)
                    .background(.black.opacity(0.3), in: RoundedRectangle(cornerRadius: 5))
                    .foregroundStyle(.white)
                    .frame(
                        maxWidth: .infinity,
                        maxHeight: .infinity,
                        alignment: .topLeading
                    )
                    .padding(6)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture(perform: mediaTap)
    }

    private var mediaSize: CGSize {
        message.scaledMediaSize(
            maximum: CGSize(width: 200, height: 260),
            minimumWidth: 120
        )
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
