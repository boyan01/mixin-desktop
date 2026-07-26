import Observation
import SwiftUI

struct ContentView: View {
    @State private var model = ConversationListModel()

    var body: some View {
        NavigationSplitView {
            VStack(spacing: 0) {
                header
                Divider()
                content
            }
            .navigationSplitViewColumnWidth(min: 260, ideal: 300, max: 340)
        } detail: {
            Text(model.selectedConversation?.name ?? "Pick a conversation")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task {
            await model.load()
        }
    }

    private var header: some View {
        HStack(spacing: 12) {
            Text("Conversations")
                .font(.headline)
            Spacer()
            ProgressView()
                .controlSize(.small)
                .help("Native loading animation performance probe")
        }
        .padding(.horizontal, 16)
        .frame(height: 56)
    }

    @ViewBuilder
    private var content: some View {
        switch model.state {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .ready:
            if model.conversations.isEmpty {
                ContentUnavailableView(
                    "No conversations yet",
                    systemImage: "bubble.left.and.bubble.right"
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(model.conversations) { conversation in
                            ConversationRow(
                                conversation: conversation,
                                selected: model.selectedConversationID == conversation.id
                            )
                            .contentShape(Rectangle())
                            .onTapGesture {
                                model.selectedConversationID = conversation.id
                            }
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
        case .signedOut:
            ContentUnavailableView(
                "Sign in required",
                systemImage: "person.crop.circle.badge.exclamationmark",
                description: Text("Sign in from the existing desktop app first.")
            )
        case let .failed(message):
            ContentUnavailableView(
                "Unable to load conversations",
                systemImage: "exclamationmark.triangle",
                description: Text(message)
            )
        }
    }
}

private struct ConversationRow: View {
    let conversation: SwiftConversationListItem
    let selected: Bool

    var body: some View {
        HStack(spacing: 12) {
            ConversationAvatar(conversation: conversation)
            VStack(spacing: 8) {
                HStack(spacing: 8) {
                    Text(conversation.name)
                        .font(.system(size: 16))
                        .lineLimit(1)
                    Spacer(minLength: 8)
                    Text(conversation.formattedTime)
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                }
                HStack(spacing: 8) {
                    Text(conversation.preview.isEmpty ? "No messages yet" : conversation.preview)
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                    Spacer(minLength: 4)
                    if conversation.mentionCount > 0 {
                        UnreadBadge(text: "@", muted: false)
                    }
                    if conversation.unseenCount > 0 {
                        UnreadBadge(
                            text: String(conversation.unseenCount),
                            muted: conversation.isMuted
                        )
                    } else if conversation.isPinned {
                        Image(systemName: "pin.fill")
                            .font(.system(size: 11))
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .padding(.horizontal, 12)
        .frame(height: 78)
        .background {
            RoundedRectangle(cornerRadius: 8)
                .fill(selected ? Color.accentColor.opacity(0.14) : .clear)
                .padding(.horizontal, 8)
        }
    }
}

private struct ConversationAvatar: View {
    let conversation: SwiftConversationListItem

    var body: some View {
        AsyncImage(url: URL(string: conversation.iconUrl)) { image in
            image.resizable().scaledToFill()
        } placeholder: {
            ZStack {
                Color.accentColor.opacity(0.16)
                Text(conversation.initial)
                    .font(.headline)
                    .foregroundStyle(Color.accentColor)
            }
        }
        .frame(width: 50, height: 50)
        .clipShape(Circle())
    }
}

private struct UnreadBadge: View {
    let text: String
    let muted: Bool

    var body: some View {
        Text(text)
            .font(.system(size: 12))
            .foregroundStyle(.white)
            .padding(.horizontal, 6)
            .frame(minWidth: 26, minHeight: 20)
            .background(muted ? Color.secondary : Color.accentColor)
            .clipShape(Capsule())
    }
}

@MainActor
@Observable
final class ConversationListModel {
    enum State {
        case loading
        case ready
        case signedOut
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var conversations: [SwiftConversationListItem] = []
    var selectedConversationID: String?
    private var desktop: SwiftDesktopHandle?
    private var account: SwiftAccountHandle?

    var selectedConversation: SwiftConversationListItem? {
        conversations.first { $0.id == selectedConversationID }
    }

    func load() async {
        state = .loading
        do {
            let desktop = try await openDesktop()
            let account = try await desktop.restoreAccount()
            conversations = try await account.conversations(
                category: "chats",
                circleId: nil,
                keyword: "",
                unseenOnly: false,
                limit: 15,
                offset: 0
            )
            self.desktop = desktop
            self.account = account
            state = .ready
        } catch SwiftClientError.NotFound {
            conversations = []
            state = .signedOut
        } catch {
            state = .failed(error.localizedDescription)
        }
    }
}

extension SwiftConversationListItem: Identifiable {
    public var id: String {
        conversationId
    }

    var preview: String {
        lastMessage
    }

    var initial: String {
        name.first.map { String($0).uppercased() } ?? "?"
    }

    var formattedTime: String {
        let date = Date(timeIntervalSince1970: TimeInterval(updatedAtMillis) / 1_000)
        if Calendar.current.isDateInToday(date) {
            return date.formatted(date: .omitted, time: .shortened)
        }
        return date.formatted(.dateTime.month(.twoDigits).day(.twoDigits))
    }
}

#Preview {
    ContentView()
        .frame(width: 960, height: 640)
}
