import Observation
import SwiftUI

struct ConversationListView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationListModel()
    @State private var keyword = ""
    @State private var unseenOnly = false
    @State private var pendingMute: SwiftConversationListItem?
    @State private var pendingDelete: SwiftConversationListItem?
    @FocusState private var searchFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            header
            NetworkStatusView()
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            AudioPlayerBar()
        }
        .background(theme.primary)
        .navigationSplitViewColumnWidth(min: 260, ideal: 300, max: 340)
        .task {
            await model.start(account: session.handle)
        }
        .task(id: Query(section: navigation.section, keyword: keyword, unseenOnly: unseenOnly)) {
            if !keyword.isEmpty {
                try? await Task.sleep(for: .milliseconds(200))
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
            if !keyword.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                await model.searchRemote(
                    account: session.handle,
                    query: keyword
                )
            }
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
        .onChange(of: navigation.conversationCommandRequest?.revision) {
            guard let request = navigation.conversationCommandRequest,
                  let conversation = navigation.selectedConversation
            else {
                return
            }
            switch request.command {
            case .mute:
                if conversation.isMuted {
                    mute(conversation, duration: 0)
                } else {
                    pendingMute = conversation
                }
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
                Task {
                    if await model.delete(conversation) {
                        navigation.conversationDeleted(conversation.conversationId)
                    }
                }
                pendingDelete = nil
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
    }

    private var header: some View {
        HStack(spacing: 0) {
            Spacer()
                .frame(width: 16)

            ConversationSearchField(
                text: $keyword,
                focus: $searchFocused,
                placeholder: unseenOnly ? "Search unread (⌘K)" : "Search (⌘K)"
            )

            Spacer()
                .frame(width: 8)

            Button {
                unseenOnly.toggle()
            } label: {
                Image("ConversationFilter")
                    .resizable()
                    .renderingMode(.template)
                    .scaledToFit()
                    .frame(width: 24, height: 24)
                    .foregroundStyle(unseenOnly ? theme.accent : theme.icon)
            }
            .buttonStyle(ConversationHeaderButtonStyle())
            .help("Show unread conversations")
            .accessibilityLabel("Show unread conversations")
            .accessibilityValue(unseenOnly ? "On" : "Off")

            Spacer()
                .frame(width: 4)

            Menu {
                Button("New Conversation") {
                    navigation.showCreation(.conversation)
                }
                Button("New Group") {
                    navigation.showCreation(.group)
                }
                Button("New Circle") {
                    navigation.showCreation(.circle)
                }
            } label: {
                Image("ComposerAdd")
                    .resizable()
                    .renderingMode(.template)
                    .scaledToFit()
                    .frame(width: 32, height: 32)
                    .foregroundStyle(theme.icon)
                    .frame(width: 40, height: 40)
                    .contentShape(Circle())
            }
            .menuStyle(.borderlessButton)
            .fixedSize()
            .help("Create")

            Spacer()
                .frame(width: 12)
        }
        .frame(height: 64)
        .background(theme.primary)
    }

    @ViewBuilder
    private var content: some View {
        switch model.state {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .ready:
            if model.conversations.isEmpty, model.remoteUser == nil {
                ConversationEmptyView()
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        if let user = model.remoteUser {
                            Button {
                                Task {
                                    if let conversationID = await model.openRemoteConversation(
                                        account: session.handle,
                                        user: user
                                    ) {
                                        navigation.selectConversation(
                                            conversationID,
                                            name: user.fullName
                                        )
                                    }
                                }
                            } label: {
                                HStack(spacing: 12) {
                                    MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                                        image.resizable().scaledToFill()
                                    } placeholder: {
                                        Image(systemName: "person.crop.circle.fill")
                                            .foregroundStyle(.secondary)
                                    }
                                    .frame(width: 44, height: 44)
                                    .clipShape(Circle())
                                    VStack(alignment: .leading, spacing: 3) {
                                        Text(user.fullName)
                                            .foregroundStyle(.primary)
                                        Text("Global User · \(user.identityNumber)")
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                }
                                .padding(.horizontal, 12)
                                .frame(minHeight: 62)
                            }
                            .buttonStyle(MixinRowButtonStyle(selected: false))
                            .padding(.horizontal, 8)
                            Divider()
                        }
                        ForEach(model.conversations) { conversation in
                            Button {
                                navigation.selectConversation(conversation)
                            } label: {
                                ConversationRow(
                                    conversation: conversation,
                                    keyword: keyword,
                                    currentUserID: session.profile.userId
                                )
                            }
                            .buttonStyle(MixinRowButtonStyle(
                                selected: navigation.selectedConversationID == conversation.id
                            ))
                            .padding(.horizontal, 8)
                            .contextMenu {
                                Button(conversation.isPinned ? "Unpin" : "Pin") {
                                    Task {
                                        await model.setPinned(
                                            conversation,
                                            pinned: !conversation.isPinned
                                        )
                                    }
                                }
                                Menu(conversation.isMuted ? "Unmute" : "Mute") {
                                    if conversation.isMuted {
                                        Button("Unmute") {
                                            mute(conversation, duration: 0)
                                        }
                                    } else {
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
                                    }
                                }
                                Divider()
                                Button("Delete Chat", role: .destructive) {
                                    pendingDelete = conversation
                                }
                            }
                        }
                        if model.canLoadMore || model.loadingMore {
                            ProgressView()
                                .controlSize(.small)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 12)
                                .onAppear {
                                    Task {
                                        await model.loadNextPage()
                                    }
                                }
                        }
                    }
                }
            }
        case let .failed(message):
            ContentUnavailableView(
                "Unable to load conversations",
                systemImage: "exclamationmark.triangle",
                description: Text(message)
            )
        }
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

    private func mute(
        _ conversation: SwiftConversationListItem,
        duration: Int64
    ) {
        Task {
            await model.setMuted(conversation, duration: duration)
        }
    }
}

private struct ConversationSearchField: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @Binding var text: String
    let focus: FocusState<Bool>.Binding
    let placeholder: String

    var body: some View {
        HStack(spacing: 0) {
            Image("ConversationSearch")
                .resizable()
                .renderingMode(.template)
                .scaledToFit()
                .frame(width: 16, height: 16)
                .foregroundStyle(theme.secondaryText)
                .padding(.leading, 16)
                .padding(.trailing, 8)

            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)
                .focused(focus)
                .onExitCommand {
                    if text.isEmpty {
                        focus.wrappedValue = false
                    } else {
                        text = ""
                    }
                }

            if text.isEmpty {
                Spacer()
                    .frame(width: 40)
            } else {
                Button {
                    text = ""
                    focus.wrappedValue = true
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(theme.secondaryText)
                        .frame(width: 40, height: 36)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Clear search")
            }
        }
        .frame(height: 36)
        .background(searchBackground, in: Capsule())
    }

    private var searchBackground: Color {
        colorScheme == .dark
            ? Color.white.opacity(0.08)
            : Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255)
    }
}

private struct ConversationHeaderButtonStyle: ButtonStyle {
    @Environment(\.mixinTheme) private var theme

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(width: 40, height: 40)
            .contentShape(Circle())
            .background(
                configuration.isPressed ? theme.icon.opacity(0.1) : .clear,
                in: Circle()
            )
    }
}

private struct ConversationEmptyView: View {
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        VStack(spacing: 24) {
            Image("ConversationEmpty")
                .resizable()
                .scaledToFit()
                .frame(width: 58, height: 78)
            Text("No Data")
                .font(.system(size: 14))
                .foregroundStyle(theme.secondaryText)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

private struct ConversationRow: View {
    @Environment(\.mixinTheme) private var theme
    let conversation: SwiftConversationListItem
    let keyword: String
    let currentUserID: String

    var body: some View {
        HStack(spacing: 12) {
            ConversationAvatar(conversation: conversation)
            VStack(spacing: 8) {
                HStack(spacing: 8) {
                    HStack(spacing: 4) {
                        MessageRichText(
                            content: conversation.name,
                            baseFontSize: 16,
                            lineLimit: 1,
                            highlight: keyword
                        )
                        ConversationIdentityBadge(conversation: conversation)
                    }
                    Spacer(minLength: 8)
                    Text(conversation.formattedTime)
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                }
                HStack(spacing: 8) {
                    ConversationPreview(
                        conversation: conversation,
                        currentUserID: currentUserID,
                        keyword: keyword
                    )
                    Spacer(minLength: 4)
                    if conversation.mentionCount > 0 {
                        UnreadBadge(text: "@", muted: false)
                    }
                    if conversation.unseenCount > 0 {
                        UnreadBadge(
                            text: String(conversation.unseenCount),
                            muted: conversation.isMuted
                        )
                    } else {
                        if conversation.isMuted {
                            Image("ConversationMute")
                                .resizable()
                                .renderingMode(.template)
                                .scaledToFit()
                                .frame(width: 14, height: 14)
                                .foregroundStyle(theme.secondaryText)
                        }
                        if conversation.isPinned {
                            Image("ConversationPin")
                                .resizable()
                                .renderingMode(.template)
                                .scaledToFit()
                                .frame(width: 14, height: 14)
                                .foregroundStyle(theme.secondaryText)
                        }
                    }
                }
            }
        }
        .padding(.horizontal, 12)
        .frame(height: 78)
    }
}

private struct ConversationAvatar: View {
    let conversation: SwiftConversationListItem

    @ViewBuilder
    var body: some View {
        if conversation.category == "GROUP", !conversation.groupAvatars.isEmpty {
            GroupConversationAvatar(avatars: Array(conversation.groupAvatars.prefix(4)))
                .frame(width: 50, height: 50)
                .clipShape(Circle())
        } else {
            AvatarTile(
                name: conversation.name,
                url: conversation.iconUrl
            )
            .frame(width: 50, height: 50)
            .clipShape(Circle())
        }
    }
}

private struct GroupConversationAvatar: View {
    let avatars: [SwiftGroupAvatar]

    var body: some View {
        GeometryReader { proxy in
            let cell = proxy.size.width / 2
            switch avatars.count {
            case 1:
                AvatarTile(name: avatars[0].name, url: avatars[0].avatarUrl)
            case 2:
                HStack(spacing: 1) {
                    ForEach(Array(avatars.enumerated()), id: \.offset) { _, avatar in
                        AvatarTile(name: avatar.name, url: avatar.avatarUrl)
                    }
                }
            case 3:
                HStack(spacing: 1) {
                    AvatarTile(name: avatars[0].name, url: avatars[0].avatarUrl)
                    VStack(spacing: 1) {
                        AvatarTile(name: avatars[1].name, url: avatars[1].avatarUrl)
                        AvatarTile(name: avatars[2].name, url: avatars[2].avatarUrl)
                    }
                }
            default:
                LazyVGrid(
                    columns: [
                        GridItem(.fixed(cell), spacing: 1),
                        GridItem(.fixed(cell), spacing: 1),
                    ],
                    spacing: 1
                ) {
                    ForEach(Array(avatars.enumerated()), id: \.offset) { _, avatar in
                        AvatarTile(name: avatar.name, url: avatar.avatarUrl)
                            .frame(width: cell, height: cell)
                    }
                }
            }
        }
    }
}

private struct AvatarTile: View {
    let name: String
    let url: String

    var body: some View {
        MixinRemoteImage(url: URL(string: url)) { image in
            image.resizable().scaledToFill()
        } placeholder: {
            ZStack {
                Color.accentColor.opacity(0.16)
                Text(name.first.map { String($0).uppercased() } ?? "?")
                    .font(.headline)
                    .foregroundStyle(Color.accentColor)
            }
        }
        .clipped()
    }
}

private struct ConversationIdentityBadge: View {
    let conversation: SwiftConversationListItem

    var body: some View {
        HStack(spacing: 3) {
            if let plan = conversation.activeMembershipPlan {
                Image(systemName: plan == "prosperity" ? "crown.fill" : "star.circle.fill")
                    .foregroundStyle(plan == "advance" ? .blue : plan == "elite" ? .purple : .orange)
                    .help("Mixin \(plan.capitalized)")
            } else if conversation.isVerified {
                Image(systemName: "checkmark.seal.fill")
                    .foregroundStyle(.blue)
                    .help("Verified")
            } else if conversation.isBot {
                Image(systemName: "bolt.circle.fill")
                    .foregroundStyle(.secondary)
                    .help("Bot")
            }
            if conversation.isScam {
                Image(systemName: "exclamationmark.shield.fill")
                    .foregroundStyle(.red)
                    .help("Scam warning")
            }
        }
        .font(.system(size: 12))
        .accessibilityElement(children: .combine)
    }
}

private struct ConversationPreview: View {
    @Environment(\.mixinTheme) private var theme
    let conversation: SwiftConversationListItem
    let currentUserID: String
    let keyword: String

    var body: some View {
        HStack(spacing: 4) {
            if !conversation.hasDraft,
               conversation.showsOutgoingStatus(for: currentUserID),
               let statusSymbol = conversation.statusSymbol
            {
                if let assetName = conversation.statusAssetName {
                    Image(assetName)
                        .resizable()
                        .renderingMode(.template)
                        .scaledToFit()
                        .frame(width: 14, height: 8)
                        .foregroundStyle(
                            conversation.lastMessageStatus == "READ"
                                ? theme.accent
                                : theme.secondaryText
                        )
                } else {
                    Image(systemName: statusSymbol)
                        .font(.system(size: 11))
                        .foregroundStyle(
                            conversation.lastMessageStatus == "FAILED"
                                ? theme.destructive
                                : theme.secondaryText
                        )
                }
            }
            if let icon = conversation.previewSymbol, !conversation.hasDraft {
                Image(systemName: icon)
                    .font(.system(size: 11))
                    .foregroundStyle(theme.secondaryText)
            }
            if conversation.hasDraft {
                Text("Draft:")
                    .font(.system(size: 14))
                    .foregroundStyle(theme.destructive)
            }
            MessageRichText(
                content: conversation.rowPreview(currentUserID: currentUserID),
                baseFontSize: 14,
                color: theme.secondaryText,
                lineLimit: 1,
                highlight: keyword
            )
        }
        .lineLimit(1)
    }
}

private struct UnreadBadge: View {
    @Environment(\.mixinTheme) private var theme
    let text: String
    let muted: Bool

    var body: some View {
        Text(text)
            .font(.system(size: 12))
            .foregroundStyle(.white)
            .padding(.horizontal, 6)
            .frame(minWidth: 26, minHeight: 20)
            .background(muted ? theme.secondaryText : theme.accent)
            .clipShape(Capsule())
    }
}

@MainActor
@Observable
final class ConversationListModel {
    private static let pageSize: Int64 = 50

    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var conversations: [SwiftConversationListItem] = []
    private(set) var remoteUser: SwiftUserItem?
    private(set) var operationError: String?
    private(set) var canLoadMore = false
    private(set) var loadingMore = false
    private var requestVersion = 0
    private var remoteSearchVersion = 0
    private var account: SwiftAccountHandle?
    private var currentSection: HomeSection = .chats
    private var currentKeyword = ""
    private var currentUnseenOnly = false
    private var loadedLimit = Int(ConversationListModel.pageSize)
    private var nextOffset: Int64 = 0
    private var subscription: SwiftConversationSubscription?
    private var subscriptionTask: Task<Void, Never>?

    func start(account: SwiftAccountHandle) async {
        guard self.account !== account else {
            return
        }
        stop()
        self.account = account
        let subscription = account.conversationChanges()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let event = await subscription.next()
            {
                guard let self else {
                    return
                }
                if event.reloadAll {
                    await reloadVisibleWindow()
                } else if !event.conversationIds.isEmpty {
                    await applyChanges(conversationIDs: event.conversationIds)
                }
            }
        }
    }

    func load(
        account: SwiftAccountHandle,
        section: HomeSection,
        keyword: String,
        unseenOnly: Bool
    ) async {
        currentSection = section
        currentKeyword = keyword
        currentUnseenOnly = unseenOnly
        requestVersion += 1
        let version = requestVersion
        state = .loading
        remoteSearchVersion += 1
        remoteUser = nil
        loadingMore = false
        loadedLimit = Int(Self.pageSize)
        nextOffset = 0
        do {
            let filter = section.conversationFilter
            let page = try await account.conversations(
                category: filter.category,
                circleId: filter.circleID,
                keyword: keyword,
                unseenOnly: unseenOnly,
                limit: Self.pageSize,
                offset: 0
            )
            guard version == requestVersion else {
                return
            }
            conversations = page
            nextOffset = Int64(page.count)
            canLoadMore = page.count == Self.pageSize
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func loadNextPage() async {
        guard let account,
              case .ready = state,
              canLoadMore,
              !loadingMore
        else {
            return
        }
        loadingMore = true
        let version = requestVersion
        let offset = nextOffset
        do {
            let filter = currentSection.conversationFilter
            let page = try await account.conversations(
                category: filter.category,
                circleId: filter.circleID,
                keyword: currentKeyword,
                unseenOnly: currentUnseenOnly,
                limit: Self.pageSize,
                offset: offset
            )
            guard version == requestVersion else {
                return
            }
            merge(page)
            nextOffset += Int64(page.count)
            loadedLimit = max(loadedLimit, Int(nextOffset))
            canLoadMore = page.count == Self.pageSize
        } catch {
            guard version == requestVersion else {
                return
            }
            operationError = MixinErrorPresenter.message(for: error)
        }
        if version == requestVersion {
            loadingMore = false
        }
    }

    func searchRemote(account: SwiftAccountHandle, query: String) async {
        remoteSearchVersion += 1
        let version = remoteSearchVersion
        do {
            let user = try await account.searchUser(
                query: query.trimmingCharacters(in: .whitespacesAndNewlines)
            )
            guard version == remoteSearchVersion, !Task.isCancelled else {
                return
            }
            remoteUser = conversations.contains(where: {
                $0.ownerId == user.userId
            }) ? nil : user
        } catch {
            guard version == remoteSearchVersion else {
                return
            }
            remoteUser = nil
        }
    }

    func openRemoteConversation(
        account: SwiftAccountHandle,
        user: SwiftUserItem
    ) async -> String? {
        do {
            return try await account.openUserConversation(userId: user.userId)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return nil
        }
    }

    func stop() {
        subscription?.cancel()
        subscription = nil
        subscriptionTask?.cancel()
        subscriptionTask = nil
        account = nil
    }

    func setPinned(
        _ conversation: SwiftConversationListItem,
        pinned: Bool
    ) async {
        guard let account else {
            return
        }
        do {
            try await account.setConversationPinned(
                conversationId: conversation.conversationId,
                pinned: pinned
            )
            await reloadVisibleWindow()
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func setMuted(
        _ conversation: SwiftConversationListItem,
        duration: Int64
    ) async {
        guard let account else {
            return
        }
        do {
            try await account.setConversationMuted(
                conversationId: conversation.conversationId,
                ownerId: conversation.ownerId,
                category: conversation.category,
                durationSeconds: duration
            )
            await reloadVisibleWindow()
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func delete(_ conversation: SwiftConversationListItem) async -> Bool {
        guard let account else {
            return false
        }
        do {
            try await account.deleteConversation(
                conversationId: conversation.conversationId
            )
            await reloadVisibleWindow()
            return true
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func dismissOperationError() {
        operationError = nil
    }

    private func reloadVisibleWindow() async {
        guard let account else {
            return
        }
        let version = requestVersion
        let filter = currentSection.conversationFilter
        let requestedLimit = max(loadedLimit, Int(Self.pageSize))
        do {
            let items = try await account.conversations(
                category: filter.category,
                circleId: filter.circleID,
                keyword: currentKeyword,
                unseenOnly: currentUnseenOnly,
                limit: Int64(requestedLimit),
                offset: 0
            )
            guard version == requestVersion else {
                return
            }
            conversations = items
            loadedLimit = requestedLimit
            nextOffset = Int64(items.count)
            canLoadMore = items.count == requestedLimit
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func applyChanges(conversationIDs: [String]) async {
        guard let account else {
            return
        }
        let version = requestVersion
        do {
            let changed = try await account.conversationItemsByIds(
                conversationIds: Array(Set(conversationIDs))
            )
            guard version == requestVersion else {
                return
            }
            let changedIDs = Set(conversationIDs)
            conversations.removeAll { changedIDs.contains($0.conversationId) }
            conversations.append(contentsOf: changed.filter(matchesCurrentQuery))
            conversations.sort(by: Self.isOrderedBefore)
            if conversations.count > loadedLimit {
                conversations.removeLast(conversations.count - loadedLimit)
            }
        } catch {
            guard version == requestVersion else {
                return
            }
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func merge(_ items: [SwiftConversationListItem]) {
        var byID = Dictionary(
            uniqueKeysWithValues: conversations.map { ($0.conversationId, $0) }
        )
        for item in items {
            byID[item.conversationId] = item
        }
        conversations = byID.values.sorted(by: Self.isOrderedBefore)
    }

    private func matchesCurrentQuery(_ item: SwiftConversationListItem) -> Bool {
        let filter = currentSection.conversationFilter
        let sectionMatches = switch filter.category {
        case "contacts":
            item.category == "CONTACT" && item.relationship == "FRIEND" && !item.isBot
        case "groups":
            item.category == "GROUP"
        case "bots":
            item.category == "CONTACT" && item.isBot
        case "strangers":
            item.category == "CONTACT" && item.relationship == "STRANGER" && !item.isBot
        case "circle":
            filter.circleID.map(item.circleIds.contains) ?? false
        default:
            item.category == "CONTACT" || item.category == "GROUP"
        }
        guard sectionMatches, !currentUnseenOnly || item.unseenCount > 0 else {
            return false
        }
        let query = currentKeyword.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return true
        }
        return [item.name, item.identityNumber, item.lastMessage].contains {
            $0.localizedCaseInsensitiveContains(query)
        }
    }

    private static func isOrderedBefore(
        _ lhs: SwiftConversationListItem,
        _ rhs: SwiftConversationListItem
    ) -> Bool {
        if lhs.pinTimeMillis != rhs.pinTimeMillis {
            return lhs.pinTimeMillis > rhs.pinTimeMillis
        }
        if lhs.hasDraft != rhs.hasDraft {
            return lhs.hasDraft
        }
        if lhs.updatedAtMillis != rhs.updatedAtMillis {
            return lhs.updatedAtMillis > rhs.updatedAtMillis
        }
        return lhs.conversationId < rhs.conversationId
    }
}

private struct Query: Hashable {
    let section: HomeSection
    let keyword: String
    let unseenOnly: Bool
}

extension SwiftConversationListItem: Identifiable {
    public var id: String {
        conversationId
    }

    var initial: String {
        name.first.map { String($0).uppercased() } ?? "?"
    }

    var preview: String {
        lastMessage
    }

    var hasDraft: Bool {
        status != 3 && !draft.isEmpty
    }

    var activeMembershipPlan: String? {
        guard let membership,
              let data = membership.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let plan = object["plan"] as? String,
              ["advance", "elite", "prosperity"].contains(plan),
              let expiration = object["expired_at"] as? String,
              let expirationDate = ISO8601DateFormatter.mixin.date(from: expiration),
              expirationDate > Date()
        else {
            return nil
        }
        return plan
    }

    var previewSymbol: String? {
        guard lastMessageStatus != "FAILED", let category = lastMessageCategory else {
            return nil
        }
        if category == "SYSTEM_SAFE_INSCRIPTION" {
            return "arrow.left.arrow.right"
        }
        if category.contains("TRANSCRIPT") || category.contains("DATA") {
            return "doc.fill"
        }
        if category.contains("IMAGE") {
            return "photo.fill"
        }
        if category.contains("VIDEO") || category.contains("LIVE") {
            return "video.fill"
        }
        if category.contains("AUDIO") || category.hasPrefix("WEBRTC_") {
            return "waveform"
        }
        if category.contains("LOCATION") {
            return "location.fill"
        }
        if category.contains("CONTACT") {
            return "person.crop.circle.fill"
        }
        if category.contains("STICKER") {
            return "face.smiling.fill"
        }
        if category.contains("SNAPSHOT") {
            return "arrow.left.arrow.right"
        }
        return nil
    }

    var statusSymbol: String? {
        switch lastMessageStatus?.uppercased() {
        case "FAILED":
            "exclamationmark.circle.fill"
        case "READ":
            "checkmark.circle.fill"
        case "DELIVERED":
            "checkmark.circle"
        case "SENT":
            "checkmark"
        case "SENDING", "PENDING":
            "clock"
        default:
            nil
        }
    }

    var statusAssetName: String? {
        switch lastMessageStatus?.uppercased() {
        case "READ":
            "ConversationRead"
        case "DELIVERED":
            "ConversationDelivered"
        case "SENT":
            "ConversationSent"
        default:
            nil
        }
    }

    func showsOutgoingStatus(for currentUserID: String) -> Bool {
        guard lastMessageSenderId == currentUserID, let category = lastMessageCategory else {
            return false
        }
        return !Self.statusUnsupportedCategories.contains(category)
    }

    func rowPreview(currentUserID: String) -> String {
        if hasDraft {
            return draft
        }
        guard let category = lastMessageCategory else {
            return "No messages yet"
        }
        let text: String
        if lastMessageStatus == "FAILED" {
            text = "Waiting for this message"
        } else if lastMessageStatus == "UNKNOWN" {
            text = "Message not supported"
        } else {
            text = categoryPreview(category: category, currentUserID: currentUserID)
        }
        if category == "SYSTEM_CONVERSATION" || category == "MESSAGE_PIN" {
            return text
        }
        let showSender = category == "GROUP" || self.category == "GROUP"
            || lastMessageSenderId != ownerId
        guard showSender, !text.isEmpty else {
            return text
        }
        let sender = lastMessageSenderId == currentUserID
            ? "You"
            : lastMessageSenderName ?? ""
        return sender.isEmpty ? text : "\(sender): \(text)"
    }

    private func categoryPreview(category: String, currentUserID: String) -> String {
        if category == "SYSTEM_CONVERSATION" {
            return systemConversationPreview(currentUserID: currentUserID)
        }
        if category == "MESSAGE_PIN" {
            let sender = lastMessageSenderName ?? ""
            return "\(sender) pinned \(Self.pinPreview(lastMessage))"
                .trimmingCharacters(in: .whitespaces)
        }
        if category.contains("TEXT") {
            return lastMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if category.contains("SNAPSHOT") {
            return "[Transfer]"
        }
        if category.contains("STICKER") {
            return "[Sticker]"
        }
        if category.contains("IMAGE") {
            return "[Image]"
        }
        if category.contains("VIDEO") {
            return "[Video]"
        }
        if category.contains("LIVE") {
            return "[Live]"
        }
        if category.contains("DATA") {
            return "[File]"
        }
        if category.contains("POST") {
            let content = lastMessage.trimmingCharacters(in: .whitespacesAndNewlines)
            return content.isEmpty ? "Post" : content
        }
        if category.contains("LOCATION") {
            return "[Location]"
        }
        if category.contains("AUDIO") {
            return "[Audio]"
        }
        if category == "APP_BUTTON_GROUP" {
            return Self.appButtonPreview(lastMessage)
        }
        if category == "APP_CARD" {
            return Self.appCardPreview(lastMessage)
        }
        if category.contains("CONTACT") {
            return "[Contact]"
        }
        if category.hasPrefix("WEBRTC_") || category.hasPrefix("KRAKEN_") {
            return "Voice call"
        }
        if category.contains("RECALL") {
            return lastMessageSenderId == currentUserID
                ? "[You deleted this message]"
                : "[This message was deleted]"
        }
        if category.contains("TRANSCRIPT") {
            return "[Transcript]"
        }
        if category.contains("INSCRIPTION") {
            return "[Collectible]"
        }
        return "Message not supported"
    }

    private func systemConversationPreview(currentUserID: String) -> String {
        let participant = lastMessageParticipantId == currentUserID
            ? "You"
            : lastMessageParticipantName ?? ""
        let sender = lastMessageSenderId == currentUserID ? "You" : lastMessageSenderName ?? ""
        switch lastMessageAction?.uppercased() {
        case "JOIN":
            return "\(participant) joined the group"
        case "EXIT":
            return "\(participant) left the group"
        case "ADD":
            return "\(sender) added \(participant)"
        case "REMOVE":
            return "\(sender) removed \(participant)"
        case "CREATE":
            return "\(sender) created this group"
        case "ROLE":
            return "\(participant) is now an admin"
        case "EXPIRE":
            guard let seconds = Int(lastMessage) else {
                return "\(sender) changed disappearing message settings"
            }
            if seconds <= 0 {
                return "\(sender) disabled disappearing messages"
            }
            return "\(sender) set disappearing messages to \(Self.durationText(seconds))"
        default:
            return "Message not supported"
        }
    }

    private static func durationText(_ seconds: Int) -> String {
        if seconds < 60 {
            return "\(seconds) sec"
        }
        if seconds < 3_600 {
            return "\(seconds / 60) min"
        }
        if seconds < 86_400 {
            return "\(seconds / 3_600) hr"
        }
        if seconds < 604_800 {
            return "\(seconds / 86_400) day"
        }
        return "\(seconds / 604_800) wk"
    }

    private static func appButtonPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let items = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]]
        else {
            return ""
        }
        return items.compactMap { $0["label"] as? String }
            .map { "[\($0)]" }
            .joined()
    }

    private static func appCardPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let card = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let title = card["title"] as? String
        else {
            return "[Card]"
        }
        return "[\(title)]"
    }

    private static func pinPreview(_ content: String) -> String {
        guard let data = content.data(using: .utf8),
              let item = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let category = item["category"] as? String
        else {
            return "a message"
        }
        let nestedContent = item["content"] as? String ?? ""
        if category.contains("TEXT") {
            return nestedContent.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if category.contains("IMAGE") {
            return "[Image]"
        }
        if category.contains("VIDEO") {
            return "[Video]"
        }
        if category.contains("AUDIO") {
            return "[Audio]"
        }
        if category.contains("DATA") {
            return "[File]"
        }
        if category.contains("STICKER") {
            return "[Sticker]"
        }
        if category.contains("CONTACT") {
            return "[Contact]"
        }
        if category.contains("LOCATION") {
            return "[Location]"
        }
        return "a message"
    }

    private static let statusUnsupportedCategories: Set<String> = [
        "SYSTEM_CONVERSATION",
        "SYSTEM_ACCOUNT_SNAPSHOT",
        "MESSAGE_RECALL",
        "MESSAGE_PIN",
        "WEBRTC_AUDIO_CANCEL",
        "WEBRTC_AUDIO_DECLINE",
        "WEBRTC_AUDIO_END",
        "WEBRTC_AUDIO_BUSY",
        "WEBRTC_AUDIO_FAILED",
        "KRAKEN_END",
        "KRAKEN_DECLINE",
        "KRAKEN_CANCEL",
        "KRAKEN_INVITE",
    ]

    var formattedTime: String {
        let date = Date(timeIntervalSince1970: TimeInterval(updatedAtMillis) / 1_000)
        if Calendar.current.isDateInToday(date) {
            return date.formatted(date: .omitted, time: .shortened)
        }
        return date.formatted(.dateTime.month(.twoDigits).day(.twoDigits))
    }
}

private extension ISO8601DateFormatter {
    static let mixin: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()
}
