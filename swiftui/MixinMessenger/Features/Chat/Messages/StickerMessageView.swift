import SwiftUI

struct StickerMessageView: View {
    let message: SwiftMessageItem
    let onShowDetail: (String) -> Void

    var body: some View {
        if let url = message.presentationImageURL {
            Button {
                if let stickerID = message.stickerId {
                    onShowDetail(stickerID)
                }
            } label: {
                StickerImage(
                    urlString: url.absoluteString,
                    assetType: message.stickerAssetType,
                    stickerID: message.stickerId
                )
                .frame(maxWidth: 140, maxHeight: 140)
            }
            .buttonStyle(.plain)
            .disabled(message.stickerId == nil)
        } else {
            AttachmentFallbackMessageView(message: message)
        }
    }
}

struct AttachmentFallbackMessageView: View {
    let message: SwiftMessageItem

    var body: some View {
        Label(message.mediaName ?? message.category, systemImage: "paperclip")
    }
}

struct TransferFallbackMessageView: View {
    let message: SwiftMessageItem

    var body: some View {
        Label(
            [message.snapshotAmount, message.snapshotAssetSymbol]
                .compactMap { $0 }
                .joined(separator: " "),
            systemImage: "arrow.left.arrow.right.circle.fill"
        )
    }
}
