import SwiftUI

enum ConversationListPhase {
    case loading
    case ready
    case failed(String)
}

struct ConversationListContentState {
    let phase: ConversationListPhase
    let conversations: [ConversationListData]
    let search: ConversationSearchState
    let circles: [CircleItem]
    let currentCircleID: String?
    let canLoadMore: Bool
    let loadingMore: Bool
}

struct ConversationListActions {
    let showSidebar: (() -> Void)?
    let searchContact: () -> Void
    let create: (ConversationCreationKind) -> Void
    let select: (ConversationListData) -> Void
    let selectMessage: (MessageItem, ConversationListData?) -> Void
    let selectUser: (UserProfileItem) -> Void
    let openMao: (UserProfileItem) -> Void
    let searchUser: (String) -> Void
    let openLink: (URL) -> Void
    let loadNextPage: () -> Void
    let togglePinned: (ConversationListData) -> Void
    let toggleMuted: (ConversationListData) -> Void
    let editCircleMembership: (ConversationListData, String, Bool) -> Void
    let delete: (ConversationListData) -> Void
    let clearFilters: () -> Void
}

struct ConversationListContent: View {
    @Environment(\.mixinTheme) private var theme
    let state: ConversationListContentState
    @Binding var keyword: String
    @Binding var unseenOnly: Bool
    let selectedConversationID: String?
    let searchFocus: FocusState<Bool>.Binding
    let currentUserID: String
    let actions: ConversationListActions

    var body: some View {
        VStack(spacing: 0) {
            header
            NetworkStatusView()
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            AudioPlayerBar()
        }
    }

    private var header: some View {
        HStack(spacing: 0) {
            if let showSidebar = actions.showSidebar {
                AppIconButton(
                    systemName: "line.3.horizontal",
                    iconSize: 20,
                    help: "Show Sidebar",
                    action: showSidebar
                )
            } else {
                Spacer()
                    .frame(width: 16)
            }

            MixinSearchField(
                text: $keyword,
                focus: searchFocus,
                placeholder: unseenOnly ? "Search unread" : "Search",
                shortcutHint: "⌘K"
            )

            Spacer()
                .frame(width: 8)

            AppIconButton(
                assetName: "ConversationFilter",
                selected: unseenOnly,
                help: "Show unread conversations"
            ) {
                unseenOnly.toggle()
            }
            .accessibilityValue(unseenOnly ? "On" : "Off")

            Spacer()
                .frame(width: 4)

            AppIconMenu(systemName: "plus", iconSize: 24, help: "Create") {
                Button("Search Contact") {
                    actions.searchContact()
                }
                Button("New Conversation") {
                    actions.create(.conversation)
                }
                Button("New Group") {
                    actions.create(.group)
                }
                Button("New Circle") {
                    actions.create(.circle)
                }
            }

            Spacer()
                .frame(width: 12)
        }
        .frame(height: 64)
        .background(theme.primary)
    }

    @ViewBuilder
    private var content: some View {
        if !keyword.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            ConversationSearchResultsView(
                keyword: keyword,
                conversations: state.conversations,
                state: state.search,
                actions: ConversationSearchActions(
                    selectConversation: actions.select,
                    selectMessage: actions.selectMessage,
                    selectUser: actions.selectUser,
                    openMao: actions.openMao,
                    searchUser: actions.searchUser,
                    openLink: actions.openLink,
                    clear: actions.clearFilters
                )
            )
        } else {
            switch state.phase {
            case .loading:
                if state.conversations.isEmpty {
                    ProgressView()
                        .tint(theme.accent)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    conversationList(refreshing: true)
                }
            case .ready:
                conversationList(refreshing: false)
            case let .failed(message):
                ContentUnavailableView(
                    "Unable to load conversations",
                    systemImage: "exclamationmark.triangle",
                    description: Text(message)
                )
            }
        }
    }

    @ViewBuilder
    private func conversationList(refreshing: Bool) -> some View {
        if state.conversations.isEmpty {
            ConversationEmptyView(
                unseenOnly: unseenOnly
            )
        } else {
            AppScrollView {
                LazyVStack(spacing: 0) {
                    if refreshing {
                        loadingRow(verticalPadding: 6)
                    }

                    ForEach(state.conversations) { conversation in
                        conversationRow(conversation)
                    }

                    if state.canLoadMore || state.loadingMore {
                        loadingRow(verticalPadding: 12)
                            .onAppear(perform: actions.loadNextPage)
                    }
                }
            }
        }
    }

    private func loadingRow(verticalPadding: CGFloat) -> some View {
        ProgressView()
            .controlSize(.small)
            .tint(theme.accent)
            .frame(maxWidth: .infinity)
            .padding(.vertical, verticalPadding)
    }

    private func conversationRow(
        _ conversation: ConversationListData
    ) -> some View {
        ConversationRow(
            conversation: conversation,
            keyword: keyword,
            currentUserID: currentUserID,
            selected: selectedConversationID == conversation.conversationId,
            onSelect: {
                ConversationListDiagnostics.rowActivated(
                    conversationID: conversation.conversationId,
                    selectedConversationID: selectedConversationID
                )
                guard selectedConversationID != conversation.conversationId else {
                    return
                }
                actions.select(conversation)
            }
        )
        .onHover { hovering in
            ConversationListDiagnostics.rowHover(
                conversationID: conversation.conversationId,
                hovering: hovering,
                selectedConversationID: selectedConversationID
            )
        }
        .contextMenu {
            Button(conversation.isPinned ? "Unpin" : "Pin") {
                actions.togglePinned(conversation)
            }
            Button(conversation.isMuted ? "Unmute" : "Mute") {
                actions.toggleMuted(conversation)
            }
            Divider()
            let availableCircles = state.circles.filter {
                !conversation.circleIds.contains($0.circleId)
            }
            if !availableCircles.isEmpty {
                Menu("Add to Circle") {
                    ForEach(availableCircles, id: \.circleId) { circle in
                        Button(circle.name) {
                            actions.editCircleMembership(
                                conversation,
                                circle.circleId,
                                true
                            )
                        }
                    }
                }
                Divider()
            }
            Button("Delete Chat", role: .destructive) {
                actions.delete(conversation)
            }
            if let currentCircleID = state.currentCircleID,
               conversation.circleIds.contains(currentCircleID)
            {
                Button("Remove Chat from Circle", role: .destructive) {
                    actions.editCircleMembership(
                        conversation,
                        currentCircleID,
                        false
                    )
                }
            }
        }
    }
}

private struct ConversationEmptyView: View {
    let unseenOnly: Bool

    var body: some View {
        VStack(spacing: 24) {
            Image("ConversationEmpty")
                .resizable()
                .scaledToFit()
                .frame(width: 58, height: 78)

            Text(title)
                .font(.system(size: 14))
                .foregroundStyle(Color(red: 229 / 255, green: 233 / 255, blue: 240 / 255))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var title: String {
        if unseenOnly {
            return "No chats, contacts or messages found."
        }
        return "No Data"
    }
}
