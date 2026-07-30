import SwiftUI

struct AudioMessageView: View {
    @Environment(\.mixinTheme) private var theme
    @Environment(SettingsPreferencesModel.self) private var preferences
    @State private var coordinator = AudioPlaybackCoordinator.shared

    let message: MessageItem
    let playlist: [MessageItem]
    let mediaDirectory: URL?
    let conversationName: String?
    let outgoing: Bool
    let onDownload: () -> Void
    let onCancelTransfer: () -> Void
    let onRetryTransfer: () -> Void
    let onMarkRead: (String) -> Void

    var body: some View {
        HStack(spacing: 8) {
                Button(action: primaryAction) {
                    statusImage
                        .frame(width: 20, height: 20)
                }
                .buttonStyle(.plain)
                .help(actionHelp)

                VStack(alignment: .leading, spacing: 8) {
                    AudioWaveformView(
                        samples: message.decodedWaveform,
                        progress: playbackProgress,
                        backgroundColor: waveformBackgroundColor,
                        foregroundColor: waveformForegroundColor
                    )
                    .frame(width: 238, height: 12)

                    Text(Self.format(durationMillis))
                        .font(
                            .system(
                                size: 12 + preferences.chatFontSizeDelta
                            )
                            .monospacedDigit()
                        )
                        .foregroundStyle(.secondary)
                }
            }
        .frame(maxWidth: 266, alignment: .leading)
    }

    @ViewBuilder
    private var statusImage: some View {
        switch message.mediaStatus.uppercased() {
        case "CANCELED":
            Image(systemName: outgoing ? "arrow.clockwise" : "arrow.down.circle")
        case "PENDING":
            ProgressView()
                .controlSize(.small)
        case "EXPIRED":
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.orange)
        default:
            Image(systemName: isCurrent && coordinator.isPlaying
                ? "stop.circle.fill"
                : "play.circle.fill")
        }
    }

    private var actionHelp: String {
        switch message.mediaStatus.uppercased() {
        case "CANCELED":
            outgoing ? "Retry upload" : "Download audio"
        case "PENDING":
            "Cancel transfer"
        case "EXPIRED":
            "Audio expired"
        default:
            isCurrent && coordinator.isPlaying ? "Stop audio" : "Play audio"
        }
    }

    private var durationMillis: Int64 {
        Int64(message.mediaDuration) ?? 0
    }

    private var isCurrent: Bool {
        coordinator.currentID == message.messageId
    }

    private var playbackProgress: Double {
        guard isCurrent, coordinator.durationMillis > 0 else {
            return 0
        }
        return min(
            1,
            max(
                0,
                Double(coordinator.positionMillis)
                    / Double(coordinator.durationMillis)
            )
        )
    }

    private var usesReadWaveform: Bool {
        outgoing || message.mediaStatus.uppercased() == "READ"
    }

    private var waveformBackgroundColor: Color {
        usesReadWaveform
            ? Color(red: 221 / 255, green: 221 / 255, blue: 221 / 255)
            : theme.accent
    }

    private var waveformForegroundColor: Color {
        usesReadWaveform
            ? Color(red: 155 / 255, green: 155 / 255, blue: 155 / 255)
            : theme.accent
    }

    private func primaryAction() {
        switch message.mediaStatus.uppercased() {
        case "CANCELED":
            outgoing ? onRetryTransfer() : onDownload()
        case "PENDING":
            onCancelTransfer()
        case "DONE", "READ":
            togglePlayback()
        default:
            break
        }
    }

    private func togglePlayback() {
        guard let item = message.audioPlaybackItem(
            mediaDirectory: mediaDirectory,
            conversationName: conversationName
        ) else {
            return
        }
        let items = playlist.compactMap {
            $0.audioPlaybackItem(
                mediaDirectory: mediaDirectory,
                conversationName: conversationName
            )
        }
        Task {
            do {
                if isCurrent {
                    coordinator.stop()
                }
                try await coordinator.play(
                    item,
                    playlist: items,
                    onMarkRead: onMarkRead
                )
            } catch {
                coordinator.stop()
            }
        }
    }

    static func format(_ milliseconds: Int64) -> String {
        let seconds = max(1, milliseconds / 1_000)
        return String(format: "%02lld:%02lld", seconds / 60, seconds % 60)
    }
}

struct VoiceRecordingPreview: View {
    @Environment(\.mixinTheme) private var theme
    @State private var coordinator = AudioPlaybackCoordinator.shared

    let recording: VoiceRecordingDraft

    private var previewID: String {
        "voice-preview-\(recording.url.path)"
    }

    var body: some View {
        HStack(spacing: 0) {
            Spacer()
                .frame(width: 2)

            Button {
                togglePlayback()
            } label: {
                Image(systemName: isPlaying ? "stop.fill" : "play.fill")
                    .font(.system(size: 16, weight: .medium))
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(.plain)
            .frame(width: 40, height: 40)

            Spacer()
                .frame(width: 2)

            AudioWaveformView(
                samples: recording.waveform,
                progress: progress,
                backgroundColor: theme.secondaryText.opacity(0.35),
                foregroundColor: theme.accent
            )
            .frame(height: 20)

            Spacer()
                .frame(width: 10)

            Text(durationText)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)

            Spacer()
                .frame(width: 12)
        }
        .frame(height: 32)
        .background(theme.listSelected)
        .clipShape(RoundedRectangle(cornerRadius: 15))
        .onDisappear {
            if coordinator.currentID == previewID {
                coordinator.stop()
            }
        }
    }

    private var isPlaying: Bool {
        coordinator.currentID == previewID && coordinator.isPlaying
    }

    private var durationText: String {
        let seconds = max(0, recording.durationMillis / 1_000)
        return String(format: "%lld:%02lld", seconds / 60, seconds % 60)
    }

    private var progress: Double {
        guard coordinator.currentID == previewID,
              recording.durationMillis > 0
        else {
            return 0
        }
        return min(
            1,
            Double(coordinator.positionMillis) / Double(recording.durationMillis)
        )
    }

    private func togglePlayback() {
        if coordinator.currentID == previewID {
            coordinator.stop()
            return
        }
        Task {
            do {
                try await coordinator.playPreview(
                    id: previewID,
                    url: recording.url,
                    durationMillis: recording.durationMillis
                )
            } catch {
                coordinator.stop()
            }
        }
    }
}

private struct AudioWaveformView: View {
    let samples: [UInt8]
    let progress: Double
    let backgroundColor: Color
    let foregroundColor: Color

    var body: some View {
        GeometryReader { geometry in
            let displaySamples = reducedSamples(for: geometry.size.width)
            HStack(alignment: .bottom, spacing: 2) {
                ForEach(Array(displaySamples.enumerated()), id: \.offset) { index, sample in
                    Capsule()
                        .fill(
                            Double(index) / Double(max(1, displaySamples.count - 1)) <= progress
                                ? foregroundColor
                                : backgroundColor
                        )
                        .frame(
                            maxHeight: max(
                                2,
                                geometry.size.height * CGFloat(max(16, sample)) / 255
                            )
                        )
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
        }
        .accessibilityLabel("Audio waveform")
        .accessibilityValue("\(Int(progress * 100)) percent")
    }

    private func reducedSamples(for width: CGFloat) -> [UInt8] {
        let source = samples.isEmpty
            ? Array(repeating: UInt8(0), count: 60)
            : samples
        let count = min(source.count, max(1, Int((width + 2) / 4)))
        return (0 ..< count).map { index in
            let lower = index * source.count / count
            let upper = max(lower + 1, (index + 1) * source.count / count)
            return source[lower ..< upper].max() ?? 0
        }
    }
}

extension MessageItem {
    var decodedWaveform: [UInt8] {
        guard let mediaWaveform,
              let data = Data(base64Encoded: mediaWaveform)
        else {
            return []
        }
        return [UInt8](data)
    }

    func audioPlaybackItem(
        mediaDirectory: URL?,
        conversationName: String?
    ) -> AudioQueueItem? {
        guard let url = localMediaURL(mediaDirectory: mediaDirectory),
              FileManager.default.fileExists(atPath: url.path),
              ["DONE", "READ"].contains(mediaStatus.uppercased())
        else {
            return nil
        }
        return AudioQueueItem(
            messageID: messageId,
            conversationID: conversationId,
            conversationName: conversationName,
            url: url,
            durationMillis: Int64(mediaDuration) ?? 0,
            unread: mediaStatus.uppercased() == "DONE"
        )
    }

    private func localMediaURL(mediaDirectory: URL?) -> URL? {
        guard let source = mediaUrl?.trimmingCharacters(
            in: .whitespacesAndNewlines
        ), !source.isEmpty else {
            return nil
        }
        if let url = URL(string: source), url.isFileURL {
            return url
        }
        if source.hasPrefix("/") {
            return URL(fileURLWithPath: source)
        }
        guard !source.contains("://"), let mediaDirectory else {
            return nil
        }
        return mediaDirectory.appending(path: source, directoryHint: .notDirectory)
    }
}
