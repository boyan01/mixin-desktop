import SwiftUI

struct VoiceRecorderBar: View {
    @Environment(\.mixinTheme) private var theme
    let recorder: VoiceRecorderModel
    let sending: Bool
    let errorText: String?
    let onSend: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                Button {
                    Task {
                        await recorder.cancel()
                    }
                } label: {
                    Image("RecordClose")
                        .renderingMode(.template)
                        .resizable()
                        .frame(width: 24, height: 24)
                        .foregroundStyle(theme.icon)
                }
                .buttonStyle(MixinActionButtonStyle())
                .help("Discard recording")

                Group {
                    if let recording = recorder.recording {
                        VoiceRecordingPreview(recording: recording)
                    } else {
                        HStack(spacing: 4) {
                            Circle()
                                .fill(Color(red: 229 / 255, green: 120 / 255, blue: 116 / 255))
                                .frame(width: 8, height: 8)
                            Text(AudioMessageView.format(recorder.elapsedMillis))
                                .font(.system(size: 14).monospacedDigit())
                                .foregroundStyle(theme.text)
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
                .padding(.horizontal, 16)
                .frame(maxWidth: .infinity)

                if recorder.status == .recording {
                    Button {
                        Task {
                            await recorder.stop()
                        }
                    } label: {
                        Image("RecordStop")
                            .renderingMode(.template)
                            .resizable()
                            .frame(width: 24, height: 24)
                            .foregroundStyle(theme.accent)
                    }
                    .buttonStyle(MixinActionButtonStyle())
                    .help("Stop recording")
                } else {
                    Button {
                        Task {
                            await recorder.retry()
                        }
                    } label: {
                        Image("RecordRetry")
                            .renderingMode(.template)
                            .resizable()
                            .frame(width: 24, height: 24)
                            .foregroundStyle(theme.icon)
                    }
                    .buttonStyle(MixinActionButtonStyle())
                    .help("Record again")
                }

                Button(action: onSend) {
                    if recorder.status == .sending || sending {
                        ProgressView()
                            .controlSize(.small)
                    } else {
                        Image("ComposerSend")
                            .renderingMode(.template)
                            .resizable()
                            .frame(width: 24, height: 24)
                            .foregroundStyle(theme.icon)
                    }
                }
                .buttonStyle(MixinActionButtonStyle())
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
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(minHeight: 56)
        .background(theme.primary)
    }
}
