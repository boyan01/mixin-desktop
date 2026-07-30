import SwiftUI

struct StickerMessageView: View {
    let message: MessageItem
    let onShowDetail: (String) -> Void
    @Environment(\.displayScale) private var displayScale
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        let size = stickerSize
        if let url = stickerURL {
            Button {
                if let stickerID = message.stickerId {
                    onShowDetail(stickerID)
                }
            } label: {
                ZStack {
                    Rectangle().fill(theme.stickerPlaceholder)
                    StickerImage(
                        urlString: url.absoluteString,
                        assetType: message.stickerAssetType,
                        stickerID: message.stickerId
                    )
                }
                .frame(width: size.width, height: size.height)
            }
            .buttonStyle(.plain)
            .disabled(message.stickerId == nil)
        } else {
            Rectangle()
                .fill(theme.stickerPlaceholder)
                .frame(width: size.width, height: size.height)
        }
    }

    private var stickerSize: CGSize {
        let scale = max(displayScale, 1)
        let sourceWidth = CGFloat(message.stickerAssetWidth ?? message.mediaWidth ?? 0) / scale
        let sourceHeight = CGFloat(message.stickerAssetHeight ?? message.mediaHeight ?? 0) / scale
        let minimum: CGFloat = 60
        let maximum: CGFloat = 140

        guard sourceWidth > 0, sourceHeight > 0 else {
            return CGSize(width: maximum, height: maximum)
        }
        let size: CGSize
        let scaleFactor: CGFloat
        if sourceWidth < minimum {
            let factor = minimum / sourceWidth
            size = factor * sourceHeight > maximum
                ? CGSize(width: maximum, height: factor * sourceHeight)
                : CGSize(width: minimum, height: factor * sourceHeight)
            scaleFactor = factor
        } else if sourceHeight < minimum {
            let factor = minimum / sourceHeight
            size = factor * sourceWidth > maximum
                ? CGSize(width: factor * sourceWidth, height: maximum)
                : CGSize(width: factor * sourceWidth, height: minimum)
            scaleFactor = factor
        } else if sourceWidth > maximum || sourceHeight > maximum {
            let factor = maximum / max(sourceWidth, sourceHeight)
            size = CGSize(width: sourceWidth * factor, height: sourceHeight * factor)
            scaleFactor = factor
        } else {
            size = CGSize(width: sourceWidth, height: sourceHeight)
            scaleFactor = 1
        }
        guard scaleFactor <= 0.5,
              displayScale <= 1.5,
              message.stickerAssetType?.lowercased() != "json",
              max(size.width, size.height) >= maximum
        else {
            return size
        }
        return CGSize(width: size.width * 200 / 140, height: size.height * 200 / 140)
    }

    private var stickerURL: URL? {
        [message.stickerAssetUrl ?? "", message.mediaUrl ?? ""]
            .filter { !$0.isEmpty }
            .compactMap(URL.init(string:))
            .first
    }
}

struct AttachmentFallbackMessageView: View {
    let message: MessageItem

    var body: some View {
        Label(message.mediaName ?? message.category, systemImage: "paperclip")
    }
}

struct TransferFallbackMessageView: View {
    let message: MessageItem

    var body: some View {
        Label(
            [message.snapshotAmount, message.snapshotAssetSymbol]
                .compactMap { $0 }
                .joined(separator: " "),
            systemImage: "arrow.left.arrow.right.circle.fill"
        )
    }
}
