import SwiftUI

struct StickerImage: View {
    @Environment(AccountSession.self) private var session

    let urlString: String
    let assetType: String?
    var stickerID: String?

    var body: some View {
        switch StickerAssetKind(assetType: assetType, urlString: urlString) {
        case .lottie:
            StickerLottieView(urlString: urlString) {
                guard let stickerID, !stickerID.isEmpty else {
                    return
                }
                try? await session.handle.refreshSticker(stickerId: stickerID)
            }
        case .image:
            MixinAsyncImage(url: URL(string: urlString)) { phase in
                switch phase {
                case let .success(image):
                    image
                        .resizable()
                        .scaledToFit()
                case .failure:
                    Image(systemName: "exclamationmark.triangle")
                        .foregroundStyle(.secondary)
                case .empty:
                    ProgressView()
                        .controlSize(.small)
                }
            }
        }
    }
}

private enum StickerAssetKind {
    case lottie
    case image

    init(assetType: String?, urlString: String) {
        let normalizedType = assetType?
            .split(separator: ";", maxSplits: 1)
            .first?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        if let normalizedType, !normalizedType.isEmpty {
            self = Self.lottieTypes.contains(normalizedType) ? .lottie : .image
            return
        }
        if URL(string: urlString)?.pathExtension.lowercased() == "json" {
            self = .lottie
        } else {
            self = .image
        }
    }

    private static let lottieTypes: Set<String> = [
        "json",
        "lottie",
        "application/json",
        "application/lottie+json",
        "application/vnd.lottie+json",
    ]
}
