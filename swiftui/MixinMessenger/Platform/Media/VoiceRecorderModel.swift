import AVFoundation
import Observation

struct VoiceRecordingDraft: Sendable {
    let url: URL
    let durationMillis: Int64
    let waveform: [UInt8]
}

@MainActor
@Observable
final class VoiceRecorderModel {
    enum Status {
        case idle
        case recording
        case recorded
        case sending
    }

    private(set) var status: Status = .idle
    private(set) var elapsedMillis: Int64 = 0
    private(set) var recording: VoiceRecordingDraft?
    private(set) var errorMessage: String?

    private var media: SwiftMediaHandle?
    private var recorderSubscription: SwiftMediaRecorderSubscription?
    private var recorderTask: Task<Void, Never>?
    private var elapsedTask: Task<Void, Never>?
    private var startedAt: ContinuousClock.Instant?

    func start(using media: SwiftMediaHandle) async {
        guard status != .recording, status != .sending else {
            return
        }
        bind(media)
        discardRecordedFile()
        errorMessage = nil
        guard await microphoneAccessGranted() else {
            errorMessage = "Microphone access is required to record a voice message."
            return
        }

        do {
            try await media.startVoiceRecording()
            elapsedMillis = 0
            recording = nil
            status = .recording
            startedAt = .now
            startElapsedUpdates()
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
            status = .idle
        }
    }

    func stop() async {
        guard status == .recording, let media else {
            return
        }
        stopElapsedUpdates()
        do {
            apply(try await media.stopVoiceRecording())
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
            status = .idle
        }
    }

    func retry() async {
        guard let media else {
            return
        }
        await start(using: media)
    }

    func cancel() async {
        stopElapsedUpdates()
        if status == .recording {
            do {
                try await media?.cancelVoiceRecording()
            } catch {
                errorMessage = MixinErrorPresenter.message(for: error)
            }
        }
        discardRecordedFile()
        status = .idle
        elapsedMillis = 0
        errorMessage = nil
    }

    func send(
        _ operation: (VoiceRecordingDraft) async -> Bool
    ) async -> Bool {
        if status == .recording {
            await stop()
        }
        guard let recording else {
            return false
        }
        status = .sending
        if await operation(recording) {
            try? FileManager.default.removeItem(at: recording.url)
            self.recording = nil
            elapsedMillis = 0
            status = .idle
            return true
        }
        status = .recorded
        return false
    }

    func clearError() {
        errorMessage = nil
    }

    func dispose() {
        stopElapsedUpdates()
        recorderSubscription?.cancel()
        recorderSubscription = nil
        recorderTask?.cancel()
        recorderTask = nil
        if status == .recording, let media {
            Task {
                try? await media.cancelVoiceRecording()
            }
        }
        discardRecordedFile()
        status = .idle
    }

    private func bind(_ media: SwiftMediaHandle) {
        guard self.media !== media else {
            return
        }
        recorderSubscription?.cancel()
        recorderTask?.cancel()
        self.media = media
        let subscription = media.subscribeVoiceRecorder()
        recorderSubscription = subscription
        recorderTask = Task { [weak self] in
            while !Task.isCancelled, let event = await subscription.next() {
                guard let self else {
                    return
                }
                switch event {
                case let .changed(snapshot):
                    if snapshot.status == .recorded,
                       let recording = snapshot.recording,
                       status == .recording
                    {
                        stopElapsedUpdates()
                        apply(recording)
                    }
                case let .failed(message):
                    stopElapsedUpdates()
                    errorMessage = message
                    status = .idle
                }
            }
        }
    }

    private func apply(_ result: VoiceRecording) {
        let recording = VoiceRecordingDraft(
            url: URL(fileURLWithPath: result.path),
            durationMillis: Int64(clamping: result.durationMillis),
            waveform: Array(result.waveform)
        )
        guard recording.durationMillis > 0, !recording.waveform.isEmpty else {
            errorMessage = "The recorded voice message is empty."
            status = .idle
            return
        }
        self.recording = recording
        elapsedMillis = recording.durationMillis
        status = .recorded
    }

    private func startElapsedUpdates() {
        elapsedTask?.cancel()
        elapsedTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(100))
                guard let self, let startedAt else {
                    return
                }
                let elapsed = startedAt.duration(to: .now)
                elapsedMillis = min(
                    60_000,
                    Int64(elapsed.components.seconds) * 1_000
                        + Int64(elapsed.components.attoseconds / 1_000_000_000_000_000)
                )
            }
        }
    }

    private func stopElapsedUpdates() {
        elapsedTask?.cancel()
        elapsedTask = nil
        startedAt = nil
    }

    private func discardRecordedFile() {
        if let recording {
            try? FileManager.default.removeItem(at: recording.url)
        }
        recording = nil
    }

    private func microphoneAccessGranted() async -> Bool {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized:
            true
        case .notDetermined:
            await AVCaptureDevice.requestAccess(for: .audio)
        case .denied, .restricted:
            false
        @unknown default:
            false
        }
    }
}
