import Foundation
import Observation

struct AudioQueueItem: Hashable {
    let messageID: String
    let conversationID: String?
    let conversationName: String?
    let url: URL
    let durationMillis: Int64
    let unread: Bool

    init(
        messageID: String,
        conversationID: String? = nil,
        conversationName: String? = nil,
        url: URL,
        durationMillis: Int64,
        unread: Bool
    ) {
        self.messageID = messageID
        self.conversationID = conversationID
        self.conversationName = conversationName
        self.url = url
        self.durationMillis = durationMillis
        self.unread = unread
    }
}

@MainActor
@Observable
final class AudioPlaybackCoordinator {
    static let shared = AudioPlaybackCoordinator()

    private(set) var currentID: String?
    private(set) var currentConversationID: String?
    private(set) var currentConversationName: String?
    private(set) var positionMillis: Int64 = 0
    private(set) var durationMillis: Int64 = 0
    private(set) var isPlaying = false
    private(set) var speed: Float = 1

    private var media: SwiftMediaHandle?
    private var subscription: SwiftMediaPlaybackSubscription?
    private var subscriptionTask: Task<Void, Never>?
    private var positionTask: Task<Void, Never>?
    private var playlistByID: [String: AudioQueueItem] = [:]
    private var onMarkRead: ((String) -> Void)?
    private var markedRead = Set<String>()
    private var anchorPositionMillis: Int64 = 0
    private var anchorInstant: ContinuousClock.Instant?

    func bind(_ media: SwiftMediaHandle) {
        guard self.media !== media else {
            return
        }
        subscription?.cancel()
        subscriptionTask?.cancel()
        self.media = media
        let subscription = media.subscribeAudioPlayback()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled, let event = await subscription.next() {
                guard let self else {
                    return
                }
                handle(event)
            }
        }
        startPositionUpdates()
        apply(media.audioPlaybackSnapshot())
    }

    func play(
        _ item: AudioQueueItem,
        playlist: [AudioQueueItem] = [],
        onMarkRead: ((String) -> Void)? = nil
    ) async throws {
        guard let media else {
            throw MediaPlaybackError.unavailable
        }
        var items = playlist
        var startIndex = items.firstIndex(of: item)
        if startIndex == nil {
            items = [item]
            startIndex = 0
        }
        playlistByID = Dictionary(
            uniqueKeysWithValues: items.map { ($0.messageID, $0) }
        )
        self.onMarkRead = onMarkRead
        markedRead.removeAll()
        try await media.playAudio(
            playlist: items.map(\.mediaItem),
            startIndex: UInt64(startIndex ?? 0)
        )
    }

    func playPreview(
        id: String,
        url: URL,
        durationMillis: Int64
    ) async throws {
        try await play(AudioQueueItem(
            messageID: id,
            url: url,
            durationMillis: durationMillis,
            unread: false
        ))
    }

    func toggle(
        _ item: AudioQueueItem,
        playlist: [AudioQueueItem],
        onMarkRead: ((String) -> Void)? = nil
    ) async throws {
        if currentID == item.messageID {
            if isPlaying {
                try await media?.pauseAudio()
            } else {
                try await media?.resumeAudio()
            }
        } else {
            try await play(item, playlist: playlist, onMarkRead: onMarkRead)
        }
    }

    func pause() {
        Task {
            do {
                try await media?.pauseAudio()
            } catch {
                _ = MixinErrorPresenter.message(for: error)
            }
        }
    }

    func resume() {
        Task {
            do {
                try await media?.resumeAudio()
            } catch {
                _ = MixinErrorPresenter.message(for: error)
            }
        }
    }

    func seek(to fraction: Double) {
        let fraction = max(0, min(1, fraction))
        media?.seekAudio(
            positionMillis: UInt64(Double(max(0, durationMillis)) * fraction)
        )
    }

    func toggleSpeed() {
        let next: Float = speed == 1 ? 2 : 1
        Task {
            do {
                try await media?.setAudioSpeed(speed: Double(next))
            } catch {
                _ = MixinErrorPresenter.message(for: error)
            }
        }
    }

    func stop() {
        media?.stopAudio()
        playlistByID = [:]
        onMarkRead = nil
        markedRead.removeAll()
        clearPlayback()
    }

    private func handle(_ event: MediaPlaybackEvent) {
        switch event {
        case let .changed(snapshot):
            apply(snapshot)
        case .finished:
            break
        case let .failed(_, message):
            _ = MixinErrorPresenter.message(
                for: MediaPlaybackError.failed(message)
            )
            clearPlayback()
        }
    }

    private func apply(_ snapshot: AudioPlaybackSnapshot) {
        let item = snapshot.item.flatMap { playlistByID[$0.id] }
        currentID = snapshot.item?.id
        currentConversationID = item?.conversationID
        currentConversationName = item?.conversationName
        positionMillis = Int64(clamping: snapshot.positionMillis)
        anchorPositionMillis = positionMillis
        durationMillis = Int64(clamping: snapshot.durationMillis)
        speed = Float(snapshot.speed)
        isPlaying = snapshot.status == .playing
        anchorInstant = isPlaying ? .now : nil
        if let item, item.unread, markedRead.insert(item.messageID).inserted {
            onMarkRead?(item.messageID)
        }
    }

    private func startPositionUpdates() {
        positionTask?.cancel()
        positionTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(50))
                guard let self, isPlaying, let anchorInstant else {
                    continue
                }
                let elapsed = anchorInstant.duration(to: .now)
                let elapsedMillis =
                    Int64(elapsed.components.seconds) * 1_000
                    + Int64(elapsed.components.attoseconds / 1_000_000_000_000_000)
                positionMillis = min(
                    durationMillis,
                    anchorPositionMillis + Int64(Double(elapsedMillis) * Double(speed))
                )
            }
        }
    }

    private func clearPlayback() {
        currentID = nil
        currentConversationID = nil
        currentConversationName = nil
        positionMillis = 0
        anchorPositionMillis = 0
        durationMillis = 0
        anchorInstant = nil
        isPlaying = false
    }
}

private extension AudioQueueItem {
    var mediaItem: AudioPlaybackItem {
        AudioPlaybackItem(
            id: messageID,
            path: url.path,
            durationMillis: UInt64(max(0, durationMillis))
        )
    }
}

private enum MediaPlaybackError: LocalizedError {
    case unavailable
    case failed(String)

    var errorDescription: String? {
        switch self {
        case .unavailable:
            "Audio playback is unavailable."
        case let .failed(message):
            message
        }
    }
}
