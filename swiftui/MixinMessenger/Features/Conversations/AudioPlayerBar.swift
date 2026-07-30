import SwiftUI

struct AudioPlayerBar: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.mixinTheme) private var theme
    @State private var coordinator = AudioPlaybackCoordinator.shared
    @State private var conversation: ConversationListData?

    var body: some View {
        if let conversationID = coordinator.currentConversationID,
           conversationID != navigation.selectedConversationID
        {
            VStack(spacing: 0) {
                HStack(spacing: 0) {
                    Button("2X") {
                        coordinator.toggleSpeed()
                    }
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(
                        coordinator.speed == 2 ? theme.accent : theme.secondaryText
                    )
                    .buttonStyle(MixinActionButtonStyle())
                    Button {
                        coordinator.isPlaying
                            ? coordinator.pause()
                            : coordinator.resume()
                    } label: {
                        Image(systemName: coordinator.isPlaying
                            ? "pause.fill"
                            : "play.fill")
                            .foregroundStyle(theme.icon)
                    }
                    .buttonStyle(MixinActionButtonStyle())
                    Spacer().frame(width: 8)
                    Button {
                        guard let conversation else {
                            return
                        }
                        navigation.selectConversation(conversation)
                    } label: {
                        HStack(spacing: 8) {
                            ZStack(alignment: .bottomTrailing) {
                                ConversationAvatar(
                                    conversation: conversation,
                                    size: 32
                                )
                                Image(systemName: "waveform")
                                    .font(.system(size: 16))
                                    .foregroundStyle(theme.icon)
                            }
                            .frame(width: 40, height: 32)
                            Text(conversation?.name ?? "")
                                .font(.system(size: 14))
                                .foregroundStyle(theme.text)
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(.plain)
                    .disabled(conversation == nil)
                    .frame(maxWidth: .infinity, alignment: .center)
                    Spacer().frame(width: 8)
                    Button {
                        coordinator.stop()
                    } label: {
                        Image(systemName: "xmark")
                            .foregroundStyle(theme.icon)
                    }
                    .buttonStyle(MixinActionButtonStyle())
                }
                .frame(height: 48)
                GeometryReader { proxy in
                    theme.accent
                        .frame(width: proxy.size.width * progress)
                }
                .frame(height: 3)
            }
            .background(theme.primary)
            .transition(.move(edge: .top).combined(with: .opacity))
            .task(id: conversationID) {
                conversation = nil
                do {
                    conversation = try await session.handle
                        .conversationItemsByIds(
                            conversationIds: [conversationID]
                        )
                        .first
                } catch {
                    AppLogger.error(
                        "Audio player conversation lookup failed: conversation_id=\(conversationID)",
                        error: error
                    )
                }
            }
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
