import CryptoKit
import Foundation
import Lottie
import SwiftUI

struct StickerLottieView: View {
    @Environment(AppModel.self) private var appModel
    @Environment(\.scenePhase) private var scenePhase
    @State private var animation: LottieAnimation?
    @State private var errorMessage: String?
    @State private var retryRevision = 0

    let urlString: String
    let onLoadFailure: () async -> Void

    var body: some View {
        Group {
            if let animation {
                LottieView(animation: animation)
                    .configure(\.contentMode, to: .scaleAspectFit)
                    .resizable()
                    .playbackMode(playbackMode)
                    .backgroundBehavior(.pauseAndRestore)
                    .windowBackgroundBehavior(.pauseAndRestore)
                    .accessibilityLabel("Animated sticker")
            } else if let errorMessage {
                Button {
                    retryRevision += 1
                } label: {
                    Image(systemName: "arrow.clockwise.circle")
                        .resizable()
                        .scaledToFit()
                        .foregroundStyle(.secondary)
                        .padding(8)
                }
                .buttonStyle(.plain)
                .help("Reload animated sticker: \(errorMessage)")
                .accessibilityLabel("Reload animated sticker")
            } else {
                ProgressView()
                    .controlSize(.small)
                    .accessibilityLabel("Loading animated sticker")
            }
        }
        .task(id: StickerLottieLoadID(
            urlString: urlString,
            retryRevision: retryRevision
        )) {
            await load(refresh: retryRevision > 0)
        }
    }

    private var playbackMode: LottiePlaybackMode {
        if scenePhase == .active {
            return .playing(
                .fromProgress(nil, toProgress: 1, loopMode: .loop)
            )
        }
        return .paused(at: .currentFrame)
    }

    private func load(refresh: Bool) async {
        animation = nil
        errorMessage = nil
        do {
            let data = try await StickerLottieDataCache.shared.data(
                for: urlString,
                refresh: refresh,
                desktop: appModel.desktopHandle
            )
            try Task.checkCancellation()
            animation = try LottieAnimation.from(data: data)
        } catch is CancellationError {
            return
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
            await onLoadFailure()
        }
    }
}

private struct StickerLottieLoadID: Hashable {
    let urlString: String
    let retryRevision: Int
}

private actor StickerLottieDataCache {
    static let shared = StickerLottieDataCache()

    private static let cacheDirectoryName = "cache_lottie"
    private static let maximumResponseSize: UInt64 = 32 * 1024 * 1024
    private var inFlight: [String: Task<Data, Error>] = [:]

    func data(
        for urlString: String,
        refresh: Bool,
        desktop: SwiftDesktopHandle?
    ) async throws -> Data {
        let fileURL = try cacheFileURL(for: urlString)
        if !refresh,
           let cached = cachedData(for: urlString, fileURL: fileURL)
        {
            return cached
        }
        if let task = inFlight[urlString] {
            return try await task.value
        }

        let task = Task<Data, Error> {
            let data = try await Self.download(urlString, desktop: desktop)
            try Self.store(data, at: fileURL)
            return data
        }
        inFlight[urlString] = task
        defer {
            inFlight[urlString] = nil
        }
        return try await task.value
    }

    private func cacheFileURL(for urlString: String) throws -> URL {
        try cacheRoot()
            .appendingPathComponent(Self.cacheDirectoryName, isDirectory: true)
            .appendingPathComponent(Self.md5(urlString))
    }

    private func cachedData(for urlString: String, fileURL: URL) -> Data? {
        if let data = try? Data(contentsOf: fileURL),
           !data.isEmpty
        {
            return data
        }
        for legacyURL in legacyCacheFileURLs(for: urlString) {
            guard let data = try? Data(contentsOf: legacyURL),
                  !data.isEmpty
            else {
                continue
            }
            try? Self.store(data, at: fileURL)
            return data
        }
        return nil
    }

    private func legacyCacheFileURLs(for urlString: String) -> [URL] {
        guard let root = try? cacheRoot() else {
            return []
        }
        let directory = root
            .appendingPathComponent(
                Bundle.main.bundleIdentifier ?? "MixinDesktop",
                isDirectory: true
            )
            .appendingPathComponent(Self.cacheDirectoryName, isDirectory: true)
        let sha256 = SHA256.hash(data: Data(urlString.utf8))
            .map { String(format: "%02x", $0) }
            .joined()
        return [
            directory.appendingPathComponent(Self.md5(urlString)),
            directory.appendingPathComponent("\(sha256).json"),
        ]
    }

    private func cacheRoot() throws -> URL {
        try FileManager.default.url(
            for: .cachesDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
    }

    nonisolated private static func md5(_ value: String) -> String {
        Insecure.MD5.hash(data: Data(value.utf8))
            .map { String(format: "%02x", $0) }
            .joined()
    }

    nonisolated private static func store(_ data: Data, at fileURL: URL) throws {
        try FileManager.default.createDirectory(
            at: fileURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: fileURL, options: .atomic)
    }

    nonisolated private static func download(
        _ urlString: String,
        desktop: SwiftDesktopHandle?
    ) async throws -> Data {
        guard let url = URL(string: urlString) else {
            throw StickerLottieError.invalidURL
        }
        if url.isFileURL {
            return try Data(contentsOf: url)
        }
        guard ["http", "https"].contains(url.scheme?.lowercased() ?? "") else {
            throw StickerLottieError.invalidURL
        }
        guard let desktop else {
            throw StickerLottieError.networkUnavailable
        }
        let response = try await desktop.httpRequest(
            method: "GET",
            url: url.absoluteString,
            headers: [:],
            body: nil,
            timeoutMillis: 60_000,
            maxResponseBytes: maximumResponseSize
        )
        guard (200 ..< 300).contains(response.statusCode) else {
            throw StickerLottieError.invalidResponse(Int(response.statusCode))
        }
        guard !response.body.isEmpty else {
            throw StickerLottieError.emptyAnimation
        }
        return response.body
    }
}

private enum StickerLottieError: LocalizedError {
    case invalidURL
    case invalidResponse(Int)
    case emptyAnimation
    case networkUnavailable

    var errorDescription: String? {
        switch self {
        case .invalidURL:
            "The sticker URL is invalid."
        case let .invalidResponse(statusCode):
            "The sticker server returned HTTP \(statusCode)."
        case .emptyAnimation:
            "The sticker animation is empty."
        case .networkUnavailable:
            "The sticker network service is unavailable."
        }
    }
}
