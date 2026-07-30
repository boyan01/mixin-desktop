import AppKit
import SwiftUI

struct ConversationListView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationListModel()
    @State private var searchModel = ConversationSearchModel()
    @State private var keyword = ""
    @State private var unseenOnly = false
    @State private var pendingMute: ConversationListData?
    @State private var pendingDelete: ConversationListData?
    @State private var searchProfileUserID: String?
    @State private var searchContactPresented = false
    @FocusState private var searchFocused: Bool

    var onShowSidebar: (() -> Void)?

    var body: some View {
        ConversationListContent(
            state: contentState,
            keyword: $keyword,
            unseenOnly: $unseenOnly,
            selectedConversationID: navigation.selectedConversationID,
            searchFocus: $searchFocused,
            currentUserID: session.profile.userId,
            actions: contentActions
        )
        .background(theme.primary)
        .navigationSplitViewColumnWidth(min: 260, ideal: 300, max: 340)
        .task(id: ConversationListQuery(
            section: navigation.section,
            keyword: keyword,
            unseenOnly: unseenOnly
        )) {
            await reload()
        }
        .onDisappear {
            model.stop()
        }
        .onChange(of: navigation.searchRequest) {
            searchFocused = true
        }
        .onChange(of: model.conversations) {
            navigation.updateConversationOrder(model.conversations)
        }
        .onChange(of: navigation.selectedConversationID) {
            ConversationListDiagnostics.selectionDidChange(
                selectedConversationID: navigation.selectedConversationID,
                route: String(describing: navigation.routePath)
            )
        }
        .onChange(of: navigation.conversationCommandRequest?.revision) {
            handleConversationCommand()
        }
        .confirmationDialog(
            "Mute Notifications",
            isPresented: mutePresented,
            presenting: pendingMute
        ) { conversation in
            Button("1 Hour") {
                mute(conversation, duration: 60 * 60)
            }
            Button("8 Hours") {
                mute(conversation, duration: 8 * 60 * 60)
            }
            Button("1 Week") {
                mute(conversation, duration: 7 * 24 * 60 * 60)
            }
            Button("1 Year") {
                mute(conversation, duration: 365 * 24 * 60 * 60)
            }
            Button("Cancel", role: .cancel) {
                pendingMute = nil
            }
        } message: { conversation in
            Text("Mute notifications from \(conversation.name)?")
        }
        .alert(
            "Delete Chat?",
            isPresented: deletePresented,
            presenting: pendingDelete
        ) { conversation in
            Button("Delete", role: .destructive) {
                delete(conversation)
            }
            Button("Cancel", role: .cancel) {
                pendingDelete = nil
            }
        } message: { conversation in
            Text("Delete \(conversation.name) and its local chat history?")
        }
        .alert(
            "Conversation Action Failed",
            isPresented: operationErrorPresented
        ) {
            Button("OK") {
                model.dismissOperationError()
            }
        } message: {
            Text(model.operationError ?? "")
        }
        .sheet(
            isPresented: Binding(
                get: { searchProfileUserID != nil },
                set: { if !$0 { searchProfileUserID = nil } }
            )
        ) {
            if let searchProfileUserID {
                NavigationStack {
                    MessageUserProfileView(userID: searchProfileUserID)
                }
                .frame(minWidth: 420, minHeight: 560)
            }
        }
        .sheet(isPresented: $searchContactPresented) {
            SearchContactSheet()
        }
    }

    private var contentState: ConversationListContentState {
        ConversationListContentState(
            phase: model.state,
            conversations: model.conversations,
            search: searchModel.state,
            circles: model.circles,
            currentCircleID: navigation.section.conversationFilter.circleID,
            canLoadMore: model.canLoadMore,
            loadingMore: model.loadingMore
        )
    }

    private var contentActions: ConversationListActions {
        ConversationListActions(
            showSidebar: onShowSidebar,
            searchContact: {
                searchContactPresented = true
            },
            create: navigation.showCreation,
            select: { conversation in
                ConversationListDiagnostics.selectionWillChange(
                    from: navigation.selectedConversationID,
                    to: conversation.conversationId
                )
                navigation.selectConversation(conversation)
                keyword = ""
            },
            selectMessage: { message, conversation in
                keyword = ""
                navigation.locateMessage(
                    conversationID: message.conversationId,
                    messageID: message.messageId,
                    conversationName: conversation?.name
                )
            },
            selectUser: { user in
                searchProfileUserID = user.userId
            },
            openMao: { user in
                guard let url = URL(
                    string: "mixin://apps/\(user.userId)?action=open"
                ) else {
                    return
                }
                Task {
                    await navigation.open(url, account: session.handle)
                }
            },
            searchUser: { value in
                Task {
                    do {
                        let user = try await session.handle.searchUser(query: value)
                        searchProfileUserID = user.userId
                    } catch {
                        AppLogger.error(
                            "Conversation user search failed: query=\(value)",
                            error: error
                        )
                        model.presentOperationError(error)
                    }
                }
            },
            openLink: { url in
                BotWebViewWindow.open(
                    url: url,
                    title: "",
                    conversationID: "",
                    currency: session.profile.fiatCurrency
                )
            },
            loadNextPage: {
                Task {
                    await model.loadNextPage()
                }
            },
            togglePinned: { conversation in
                Task {
                    await model.setPinned(
                        conversation,
                        pinned: !conversation.isPinned
                    )
                }
            },
            toggleMuted: requestMute,
            editCircleMembership: { conversation, circleID, add in
                Task {
                    await model.editCircleMembership(
                        conversation,
                        circleID: circleID,
                        add: add
                    )
                }
            },
            delete: { conversation in
                pendingDelete = conversation
            },
            clearFilters: clearFilters
        )
    }

    private var deletePresented: Binding<Bool> {
        Binding(
            get: { pendingDelete != nil },
            set: { presented in
                if !presented {
                    pendingDelete = nil
                }
            }
        )
    }

    private var mutePresented: Binding<Bool> {
        Binding(
            get: { pendingMute != nil },
            set: { presented in
                if !presented {
                    pendingMute = nil
                }
            }
        )
    }

    private var operationErrorPresented: Binding<Bool> {
        Binding(
            get: { model.operationError != nil },
            set: { presented in
                if !presented {
                    model.dismissOperationError()
                }
            }
        )
    }

    private func reload() async {
        await model.start(account: session.handle)
        if !keyword.isEmpty {
            try? await Task.sleep(for: .milliseconds(150))
            guard !Task.isCancelled else {
                return
            }
        }
        await model.load(
            account: session.handle,
            section: navigation.section,
            keyword: keyword,
            unseenOnly: unseenOnly
        )
        await searchModel.search(
            account: session.handle,
            query: keyword,
            category: navigation.section.conversationFilter.category,
            enabled: !unseenOnly
        )
    }

    private func handleConversationCommand() {
        guard let request = navigation.conversationCommandRequest else {
            return
        }
        guard let conversation = navigation.selectedConversation else {
            AppLogger.error(
                "ConversationList command failed: selected conversation unavailable command=\(String(describing: request.command))"
            )
            return
        }
        switch request.command {
        case .mute:
            requestMute(conversation)
        case .delete:
            pendingDelete = conversation
        case .pin:
            Task {
                await model.setPinned(
                    conversation,
                    pinned: !conversation.isPinned
                )
            }
        }
    }

    private func requestMute(_ conversation: ConversationListData) {
        if conversation.isMuted {
            mute(conversation, duration: 0)
        } else {
            pendingMute = conversation
        }
    }

    private func mute(
        _ conversation: ConversationListData,
        duration: Int64
    ) {
        Task {
            await model.setMuted(conversation, duration: duration)
        }
    }

    private func delete(_ conversation: ConversationListData) {
        Task {
            if await model.delete(conversation) {
                navigation.conversationDeleted(conversation.conversationId)
            }
        }
        pendingDelete = nil
    }

    private func clearFilters() {
        keyword = ""
        searchModel.reset()
        unseenOnly = false
        searchFocused = false
    }
}
