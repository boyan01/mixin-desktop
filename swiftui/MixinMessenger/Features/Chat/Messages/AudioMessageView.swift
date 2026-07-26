import SwiftUI

struct AudioMessageView: View {
    @State private var coordinator = AudioPlaybackCoordinator.shared

    let message: SwiftMessageItem
    let playlist: [SwiftMessageItem]
    let mediaDirectory: URL?
    let conversationName: String?
    let outgoing: Bool
    let onDownload: () -> Void
    let onCancelTransfer: () -> Void
    let onRetryTransfer: () -> Void
    let onMarkRead: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(spacing: 8) {
                Button(action: primaryAction) {
                    statusImage
                        .frame(width: 20, height: 20)
                }
                .buttonStyle(.plain)
                .help(actionHelp)

                AudioWaveformView(
                    samples: message.decodedWaveform,
                    progress: playbackProgress
                )
                .frame(width: 220, height: 18)

                Text(Self.format(durationMillis))
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }

            if isCurrent {
                HStack(spacing: 8) {
                    Slider(
                        value: Binding(
                            get: { playbackProgress },
                            set: { coordinator.seek(to: $0) }
                        ),
                        in: 0 ... 1
                    )
                    .controlSize(.small)

                    Text(Self.format(coordinator.positionMillis))
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)

                    Button("\(Int(coordinator.speed))×") {
                        coordinator.toggleSpeed()
                    }
                    .buttonStyle(.borderless)
                    .font(.caption.weight(.semibold))
                    .help("Toggle playback speed")
                }
            }
        }
        .frame(maxWidth: 280)
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
                ? "pause.circle.fill"
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
            isCurrent && coordinator.isPlaying ? "Pause audio" : "Play audio"
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
                try await coordinator.toggle(
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
    @State private var coordinator = AudioPlaybackCoordinator.shared

    let recording: VoiceRecording

    private var previewID: String {
        "voice-preview-\(recording.url.path)"
    }

    var body: some View {
        HStack(spacing: 8) {
            Button {
                togglePlayback()
            } label: {
                Image(systemName: isPlaying ? "stop.circle.fill" : "play.circle.fill")
            }
            .buttonStyle(.plain)

            AudioWaveformView(
                samples: recording.waveform,
                progress: progress
            )
            .frame(height: 18)

            Text(AudioMessageView.format(recording.durationMillis))
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 10)
        .frame(height: 34)
        .background(Color(nsColor: .selectedContentBackgroundColor).opacity(0.18))
        .clipShape(Capsule())
        .onDisappear {
            if coordinator.currentID == previewID {
                coordinator.stop()
            }
        }
    }

    private var isPlaying: Bool {
        coordinator.currentID == previewID && coordinator.isPlaying
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

    var body: some View {
        GeometryReader { geometry in
            let displaySamples = reducedSamples(for: geometry.size.width)
            HStack(alignment: .center, spacing: 2) {
                ForEach(Array(displaySamples.enumerated()), id: \.offset) { index, sample in
                    Capsule()
                        .fill(
                            Double(index) / Double(max(1, displaySamples.count - 1)) <= progress
                                ? Color.accentColor
                                : Color.secondary.opacity(0.35)
                        )
                        .frame(
                            maxHeight: max(
                                3,
                                geometry.size.height * CGFloat(max(16, sample)) / 255
                            )
                        )
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .accessibilityLabel("Audio waveform")
        .accessibilityValue("\(Int(progress * 100)) percent")
    }

    private func reducedSamples(for width: CGFloat) -> [UInt8] {
        let source = samples.isEmpty
            ? Array(repeating: UInt8(80), count: 40)
            : samples
        let count = min(source.count, max(1, Int(width / 4)))
        return (0 ..< count).map { index in
            let lower = index * source.count / count
            let upper = max(lower + 1, (index + 1) * source.count / count)
            return source[lower ..< upper].max() ?? 0
        }
    }
}

extension SwiftMessageItem {
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
    ) -> AudioPlaybackItem? {
        guard let url = localMediaURL(mediaDirectory: mediaDirectory),
              FileManager.default.fileExists(atPath: url.path),
              ["DONE", "READ"].contains(mediaStatus.uppercased())
        else {
            return nil
        }
        return AudioPlaybackItem(
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
