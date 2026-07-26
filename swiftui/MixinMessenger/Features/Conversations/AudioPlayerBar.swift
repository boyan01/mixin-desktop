import SwiftUI

struct AudioPlayerBar: View {
    @Environment(HomeNavigationModel.self) private var navigation
    @State private var coordinator = AudioPlaybackCoordinator.shared

    var body: some View {
        if let conversationID = coordinator.currentConversationID,
           conversationID != navigation.selectedConversationID
        {
            VStack(spacing: 0) {
                HStack(spacing: 10) {
                    Button("\(Int(coordinator.speed))×") {
                        coordinator.toggleSpeed()
                    }
                    .buttonStyle(.borderless)
                    .foregroundStyle(
                        coordinator.speed == 2 ? Color.accentColor : .secondary
                    )
                    Button {
                        coordinator.isPlaying
                            ? coordinator.pause()
                            : coordinator.resume()
                    } label: {
                        Image(systemName: coordinator.isPlaying
                            ? "pause.fill"
                            : "play.fill")
                    }
                    .buttonStyle(.borderless)
                    Button {
                        navigation.selectConversation(
                            conversationID,
                            name: coordinator.currentConversationName
                        )
                    } label: {
                        HStack(spacing: 8) {
                            Image(systemName: "waveform.circle.fill")
                                .font(.title2)
                        Text(coordinator.currentConversationName ?? "Audio message")
                            .lineLimit(1)
                        }
                    }
                    .buttonStyle(.plain)
                    .frame(maxWidth: .infinity, alignment: .center)
                    Spacer(minLength: 0)
                    Button {
                        coordinator.stop()
                    } label: {
                        Image(systemName: "xmark")
                    }
                    .buttonStyle(.borderless)
                }
                .padding(.horizontal, 12)
                .frame(height: 48)
                ProgressView(value: progress)
                    .progressViewStyle(.linear)
                    .controlSize(.small)
                    .frame(height: 3)
            }
            .background(.bar)
            .transition(.move(edge: .top).combined(with: .opacity))
        }
    }

    private var progress: Double {
        guard coordinator.durationMillis > 0 else {
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
}
