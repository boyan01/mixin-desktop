import SwiftUI

struct VoiceRecorderBar: View {
    let recorder: VoiceRecorderModel
    let sending: Bool
    let errorText: String?
    let onSend: () -> Void

    var body: some View {
        VStack(spacing: 2) {
            HStack(spacing: 12) {
                Button {
                    Task {
                        await recorder.cancel()
                    }
                } label: {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.borderless)
                .help("Discard recording")

                Group {
                    if let recording = recorder.recording {
                        VoiceRecordingPreview(recording: recording)
                    } else {
                        HStack(spacing: 6) {
                            Circle()
                                .fill(.red)
                                .frame(width: 8, height: 8)
                            Text(AudioMessageView.format(recorder.elapsedMillis))
                                .font(.body.monospacedDigit())
                        }
                        .frame(maxWidth: .infinity)
                    }
                }

                if recorder.status == .recording {
                    Button {
                        Task {
                            await recorder.stop()
                        }
                    } label: {
                        Image(systemName: "stop.circle.fill")
                            .foregroundStyle(.red)
                    }
                    .buttonStyle(.borderless)
                    .help("Stop recording")
                } else {
                    Button {
                        Task {
                            await recorder.retry()
                        }
                    } label: {
                        Image(systemName: "arrow.clockwise.circle.fill")
                    }
                    .buttonStyle(.borderless)
                    .help("Record again")
                }

                Button(action: onSend) {
                    if recorder.status == .sending || sending {
                        ProgressView()
                            .controlSize(.small)
                    } else {
                        Image(systemName: "paperplane.fill")
                    }
                }
                .buttonStyle(.borderless)
                .disabled(recorder.status == .sending || sending)
                .help("Send voice message")
            }
            if let errorText, !errorText.isEmpty {
                Text(errorText)
                    .font(.caption2)
                    .foregroundStyle(.red)
                    .lineLimit(1)
            }
        }
        .padding(.horizontal, 14)
        .frame(minHeight: 58)
        .background(.bar)
    }
}
