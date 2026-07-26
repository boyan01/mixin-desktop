import AppKit
import Observation
import SwiftUI

private enum MessageSearchCategory: String, Hashable, Identifiable {
    case all
    case text
    case post

    var id: String { rawValue }

    static let filterCases: [Self] = [.text, .post]

    var title: String {
        switch self {
        case .all:
            "All"
        case .text:
            "Text"
        case .post:
            "Post"
        }
    }

    var categories: [String] {
        switch self {
        case .all:
            []
        case .text:
            ["PLAIN_TEXT", "SIGNAL_TEXT", "ENCRYPTED_TEXT"]
        case .post:
            ["PLAIN_POST", "SIGNAL_POST", "ENCRYPTED_POST"]
        }
    }
}

private struct MessageSearchTaskID: Hashable {
    let query: String
    let senderID: String?
    let category: MessageSearchCategory
}

private struct ChatTimelineRow: Identifiable, Equatable {
    let message: SwiftMessageItem
    let startsNewDay: Bool
    let sameUserPrevious: Bool
    let sameUserNext: Bool

    var id: String { message.messageId }
}

private struct ChatTimelineRenderKey: Equatable {
    let modelID: ObjectIdentifier
    let conversationID: String
    let conversationName: String?
    let lastReadMessageID: String?
    let unseenCount: Int64
    let selectedMessageIDs: Set<String>
    let showJumpToLatest: Bool
    let reduceMotion: Bool
}

private struct ChatTimelineRenderBoundary<Content: View>: View, Equatable {
    let key: ChatTimelineRenderKey
    let content: () -> Content

    init(
        key: ChatTimelineRenderKey,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.key = key
        self.content = content
    }

    static func == (
        lhs: ChatTimelineRenderBoundary<Content>,
        rhs: ChatTimelineRenderBoundary<Content>
    ) -> Bool {
        lhs.key == rhs.key
    }

    var body: some View {
        content()
    }
}

private struct ChatHeaderIcon: View {
    @Environment(\.mixinTheme) private var theme
    @State private var hovering = false

    let assetName: String

    var body: some View {
        Image(assetName)
            .resizable()
            .renderingMode(.template)
            .foregroundStyle(theme.icon)
            .frame(width: 24, height: 24)
            .padding(8)
            .background(
                hovering ? theme.icon.opacity(0.07) : .clear,
                in: Circle()
            )
            .contentShape(Circle())
            .onHover { hovering = $0 }
    }
}

private struct ChatHeaderAvatar: View {
    @Environment(\.mixinTheme) private var theme

    let conversation: SwiftConversationListItem?

    @ViewBuilder
    var body: some View {
        if let conversation,
           conversation.category == "GROUP",
           !conversation.groupAvatars.isEmpty
        {
            ChatHeaderGroupAvatar(
                avatars: Array(conversation.groupAvatars.prefix(4))
            )
            .clipShape(Circle())
        } else if let conversation {
            ChatHeaderAvatarTile(
                name: conversation.name,
                url: conversation.iconUrl
            )
            .clipShape(Circle())
        } else {
            Circle()
                .fill(theme.listSelected)
                .overlay {
                    Image(systemName: "bubble.left.fill")
                        .foregroundStyle(theme.secondaryText)
                }
        }
    }
}

private struct ChatHeaderGroupAvatar: View {
    let avatars: [SwiftGroupAvatar]

    var body: some View {
        GeometryReader { proxy in
            let cell = proxy.size.width / 2
            switch avatars.count {
            case 1:
                ChatHeaderAvatarTile(
                    name: avatars[0].name,
                    url: avatars[0].avatarUrl
                )
            case 2:
                HStack(spacing: 1) {
                    ForEach(Array(avatars.enumerated()), id: \.offset) { _, avatar in
                        ChatHeaderAvatarTile(
                            name: avatar.name,
                            url: avatar.avatarUrl
                        )
                    }
                }
            case 3:
                HStack(spacing: 1) {
                    ChatHeaderAvatarTile(
                        name: avatars[0].name,
                        url: avatars[0].avatarUrl
                    )
                    VStack(spacing: 1) {
                        ChatHeaderAvatarTile(
                            name: avatars[1].name,
                            url: avatars[1].avatarUrl
                        )
                        ChatHeaderAvatarTile(
                            name: avatars[2].name,
                            url: avatars[2].avatarUrl
                        )
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
                        ChatHeaderAvatarTile(
                            name: avatar.name,
                            url: avatar.avatarUrl
                        )
                        .frame(width: cell, height: cell)
                    }
                }
            }
        }
    }
}

private struct ChatHeaderAvatarTile: View {
    @Environment(\.mixinTheme) private var theme

    let name: String
    let url: String

    var body: some View {
        MixinRemoteImage(url: URL(string: url)) { image in
            image
                .resizable()
                .scaledToFill()
        } placeholder: {
            ZStack {
                theme.accent.opacity(0.16)
                Text(name.first.map { String($0).uppercased() } ?? "?")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(theme.accent)
            }
        }
        .clipped()
    }
}

struct ChatTimelineView: View {
    @Environment(AppModel.self) private var appModel
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.mixinTheme) private var theme
    @State private var model = ChatTimelineModel()
    @State private var draft: String
    @State private var searchPresented = false
    @State private var searchQuery = ""
    @State private var searchSender: SwiftConversationParticipantItem?
    @State private var searchCategory = MessageSearchCategory.all
    @State private var searchSenderPresented = false
    @State private var replyMessage: SwiftMessageItem?
    @State private var selectedMessageIDs = Set<String>()
    @State private var pendingDeletion: [SwiftMessageItem] = []
    @State private var attachmentURLs: [URL] = []
    @State private var contactPickerPresented = false
    @State private var stickerPresented = false
    @State private var stickerDetailID: String?
    @State private var voiceRecorder = VoiceRecorderModel()
    @State private var scamWarningVisible: Bool
    @State private var showJumpToLatest = false
    @State private var pinnedMessagesPresented = false
    @State private var dropTargeted = false
    @State private var forwardCombined: Bool?
    @State private var composerFocusRevision = 0
    @FocusState private var searchFocused: Bool
    let conversationID: String
    let conversationName: String?
    let conversationCategory: String?
    let conversationOwnerID: String?
    let conversationIsBot: Bool
    let conversationIsScam: Bool
    let participantCount: Int64
    let lastReadMessageID: String?
    let unseenCount: Int64

    init(
        conversationID: String,
        conversationName: String?,
        conversationCategory: String?,
        conversationOwnerID: String?,
        conversationIsBot: Bool,
        conversationIsScam: Bool,
        participantCount: Int64,
        lastReadMessageID: String?,
        unseenCount: Int64,
        initialDraft: String
    ) {
        self.conversationID = conversationID
        self.conversationName = conversationName
        self.conversationCategory = conversationCategory
        self.conversationOwnerID = conversationOwnerID
        self.conversationIsBot = conversationIsBot
        self.conversationIsScam = conversationIsScam
        self.participantCount = participantCount
        self.lastReadMessageID = lastReadMessageID
        self.unseenCount = unseenCount
        _scamWarningVisible = State(initialValue: conversationIsScam)
        _draft = State(initialValue: initialDraft)
    }

    var body: some View {
        VStack(spacing: 0) {
            chatHeader

            if scamWarningVisible {
                HStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundStyle(.red)
                    Text("Warning: this account has been reported as a scam.")
                        .frame(maxWidth: .infinity, alignment: .leading)
                    Button {
                        scamWarningVisible = false
                    } label: {
                        Image(systemName: "xmark")
                    }
                    .buttonStyle(.borderless)
                }
                .padding(12)
                .background(.red.opacity(0.1))
            }

            if searchPresented {
                searchContent
            } else {
                ChatTimelineRenderBoundary(
                    key: ChatTimelineRenderKey(
                        modelID: ObjectIdentifier(model),
                        conversationID: conversationID,
                        conversationName: conversationName,
                        lastReadMessageID: lastReadMessageID,
                        unseenCount: unseenCount,
                        selectedMessageIDs: selectedMessageIDs,
                        showJumpToLatest: showJumpToLatest,
                        reduceMotion: reduceMotion
                    )
                ) {
                    timelineContent
                }
                .equatable()
                    .background {
                        ChatBackgroundView()
                    }
                    .clipped()
            }

            Divider()

            if !selectedMessageIDs.isEmpty {
                selectionBar
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            } else if voiceRecorder.status != .idle {
                VoiceRecorderBar(
                    recorder: voiceRecorder,
                    sending: model.sending,
                    errorText: voiceRecorder.errorMessage ?? model.sendError,
                    onSend: sendVoiceRecording
                )
            } else if conversationCategory == "GROUP", participantCount == 0 {
                Label(
                    "You are no longer a participant in this group.",
                    systemImage: "person.crop.circle.badge.xmark"
                )
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, minHeight: 54)
            } else {
                composer
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(
            reduceMotion ? nil : .easeInOut(duration: 0.2),
            value: selectedMessageIDs.isEmpty
        )
        .task(id: conversationID) {
            await model.start(
                account: session.handle,
                conversationID: conversationID,
                lastReadMessageID: lastReadMessageID,
                unseenCount: unseenCount
            )
            await consumeMessageJump()
        }
        .onDisappear {
            model.stop()
            voiceRecorder.dispose()
        }
        .dropDestination(for: URL.self) { urls, _ in
            let files = urls.filter(\.isFileURL)
            guard !files.isEmpty, selectedMessageIDs.isEmpty else {
                return false
            }
            attachmentURLs = files
            return true
        } isTargeted: {
            dropTargeted = $0
        }
        .overlay {
            if dropTargeted, selectedMessageIDs.isEmpty {
                RoundedRectangle(cornerRadius: 14)
                    .fill(.ultraThinMaterial)
                    .stroke(Color.accentColor, style: StrokeStyle(
                        lineWidth: 2,
                        dash: [7]
                    ))
                    .padding(12)
                    .overlay {
                        Label("Drop files to send", systemImage: "tray.and.arrow.down.fill")
                            .font(.title3.weight(.semibold))
                    }
                    .allowsHitTesting(false)
            }
        }
        .onChange(of: navigation.messageSearchRequest) {
            showSearch()
        }
        .onChange(of: navigation.messageJumpRequest?.revision) {
            Task {
                await consumeMessageJump()
            }
        }
        .task(id: MessageSearchTaskID(
            query: searchQuery,
            senderID: searchSender?.userId,
            category: searchCategory
        )) {
            guard searchPresented else {
                return
            }
            let query = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !query.isEmpty else {
                model.clearSearch()
                return
            }
            try? await Task.sleep(for: .milliseconds(250))
            guard !Task.isCancelled else {
                return
            }
            await model.search(
                query: query,
                senderID: searchSender?.userId,
                categories: searchCategory.categories,
                append: false
            )
        }
        .task(id: draft) {
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else {
                return
            }
            await model.saveDraft(draft)
        }
        .confirmationDialog(
            "Delete \(pendingDeletion.count) message\(pendingDeletion.count == 1 ? "" : "s")?",
            isPresented: deletionPresented
        ) {
            if pendingDeletion.allSatisfy(canRecall) {
                Button("Delete for Everyone", role: .destructive) {
                    mutateDeletion(recall: true)
                }
            }
            Button("Delete for Me", role: .destructive) {
                mutateDeletion(recall: false)
            }
            Button("Cancel", role: .cancel) {
                pendingDeletion = []
            }
        }
        .alert("Message action failed", isPresented: mutationErrorPresented) {
            Button("OK") {
                model.clearMutationError()
            }
        } message: {
            Text(model.mutationError ?? "")
        }
        .alert("Voice recording failed", isPresented: voiceErrorPresented) {
            Button("OK") {
                voiceRecorder.clearError()
            }
        } message: {
            Text(voiceRecorder.errorMessage ?? "")
        }
        .sheet(isPresented: Binding(
            get: { !attachmentURLs.isEmpty },
            set: { if !$0 { attachmentURLs = [] } }
        )) {
            AttachmentPreviewSheet(
                urls: attachmentURLs,
                onSend: { request in
                    await model.sendAttachment(
                        request,
                        quoteMessageID: replyMessage?.messageId
                    )
                },
                onComplete: {
                    replyMessage = nil
                    attachmentURLs = []
                }
            )
        }
        .sheet(isPresented: $contactPickerPresented) {
            ContactShareSheet { userID in
                if await model.sendContact(
                    userID: userID,
                    quoteMessageID: replyMessage?.messageId
                ) {
                    replyMessage = nil
                    return true
                }
                return false
            }
        }
        .sheet(isPresented: $pinnedMessagesPresented) {
            NavigationStack {
                List(model.pinnedMessages, id: \.messageId) { message in
                    Button {
                        pinnedMessagesPresented = false
                        Task {
                            _ = await model.locate(messageID: message.messageId)
                        }
                    } label: {
                        SearchMessageRow(
                            message: message,
                            mentionNames: model.mentionNames
                        )
                    }
                    .buttonStyle(.plain)
                }
                .frame(minWidth: 420, minHeight: 480)
                .navigationTitle("Pinned Messages")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Close") {
                            pinnedMessagesPresented = false
                        }
                    }
                }
            }
        }
        .sheet(isPresented: $searchSenderPresented) {
            MessageSearchSenderSheet(
                account: session.handle,
                conversationID: conversationID,
                isBot: conversationIsBot
            ) { sender in
                searchSender = sender
            }
        }
        .sheet(isPresented: Binding(
            get: { forwardCombined != nil },
            set: { if !$0 { forwardCombined = nil } }
        )) {
            if let combined = forwardCombined {
                ForwardConversationSheet(
                    account: session.handle,
                    combined: combined
                ) { targetConversationID in
                    let succeeded = await model.forward(
                        messageIDs: selectedMessages.map(\.messageId),
                        targetConversationID: targetConversationID,
                        combined: combined
                    )
                    if succeeded {
                        selectedMessageIDs.removeAll()
                        forwardCombined = nil
                    }
                    return succeeded
                }
            }
        }
        .sheet(isPresented: Binding(
            get: { stickerDetailID != nil },
            set: { if !$0 { stickerDetailID = nil } }
        )) {
            if let stickerDetailID,
               let desktop = appModel.desktopHandle
            {
                StickerMessageDetailSheet(
                    account: session.handle,
                    desktop: desktop,
                    accountID: session.profile.userId,
                    stickerID: stickerDetailID
                )
            }
        }
    }

    private var chatHeader: some View {
        HStack(spacing: 0) {
            Button {
                navigation.toggleConversationInfo()
            } label: {
                ChatHeaderAvatar(conversation: navigation.selectedConversation)
                    .frame(width: 36, height: 36)

                VStack(alignment: .leading, spacing: 4) {
                    Text(conversationName ?? "Conversation")
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                        .lineLimit(1)

                    Text(chatHeaderSubtitle)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.secondaryText)
                        .lineLimit(1)
                }
                .padding(.leading, 10)
            }
            .buttonStyle(.plain)

            Spacer(minLength: 8)

            Button {
                showSearch()
            } label: {
                ChatHeaderIcon(assetName: "ChatHeaderSearch")
            }
            .buttonStyle(.plain)
            .help("Search messages")

            if conversationIsBot, let appID = conversationOwnerID {
                Button {
                    Task {
                        await model.openBotHome(
                            appID: appID,
                            title: conversationName ?? "",
                            currency: session.profile.fiatCurrency
                        )
                    }
                } label: {
                    ChatHeaderIcon(assetName: "ChatHeaderBot")
                }
                .buttonStyle(.plain)
                .help("Open bot app")
            }

            Button {
                navigation.toggleConversationInfo()
            } label: {
                ChatHeaderIcon(assetName: "ChatHeaderInfo")
            }
            .buttonStyle(.plain)
            .help("Conversation info")
        }
        .padding(.leading, 16)
        .padding(.trailing, 16)
        .frame(height: 64)
        .background(theme.primary)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(theme.divider)
                .frame(height: 1)
        }
    }

    private var chatHeaderSubtitle: String {
        if conversationCategory == "GROUP" {
            return "\(participantCount) participants"
        }
        if let identityNumber = navigation.selectedConversation?.identityNumber,
           !identityNumber.isEmpty
        {
            return identityNumber
        }
        return conversationID
    }

    private var searchContent: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField("Search messages", text: $searchQuery)
                    .textFieldStyle(.plain)
                    .focused($searchFocused)
                    .onExitCommand {
                        closeSearch()
                    }
                if model.searching {
                    ProgressView()
                        .controlSize(.small)
                }
                Button {
                    closeSearch()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.borderless)
                .foregroundStyle(.secondary)
                .help("Close search")
            }
            .padding(.horizontal, 14)
            .frame(height: 44)

            Divider()

            if !searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                HStack(spacing: 8) {
                    Button {
                        searchSenderPresented = true
                    } label: {
                        Label(
                            searchSender.map { "From: \($0.fullName)" } ?? "From",
                            systemImage: "person"
                        )
                    }
                    .buttonStyle(.bordered)

                    ForEach(MessageSearchCategory.filterCases) { category in
                        Button(category.title) {
                            searchCategory = searchCategory == category ? .all : category
                        }
                        .buttonStyle(.bordered)
                        .tint(searchCategory == category ? .accentColor : nil)
                    }
                    Spacer()
                }
                .controlSize(.small)
                .padding(.horizontal, 14)
                .padding(.bottom, 8)
            }

            if let error = model.searchError {
                ContentUnavailableView(
                    "Unable to search messages",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error)
                )
            } else if searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                ContentUnavailableView(
                    "Search this conversation",
                    systemImage: "text.magnifyingglass"
                )
            } else if !model.searching, model.searchResults.isEmpty {
                ContentUnavailableView.search(text: searchQuery)
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(model.searchResults, id: \.messageId) { message in
                            Button {
                                Task {
                                    if await model.locate(messageID: message.messageId) {
                                        closeSearch(clearResults: false)
                                    }
                                }
                            } label: {
                                SearchMessageRow(
                                    message: message,
                                    mentionNames: model.mentionNames,
                                    highlight: searchQuery
                                )
                            }
                            .buttonStyle(MixinRowButtonStyle(selected: false))
                            .padding(.horizontal, 6)
                            .onAppear {
                                guard message.messageId == model.searchResults.last?.messageId,
                                      model.hasMoreSearchResults
                                else {
                                    return
                                }
                                Task {
                                    await model.search(
                                        query: searchQuery.trimmingCharacters(
                                            in: .whitespacesAndNewlines
                                        ),
                                        senderID: searchSender?.userId,
                                        categories: searchCategory.categories,
                                        append: true
                                    )
                                }
                            }
                            Divider()
                                .padding(.leading, 74)
                        }
                    }
                }
            }
        }
    }

    private func showSearch() {
        searchPresented = true
        searchFocused = true
    }

    private func consumeMessageJump() async {
        guard
            let request = navigation.messageJumpRequest,
            request.conversationID == conversationID
        else {
            return
        }
        _ = await model.locate(messageID: request.messageID)
        navigation.consumeMessageJump(revision: request.revision)
    }

    private func closeSearch(clearResults: Bool = true) {
        searchPresented = false
        searchFocused = false
        searchQuery = ""
        searchSender = nil
        searchCategory = .all
        if clearResults {
            model.clearSearch()
        }
    }

    private var composer: some View {
        let hasText = !draft.trimmingCharacters(
            in: .whitespacesAndNewlines
        ).isEmpty

        return VStack(alignment: .leading, spacing: 0) {
            if let replyMessage {
                HStack(spacing: 8) {
                    Capsule()
                        .fill(Color.accentColor)
                        .frame(width: 3)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Replying to \(replyMessage.senderName)")
                            .font(.caption)
                            .foregroundStyle(Color.accentColor)
                        Text(replyMessage.displayText)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                    Button {
                        self.replyMessage = nil
                    } label: {
                        Image("ComposerClose")
                            .resizable()
                            .frame(width: 22, height: 22)
                            .padding(14)
                    }
                    .buttonStyle(.plain)
                    .help("Cancel reply")
                }
                .padding(.leading, 16)
                .background(theme.popUp)
                .transition(.move(edge: .bottom).combined(with: .opacity))
            }

            VStack(alignment: .leading, spacing: 6) {
                HStack(alignment: .bottom, spacing: 0) {
                    Menu {
                        Button {
                            contactPickerPresented = true
                        } label: {
                            Label("Contact", systemImage: "person.crop.circle")
                        }
                        Button {
                            pickAttachments()
                        } label: {
                            Label("Files and Media", systemImage: "doc")
                        }
                    } label: {
                        ComposerIcon(
                            assetName: "ComposerAdd",
                            size: 32,
                            padding: 4
                        )
                    }
                    .menuStyle(.borderlessButton)
                    .menuIndicator(.hidden)
                    .fixedSize()
                    .disabled(model.sending)
                    .help("Share contact or attach files")

                    Spacer().frame(width: 6)

                    Button {
                        stickerPresented.toggle()
                    } label: {
                        ComposerIcon(assetName: "ComposerSticker")
                    }
                    .buttonStyle(.plain)
                    .disabled(model.sending)
                    .help("Emoji, stickers and GIFs")
                    .popover(isPresented: $stickerPresented, arrowEdge: .bottom) {
                        if let desktop = appModel.desktopHandle {
                            StickerPanelView(
                                account: session.handle,
                                desktop: desktop,
                                accountID: session.profile.userId,
                                draft: $draft,
                                gifAPIKey: GiphyConfiguration.apiKey,
                                onSendSticker: { stickerID in
                                    await model.sendSticker(stickerID: stickerID)
                                },
                                onSendGIF: { gif in
                                    await model.sendRemoteGIF(gif)
                                },
                                onSent: {
                                    replyMessage = nil
                                    stickerPresented = false
                                }
                            )
                        } else {
                            ContentUnavailableView(
                                "Sticker service unavailable",
                                systemImage: "exclamationmark.triangle"
                            )
                            .frame(width: 360, height: 260)
                        }
                    }

                    Spacer().frame(width: 16)

                    MentionComposer(
                        account: session.handle,
                        conversationID: conversationID,
                        conversationCategory: conversationCategory,
                        conversationOwnerID: conversationOwnerID,
                        currentUserID: session.profile.userId,
                        encrypted: !conversationIsBot,
                        text: $draft,
                        disabled: model.sending,
                        focusRevision: composerFocusRevision,
                        onPasteFiles: { urls in
                            guard selectedMessageIDs.isEmpty, !model.sending else {
                                return
                            }
                            attachmentURLs = urls
                        },
                        onSubmit: {
                            send(silent: false)
                        },
                        onSubmitPost: {
                            sendPost()
                        }
                    )
                    .onChange(of: draft) {
                        if draft.count > 64 * 1024 {
                            draft = String(draft.prefix(64 * 1024))
                        }
                    }

                    Spacer().frame(width: 16)

                    ZStack {
                        if model.sending {
                            ProgressView()
                                .controlSize(.small)
                                .frame(width: 40, height: 40)
                        } else if hasText {
                            Menu {
                                Button("Send Silently") {
                                    send(silent: true)
                                }
                                Button("Send as Post") {
                                    sendPost()
                                }
                            } label: {
                                ComposerIcon(assetName: "ComposerSend")
                            } primaryAction: {
                                send(silent: false)
                            }
                            .menuStyle(.borderlessButton)
                            .menuIndicator(.hidden)
                            .fixedSize()
                            .help("Send message")
                            .transition(.scale(scale: 0.6).combined(with: .opacity))
                        } else {
                            Button {
                                Task {
                                    await voiceRecorder.start(using: session.media)
                                }
                            } label: {
                                ComposerIcon(assetName: "ComposerMicrophone")
                            }
                            .buttonStyle(.plain)
                            .help("Record voice message")
                            .transition(.scale(scale: 0.6).combined(with: .opacity))
                        }
                    }
                    .frame(width: 40, height: 40)
                    .disabled(model.sending)
                    .animation(
                        reduceMotion ? nil : .easeInOut(duration: 0.2),
                        value: hasText
                    )
                }

                if let sendError = model.sendError {
                    Text(sendError)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .lineLimit(2)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(theme.primary)
        }
        .animation(
            reduceMotion ? nil : .easeOut(duration: 0.16),
            value: replyMessage?.messageId
        )
    }

    private func send(silent: Bool) {
        let content = draft
        guard !content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return
        }
        draft = ""
        Task {
            if await model.send(
                content,
                quoteMessageID: replyMessage?.messageId,
                silent: silent
            ) {
                replyMessage = nil
            } else {
                draft = content
            }
        }
    }

    private func sendPost() {
        let content = draft
        guard !content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return
        }
        draft = ""
        Task {
            if await model.sendPost(content) {
                replyMessage = nil
            } else {
                draft = content
            }
        }
    }

    private func pickAttachments() {
        Task {
            let selected = await AttachmentFilePicker.select()
            if !selected.isEmpty {
                attachmentURLs = selected
            }
        }
    }

    private func reedit(_ content: String) {
        draft += content
        composerFocusRevision += 1
    }

    private var selectionBar: some View {
        HStack(spacing: 12) {
            Button("Cancel") {
                selectedMessageIDs.removeAll()
            }
            .keyboardShortcut(.cancelAction)

            Text("\(selectedMessageIDs.count) selected")
                .foregroundStyle(.secondary)

            Spacer()

            Button {
                copySelectedMessages()
            } label: {
                Label("Copy", systemImage: "doc.on.doc")
            }
            .disabled(selectedMessages.isEmpty)

            Button {
                forwardCombined = false
            } label: {
                Label("Forward", systemImage: "arrowshape.turn.up.right")
            }
            .disabled(selectedMessages.isEmpty || model.mutating)

            if selectedMessages.count >= 2, selectedMessages.count < 100 {
                Button {
                    forwardCombined = true
                } label: {
                    Label("Combine", systemImage: "rectangle.stack")
                }
                .disabled(model.mutating)
            }

            Button(role: .destructive) {
                pendingDeletion = selectedMessages
            } label: {
                Label("Delete", systemImage: "trash")
            }
            .disabled(selectedMessages.isEmpty || model.mutating)
        }
        .padding(.horizontal, 14)
        .frame(height: 54)
    }

    @ViewBuilder
    private var timelineContent: some View {
        switch model.state {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(message):
            ContentUnavailableView(
                "Unable to load messages",
                systemImage: "exclamationmark.triangle",
                description: Text(message)
            )
        case .ready:
            if model.messages.isEmpty {
                ContentUnavailableView(
                    "No messages yet",
                    systemImage: "bubble.left"
                )
            } else {
                ScrollViewReader { proxy in
                    ZStack(alignment: .bottomTrailing) {
                        ScrollView {
                            LazyVStack(spacing: 0) {
                                if model.hasOlderMessages {
                                    ProgressView()
                                        .controlSize(.small)
                                        .opacity(model.loadingOlder ? 1 : 0)
                                        .onAppear {
                                            Task {
                                                let anchor = model.messages.first?
                                                    .messageId
                                                await model.loadOlder()
                                                guard let anchor else {
                                                    return
                                                }
                                                var transaction = Transaction()
                                                transaction.disablesAnimations = true
                                                withTransaction(transaction) {
                                                    proxy.scrollTo(anchor, anchor: .top)
                                                }
                                            }
                                        }
                                }

                                ForEach(model.rows) { row in
                                    let message = row.message
                                    if row.startsNewDay {
                                        Text(
                                            message.createdAtMicros.dayChipTitle
                                        )
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                            .padding(.horizontal, 10)
                                            .padding(.vertical, 4)
                                            .background(.regularMaterial, in: Capsule())
                                            .frame(maxWidth: .infinity)
                                    }
                                    if message.messageId == unreadBoundaryMessageID {
                                        HStack(spacing: 8) {
                                            Rectangle()
                                                .frame(height: 1)
                                            Text("Unread Messages")
                                                .font(.caption)
                                            Rectangle()
                                                .frame(height: 1)
                                        }
                                        .foregroundStyle(Color.accentColor)
                                    }
                                    MessageRow(
                                        message: message,
                                        mentionNames: model.mentionNames,
                                        audioPlaylist: message.presentationKind == .audio
                                            ? model.audioMessages
                                            : [],
                                        mediaDirectory: model.mediaDirectory,
                                        conversationName: conversationName,
                                        outgoing: message.senderId == session.profile.userId,
                                        sameUserPrevious: row.sameUserPrevious,
                                        sameUserNext: row.sameUserNext,
                                        policy: policy(for: message),
                                        recalledText: model.recalledText(
                                            messageID: message.messageId
                                        ),
                                        selected: selectedMessageIDs.contains(message.messageId),
                                        selectionActive: !selectedMessageIDs.isEmpty,
                                        imageMessages: message.isRichContent
                                            ? model.imageMessages
                                            : [],
                                        loadImageWindow: model.imageMessagesAround,
                                        onReply: {
                                            replyMessage = message
                                        },
                                        onCopy: copyText,
                                        onSelect: {
                                            selectedMessageIDs.insert(message.messageId)
                                        },
                                        onToggleSelection: {
                                            toggleSelection(message.messageId)
                                        },
                                        onTogglePin: {
                                            Task {
                                                await model.setPinned(
                                                    messageID: message.messageId,
                                                    pinned: !message.pinned
                                                )
                                            }
                                        },
                                        onRecall: {
                                            pendingDeletion = [message]
                                        },
                                        onDelete: {
                                            pendingDeletion = [message]
                                        },
                                        onReedit: reedit,
                                        attachmentProgress: model.attachmentProgress(
                                            messageID: message.messageId
                                        ),
                                        onAttachmentAction: {
                                            Task {
                                                await model.performAttachmentAction(
                                                    message,
                                                    sentByCurrentUser:
                                                        message.senderId
                                                            == session.profile.userId
                                                )
                                            }
                                        },
                                        loadTranscript: {
                                            try await model.transcriptMessages(
                                                transcriptID: message.messageId
                                            )
                                        },
                                        onTranscriptAttachmentAction: { item in
                                            await model.performTranscriptAttachmentAction(
                                                transcriptID: message.messageId,
                                                message: item,
                                                sentByCurrentUser:
                                                    message.senderId
                                                        == session.profile.userId
                                            )
                                        },
                                        onMarkAudioRead: { messageID in
                                            Task {
                                                await model.markAudioRead(messageID: messageID)
                                            }
                                        },
                                        onShowStickerDetail: { stickerID in
                                            stickerDetailID = stickerID
                                        },
                                        onAppAction: { action, title in
                                            Task {
                                                await model.openMessageAction(
                                                    action,
                                                    title: title,
                                                    currency: session.profile.fiatCurrency
                                                )
                                            }
                                        },
                                        onStrangerAction: { action in
                                            await model.handleStrangerAction(
                                                action,
                                                message: message,
                                                currency: session.profile.fiatCurrency
                                            )
                                        }
                                    )
                                    .equatable()
                                    .id(message.messageId)
                                }

                                if model.hasNewerMessages {
                                    ProgressView()
                                        .controlSize(.small)
                                        .opacity(model.loadingNewer ? 1 : 0)
                                        .onAppear {
                                            Task {
                                                let anchor = model.messages.last?
                                                    .messageId
                                                await model.loadNewer()
                                                guard let anchor else {
                                                    return
                                                }
                                                var transaction = Transaction()
                                                transaction.disablesAnimations = true
                                                withTransaction(transaction) {
                                                    proxy.scrollTo(anchor, anchor: .bottom)
                                                }
                                            }
                                        }
                                }
                            }
                            .padding(.horizontal, 18)
                            .padding(.vertical, 12)
                        }
                        .defaultScrollAnchor(.bottom)
                        .onScrollGeometryChange(for: Bool.self) { geometry in
                            geometry.contentOffset.y
                                + geometry.containerSize.height
                                < geometry.contentSize.height - 120
                        } action: { _, visible in
                            showJumpToLatest = visible
                        }
                        .onChange(of: model.initialScrollMessageID) {
                            guard let messageID = model.initialScrollMessageID else {
                                return
                            }
                            proxy.scrollTo(messageID, anchor: .center)
                        }
                        timelineOverlays(proxy: proxy)
                        pinnedMessagesOverlay(proxy: proxy)
                    }
                }
            }
        }
    }

    private var unreadBoundaryMessageID: String? {
        guard unseenCount > 0, !model.messages.isEmpty else {
            return nil
        }
        if let lastReadMessageID,
           let index = model.messages.firstIndex(where: {
               $0.messageId == lastReadMessageID
           }),
           index + 1 < model.messages.count
        {
            return model.messages[index + 1].messageId
        }
        let unseen = min(Int(unseenCount), model.messages.count)
        return model.messages[model.messages.count - unseen].messageId
    }

    @ViewBuilder
    private func timelineOverlays(proxy: ScrollViewProxy) -> some View {
        VStack(alignment: .trailing, spacing: 8) {
            if !model.unreadMentionMessageIDs.isEmpty {
                Button {
                    Task {
                        guard let messageID = model.unreadMentionMessageIDs.first else {
                            return
                        }
                        if await model.locate(messageID: messageID) {
                            proxy.scrollTo(messageID, anchor: .center)
                            await model.markMentionRead(messageID: messageID)
                        }
                    }
                } label: {
                    Text(model.unreadMentionMessageIDs.count > 99
                        ? "99+ @"
                        : "\(model.unreadMentionMessageIDs.count) @")
                        .font(.callout.weight(.semibold))
                        .frame(minWidth: 38, minHeight: 38)
                        .background(.regularMaterial, in: Circle())
                }
                .buttonStyle(.plain)
                .contextMenu {
                    Button("Clear") {
                        Task {
                            await model.clearUnreadMentions()
                        }
                    }
                }
            }
            if showJumpToLatest, !model.messages.isEmpty {
                Button {
                    Task {
                        if model.hasNewerMessages {
                            await model.jumpToLatest()
                        }
                        guard let latest = model.messages.last else {
                            return
                        }
                        withAnimation {
                            proxy.scrollTo(latest.messageId, anchor: .bottom)
                        }
                    }
                } label: {
                    Image(systemName: "arrow.down")
                        .frame(width: 38, height: 38)
                        .background(.regularMaterial, in: Circle())
                }
                .buttonStyle(.plain)
                .help("Jump to latest")
            }
        }
        .padding(16)
    }

    @ViewBuilder
    private func pinnedMessagesOverlay(proxy: ScrollViewProxy) -> some View {
        if let pinned = model.pinnedMessages.first {
            HStack(spacing: 8) {
                if !model.pinnedPreviewDismissed {
                    PinMessagePreviewBubble(
                        text: "\(pinned.senderName) pinned \(pinned.displayText)",
                        onDismiss: model.dismissPinnedPreview,
                        onLocate: {
                            Task {
                                if await model.locate(messageID: pinned.messageId) {
                                    proxy.scrollTo(pinned.messageId, anchor: .center)
                                }
                            }
                        }
                    )
                    .transition(.opacity)
                } else {
                    Spacer(minLength: 0)
                }

                Button {
                    pinnedMessagesPresented = true
                } label: {
                    Image("ChatPin")
                        .resizable()
                        .renderingMode(.template)
                        .foregroundStyle(Color.primary)
                        .frame(width: 34, height: 34)
                        .background(
                            Color(nsColor: .controlBackgroundColor),
                            in: Circle()
                        )
                        .shadow(color: .black.opacity(0.15), radius: 5, y: 2)
                }
                .buttonStyle(.plain)
                .help("Pinned Messages")
            }
            .frame(height: 64)
            .padding(.horizontal, 30)
            .padding(.top, 12)
            .frame(
                maxWidth: .infinity,
                maxHeight: .infinity,
                alignment: .topTrailing
            )
            .animation(
                reduceMotion ? nil : .easeInOut(duration: 0.2),
                value: model.pinnedPreviewDismissed
            )
        }
    }

    private var selectedMessages: [SwiftMessageItem] {
        model.messages.filter { selectedMessageIDs.contains($0.messageId) }
    }

    private var deletionPresented: Binding<Bool> {
        Binding(
            get: { !pendingDeletion.isEmpty },
            set: { presented in
                if !presented {
                    pendingDeletion = []
                }
            }
        )
    }

    private var mutationErrorPresented: Binding<Bool> {
        Binding(
            get: { model.mutationError != nil },
            set: { presented in
                if !presented {
                    model.clearMutationError()
                }
            }
        )
    }

    private var voiceErrorPresented: Binding<Bool> {
        Binding(
            get: { voiceRecorder.errorMessage != nil && voiceRecorder.status == .idle },
            set: { presented in
                if !presented {
                    voiceRecorder.clearError()
                }
            }
        )
    }

    private func policy(for message: SwiftMessageItem) -> MessageActionPolicy {
        MessageActionPolicy(
            message: message,
            currentUserID: session.profile.userId,
            currentUserRole: model.currentUserRole,
            now: Date()
        )
    }

    private func canRecall(_ message: SwiftMessageItem) -> Bool {
        policy(for: message).canRecall
    }

    private func toggleSelection(_ messageID: String) {
        if selectedMessageIDs.remove(messageID) == nil {
            selectedMessageIDs.insert(messageID)
        }
    }

    private func copyText(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.writeObjects([text as NSString])
    }

    private func copySelectedMessages() {
        let content = selectedMessages.map { message in
            "\(message.senderName), (\(message.selectionFormattedTime)):\n\(message.displayText)"
        }
        .joined(separator: "\n\n")
        copyText(content)
        selectedMessageIDs.removeAll()
    }

    private func mutateDeletion(recall: Bool) {
        let messages = pendingDeletion
        let messageIDs = messages.map(\.messageId)
        pendingDeletion = []
        Task {
            let succeeded = recall
                ? await model.recall(messages: messages)
                : await model.delete(messageIDs: messageIDs)
            if succeeded {
                selectedMessageIDs.subtract(messageIDs)
                if let replyMessage, messageIDs.contains(replyMessage.messageId) {
                    self.replyMessage = nil
                }
            }
        }
    }

    private func sendVoiceRecording() {
        let quoteMessageID = replyMessage?.messageId
        Task {
            let sent = await voiceRecorder.send { recording in
                await model.sendAudio(
                    recording,
                    quoteMessageID: quoteMessageID
                )
            }
            if sent {
                replyMessage = nil
            }
        }
    }
}

private struct ComposerIcon: View {
    @Environment(\.mixinTheme) private var theme
    let assetName: String
    var size: CGFloat = 24
    var padding: CGFloat = 8

    var body: some View {
        Image(assetName)
            .resizable()
            .renderingMode(.template)
            .foregroundStyle(theme.icon)
            .frame(width: size, height: size)
            .padding(padding)
            .contentShape(Circle())
    }
}

private struct PinMessagePreviewBubble: View {
    @Environment(\.mixinTheme) private var theme
    let text: String
    let onDismiss: () -> Void
    let onLocate: () -> Void

    var body: some View {
        HStack(spacing: 4) {
            Button(action: onDismiss) {
                Image(systemName: "xmark")
                    .font(.system(size: 12, weight: .medium))
                    .frame(width: 20, height: 20)
                    .padding(8)
            }
            .buttonStyle(.plain)

            Button(action: onLocate) {
                Text(text)
                    .lineLimit(2)
                    .multilineTextAlignment(.leading)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
        }
        .foregroundStyle(theme.text)
        .padding(.leading, 16)
        .padding(.trailing, 23)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            PinMessageBubbleShape()
                .fill(Color(nsColor: .controlBackgroundColor))
                .shadow(color: .black.opacity(0.15), radius: 5, y: 2)
        }
    }
}

private struct PinMessageBubbleShape: Shape {
    private let nipWidth = 7.0
    private let nipHeight = 10.0

    func path(in rect: CGRect) -> Path {
        let bubbleRect = CGRect(
            x: rect.minX,
            y: rect.minY,
            width: max(rect.width - nipWidth, 0),
            height: rect.height
        )
        var path = Path(
            roundedRect: bubbleRect,
            cornerRadius: 8
        )
        let nipX = rect.maxX - nipWidth
        let nipY = rect.midY - nipHeight / 2
        var nip = Path()
        nip.move(to: CGPoint(x: nipX, y: nipY))
        nip.addCurve(
            to: CGPoint(
                x: nipX + nipWidth * 0.85,
                y: nipY + nipHeight / 3
            ),
            control1: CGPoint(x: nipX, y: nipY),
            control2: CGPoint(
                x: nipX + nipWidth * 0.85,
                y: nipY + nipHeight / 3
            )
        )
        nip.addCurve(
            to: CGPoint(
                x: nipX + nipWidth * 0.85,
                y: nipY + nipHeight * 0.67
            ),
            control1: CGPoint(
                x: nipX + nipWidth * 1.05,
                y: nipY + nipHeight * 0.41
            ),
            control2: CGPoint(
                x: nipX + nipWidth * 1.05,
                y: nipY + nipHeight * 0.59
            )
        )
        nip.addCurve(
            to: CGPoint(x: nipX, y: nipY + nipHeight),
            control1: CGPoint(
                x: nipX + nipWidth * 0.85,
                y: nipY + nipHeight * 0.67
            ),
            control2: CGPoint(x: nipX, y: nipY + nipHeight)
        )
        nip.closeSubpath()
        path.addPath(nip)
        return path
    }
}

private struct SearchMessageRow: View {
    let message: SwiftMessageItem
    let mentionNames: [String: String]
    var highlight = ""

    var body: some View {
        HStack(spacing: 12) {
            MixinRemoteImage(url: URL(string: message.senderAvatarUrl)) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Image(systemName: "person.crop.circle.fill")
                    .resizable()
                    .foregroundStyle(.secondary)
            }
            .frame(width: 44, height: 44)
            .clipShape(Circle())

            VStack(alignment: .leading, spacing: 5) {
                HStack {
                    Text(message.senderName)
                        .font(.headline)
                        .lineLimit(1)
                    Spacer()
                    Text(message.formattedTime)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                MessageRichText(
                    content: message.displayText,
                    baseFontSize: 13,
                    color: .secondary,
                    lineLimit: 2,
                    mentionNames: mentionNames,
                    highlight: highlight
                )
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }
}

private struct MessageRow: View, Equatable {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(SettingsPreferencesModel.self) private var preferences
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @State private var qrPresentation: MessageQRPresentation?
    @State private var scanningQRCode = false
    let message: SwiftMessageItem
    let mentionNames: [String: String]
    let audioPlaylist: [SwiftMessageItem]
    let mediaDirectory: URL?
    let conversationName: String?
    let outgoing: Bool
    let sameUserPrevious: Bool
    let sameUserNext: Bool
    let policy: MessageActionPolicy
    let recalledText: String?
    let selected: Bool
    let selectionActive: Bool
    let imageMessages: [SwiftMessageItem]
    let loadImageWindow: (String) async throws -> [SwiftImageMessageItem]
    let onReply: () -> Void
    let onCopy: (String) -> Void
    let onSelect: () -> Void
    let onToggleSelection: () -> Void
    let onTogglePin: () -> Void
    let onRecall: () -> Void
    let onDelete: () -> Void
    let onReedit: (String) -> Void
    let attachmentProgress: Double
    let onAttachmentAction: () -> Void
    let loadTranscript: () async throws -> [SwiftMessageItem]
    let onTranscriptAttachmentAction: (SwiftMessageItem) async -> Void
    let onMarkAudioRead: (String) -> Void
    let onShowStickerDetail: (String) -> Void
    let onAppAction: (String, String) -> Void
    let onStrangerAction: (String) async -> Bool

    static func == (lhs: MessageRow, rhs: MessageRow) -> Bool {
        lhs.message == rhs.message
            && lhs.mentionNames == rhs.mentionNames
            && lhs.audioPlaylist == rhs.audioPlaylist
            && lhs.mediaDirectory == rhs.mediaDirectory
            && lhs.conversationName == rhs.conversationName
            && lhs.outgoing == rhs.outgoing
            && lhs.sameUserPrevious == rhs.sameUserPrevious
            && lhs.sameUserNext == rhs.sameUserNext
            && lhs.policy == rhs.policy
            && lhs.recalledText == rhs.recalledText
            && lhs.selected == rhs.selected
            && lhs.selectionActive == rhs.selectionActive
            && lhs.imageMessages == rhs.imageMessages
            && lhs.attachmentProgress == rhs.attachmentProgress
    }

    var body: some View {
        messageLayout
            .background(selected ? theme.accent.opacity(0.14) : Color.clear)
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .contentShape(Rectangle())
            .animation(
                reduceMotion ? nil : .easeInOut(duration: 0.2),
                value: selectionActive
            )
            .animation(
                reduceMotion ? nil : .easeOut(duration: 0.14),
                value: selected
            )
            .onTapGesture {
                if selectionActive, policy.canSelect {
                    onToggleSelection()
                }
            }
            .contextMenu {
                messageContextMenu
            }
            .sheet(item: $qrPresentation) { presentation in
                MessageQRCodeSheet(presentation: presentation)
            }
    }

    @ViewBuilder
    private var messageLayout: some View {
        if message.isCenteredTimelineMessage {
            HStack {
                MessageSelectionIndicator(
                    selected: selected,
                    visible: selectionActive
                )
                Spacer(minLength: 8)
                messageBody
                    .font(.system(size: 14 + preferences.chatFontSizeDelta))
                Spacer(minLength: 8)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .frame(maxWidth: .infinity)
        } else {
            standardMessageRow
        }
    }

    private var standardMessageRow: some View {
        HStack(alignment: .bottom, spacing: 8) {
            MessageSelectionIndicator(
                selected: selected,
                visible: selectionActive
            )

            if outgoing {
                Spacer(minLength: 65)
            } else if preferences.showAvatar, !sameUserPrevious {
                avatar
            } else if preferences.showAvatar {
                Spacer()
                    .frame(width: 32)
            }

            VStack(alignment: outgoing ? .trailing : .leading, spacing: 4) {
                if !outgoing, !sameUserPrevious {
                    HStack(alignment: .firstTextBaseline, spacing: 3) {
                        Text(message.senderName)
                            .foregroundStyle(theme.secondaryText)
                        if preferences.showIdentityNumber,
                           let identity = message.senderIdentityNumber,
                           !identity.isEmpty,
                           identity != "0"
                        {
                            Text("@\(identity)")
                                .font(.caption2)
                                .foregroundStyle(theme.secondaryText.opacity(0.75))
                        }
                    }
                    .font(.system(size: 12))
                }
                messageBubble
                if message.usesOuterMetadata {
                    messageMetadata
                        .padding(.horizontal, 10)
                        .padding(.top, 4)
                }
            }

            if !outgoing {
                Spacer(minLength: 65)
            }
        }
        .padding(.horizontal, outgoing ? 16 : 8)
        .padding(.top, sameUserPrevious ? 0 : 8)
        .padding(.vertical, 2)
    }

    private var messageBubble: some View {
        Group {
            if message.hidesMetadata {
                messageBody
            } else if message.overlaysMetadata {
                messageBody
                    .overlay(alignment: .bottomTrailing) {
                        messageMetadata
                            .foregroundStyle(.white)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 3)
                            .background(.black.opacity(0.3), in: Capsule())
                            .padding(6)
                    }
            } else if message.usesOuterMetadata {
                messageBody
            } else {
                ViewThatFits(in: .horizontal) {
                    HStack(alignment: .lastTextBaseline, spacing: 6) {
                        messageBody
                        messageMetadata
                    }
                    VStack(alignment: .trailing, spacing: 2) {
                        messageBody
                        messageMetadata
                    }
                }
            }
        }
        .font(.system(size: 16 + preferences.chatFontSizeDelta))
        .padding(message.messageContentInsets)
        .background(
            message.showsBubbleSurface ? messageBubbleColor : Color.clear
        )
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }

    private var messageMetadata: some View {
        HStack(spacing: 4) {
            if message.pinned {
                Image(systemName: "pin.fill")
                    .font(.system(size: 8))
            }
            Text(message.formattedTime)
            if outgoing {
                if let assetName = message.statusAssetName {
                    Image(assetName)
                        .resizable()
                        .renderingMode(.template)
                        .scaledToFit()
                        .frame(width: 14, height: 8)
                        .foregroundStyle(
                            message.status.uppercased() == "READ"
                                ? theme.accent
                                : theme.secondaryText
                        )
                } else {
                    Image(systemName: message.statusIcon)
                        .font(.system(size: 9))
                }
            }
        }
        .font(.system(size: 10))
        .foregroundStyle(
            Color(
                red: colorScheme == .dark ? 128 / 255 : 131 / 255,
                green: colorScheme == .dark ? 131 / 255 : 145 / 255,
                blue: colorScheme == .dark ? 134 / 255 : 158 / 255
            )
        )
        .fixedSize()
    }

    private var messageBubbleColor: Color {
        if outgoing, !message.forcesIncomingBubbleColor {
            return colorScheme == .dark
                ? Color(red: 59 / 255, green: 79 / 255, blue: 103 / 255)
                : Color(red: 197 / 255, green: 237 / 255, blue: 253 / 255)
        }
        return colorScheme == .dark
            ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
            : .white
    }

    @ViewBuilder
    private var messageContextMenu: some View {
        if !selectionActive, policy.allowsMessageActions {
                if policy.canReply {
                    Button("Reply", systemImage: "arrowshape.turn.up.left") {
                        onReply()
                    }
                }
                if let text = policy.copyableText {
                    Button("Copy", systemImage: "doc.on.doc") {
                        onCopy(text)
                    }
                    if message.category.hasSuffix("_TEXT") {
                        Button("Generate QR Code", systemImage: "qrcode") {
                            qrPresentation = .generated(text)
                        }
                    }
                }
                if message.mediaStatus.isComplete, message.localMediaURL != nil {
                    if message.category.hasSuffix("_IMAGE") {
                        Button("Copy Image", systemImage: "photo.on.rectangle") {
                            MessageMediaInteraction.copyImage(message)
                        }
                        Button("Scan QR Code", systemImage: "qrcode.viewfinder") {
                            scanImageQRCode()
                        }
                        .disabled(scanningQRCode)
                    }
                    if message.category.hasSuffix("_IMAGE")
                        || message.category.hasSuffix("_VIDEO")
                        || message.category.hasSuffix("_DATA")
                    {
                        Button("Save Attachment As…", systemImage: "square.and.arrow.down") {
                            MessageMediaInteraction.save(message)
                        }
                    }
                    if message.category.hasSuffix("_DATA") {
                        Button("Show in Finder", systemImage: "finder") {
                            MessageMediaInteraction.reveal(message)
                        }
                    }
                }
                if policy.canSelect {
                    Button("Select", systemImage: "checkmark.circle") {
                        onSelect()
                    }
                }
                if policy.canPin {
                    Divider()
                    Button(
                        message.pinned ? "Unpin" : "Pin",
                        systemImage: message.pinned ? "pin.slash" : "pin"
                    ) {
                        onTogglePin()
                    }
                }
                Divider()
                if policy.canRecall {
                    Button("Delete for Everyone", systemImage: "trash", role: .destructive) {
                        onRecall()
                    }
                }
                if policy.canDelete {
                    Button("Delete for Me", systemImage: "trash", role: .destructive) {
                        onDelete()
                    }
                }
        }
    }

    private var avatar: some View {
        MixinRemoteImage(url: URL(string: message.senderAvatarUrl)) { image in
            image.resizable().scaledToFill()
        } placeholder: {
            Image(systemName: "person.crop.circle.fill")
                .resizable()
                .foregroundStyle(.secondary)
        }
        .frame(width: 32, height: 32)
        .clipShape(Circle())
    }

    @ViewBuilder
    private var messageBody: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let quote = message.quoteContent, !quote.isEmpty {
                MessageRichText(
                    content: quote,
                    baseFontSize: 12,
                    color: .secondary,
                    lineLimit: 2,
                    mentionNames: mentionNames
                )
                    .padding(.leading, 7)
                    .overlay(alignment: .leading) {
                        Capsule()
                            .fill(Color.accentColor)
                            .frame(width: 3)
                    }
            }

            MessageContentView(
                message: message,
                mentionNames: mentionNames,
                audioPlaylist: audioPlaylist,
                mediaDirectory: mediaDirectory,
                conversationName: conversationName,
                outgoing: outgoing,
                recalledText: recalledText,
                imageMessages: imageMessages,
                loadImageWindow: loadImageWindow,
                attachmentProgress: attachmentProgress,
                onAttachmentAction: onAttachmentAction,
                loadTranscript: loadTranscript,
                onTranscriptAttachmentAction: onTranscriptAttachmentAction,
                onReedit: onReedit,
                onMarkAudioRead: onMarkAudioRead,
                onShowStickerDetail: onShowStickerDetail,
                onAppAction: onAppAction,
                onStrangerAction: onStrangerAction
            )
        }
    }

    private func scanImageQRCode() {
        guard !scanningQRCode, let url = message.localMediaURL else {
            return
        }
        scanningQRCode = true
        Task {
            defer { scanningQRCode = false }
            do {
                let contents = try await MessageQRScanner.scan(imageAt: url)
                qrPresentation = contents.isEmpty
                    ? .detectionFailed
                    : .detected(contents)
            } catch {
                qrPresentation = .detectionFailed
            }
        }
    }
}

private struct MessageSelectionIndicator: View {
    let selected: Bool
    let visible: Bool

    var body: some View {
        Image(systemName: selected ? "checkmark.circle.fill" : "circle")
            .font(.system(size: 17, weight: .semibold))
            .foregroundStyle(selected ? Color.accentColor : Color.secondary)
            .symbolEffect(.bounce, value: selected)
            .frame(width: visible ? 32 : 0)
            .opacity(visible ? 1 : 0)
            .scaleEffect(visible ? 1 : 0.75)
            .clipped()
            .accessibilityHidden(!visible)
    }
}

@MainActor
@Observable
final class ChatTimelineModel {
    private struct RecalledText {
        let content: String
        let expiresAt: Date
    }

    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var messages: [SwiftMessageItem] = []
    fileprivate var rows: [ChatTimelineRow] = []
    private(set) var imageMessages: [SwiftMessageItem] = []
    private(set) var audioMessages: [SwiftMessageItem] = []
    private(set) var hasOlderMessages = false
    private(set) var hasNewerMessages = false
    private(set) var initialScrollMessageID: String?
    private(set) var sending = false
    private(set) var sendError: String?
    private(set) var searchResults: [SwiftMessageItem] = []
    private(set) var searching = false
    private(set) var searchError: String?
    private(set) var hasMoreSearchResults = false
    private(set) var mentionNames: [String: String] = [:]
    private(set) var currentUserRole: String?
    private(set) var pinnedMessages: [SwiftMessageItem] = []
    private(set) var unreadMentionMessageIDs: [String] = []
    private(set) var pinnedPreviewDismissed = false
    private(set) var mutating = false
    private(set) var mutationError: String?
    private(set) var mediaDirectory: URL?
    private var account: SwiftAccountHandle?
    private var conversationID: String?
    private var conversationSubscription: SwiftConversationSubscription?
    private var conversationSubscriptionTask: Task<Void, Never>?
    private var messageSubscription: SwiftMessageSubscription?
    private var messageSubscriptionTask: Task<Void, Never>?
    private var requestVersion = 0
    private var windowVersion = 0
    private var searchVersion = 0
    private var mentionRevision = 0
    private var mentionContentsKey = ""
    private(set) var loadingOlder = false
    private(set) var loadingNewer = false
    private var refreshInFlight = false
    private var refreshIncludesRecent = false
    private var refreshPending = false
    private var refreshRecentPending = false
    private var pinnedMessagesLoaded = false
    private var pinnedPreviewPreferenceKey: String?
    private var recalledTexts: [String: RecalledText] = [:]
    private var recalledTextExpiryTask: Task<Void, Never>?

    private static let recalledTextLimit = 100
    private static let recalledTextLifetime: TimeInterval = 6 * 60
    private static let pageSize = 60
    private static let maximumWindowSize = 240

    func recalledText(messageID: String) -> String? {
        guard let value = recalledTexts[messageID],
              value.expiresAt > Date()
        else {
            return nil
        }
        return value.content
    }

    func start(
        account: SwiftAccountHandle,
        conversationID: String,
        lastReadMessageID: String?,
        unseenCount: Int64
    ) async {
        stop()
        self.account = account
        self.conversationID = conversationID
        pinnedMessages = []
        pinnedMessagesLoaded = false
        let pinnedPreviewPreferenceKey = "show_pin_message_\(conversationID)"
        self.pinnedPreviewPreferenceKey = pinnedPreviewPreferenceKey
        pinnedPreviewDismissed = UserDefaults.standard.object(
            forKey: pinnedPreviewPreferenceKey
        ) == nil
            ? false
            : !UserDefaults.standard.bool(forKey: pinnedPreviewPreferenceKey)
        mediaDirectory = (try? account.mediaDirectory()).map {
            URL(fileURLWithPath: $0, isDirectory: true)
        }
        state = .loading

        let conversationSubscription = account.conversationChanges()
        self.conversationSubscription = conversationSubscription
        conversationSubscriptionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let event = await conversationSubscription.next()
            {
                guard let self,
                      event.reloadAll || event.conversationIds.contains(conversationID)
                else {
                    continue
                }
                await self.refreshChangedMessages(includeRecent: true)
                await self.reloadTimelineActions()
            }
        }
        let messageSubscription = account.messageChanges()
        self.messageSubscription = messageSubscription
        messageSubscriptionTask = Task { [weak self] in
            while !Task.isCancelled, await messageSubscription.next() != nil {
                guard let self, self.needsMessageRevisionRefresh else {
                    continue
                }
                await self.refreshChangedMessages(includeRecent: false)
            }
        }
        do {
            let role = try await account.currentUserRole(
                conversationId: conversationID
            )
            guard self.conversationID == conversationID else {
                return
            }
            currentUserRole = role
        } catch {
            guard self.conversationID == conversationID else {
                return
            }
            currentUserRole = nil
        }
        let loadedUnreadWindow: Bool
        if unseenCount > 0, let lastReadMessageID {
            loadedUnreadWindow = await loadUnreadWindow(
                lastReadMessageID: lastReadMessageID,
                unseenCount: unseenCount
            )
        } else {
            loadedUnreadWindow = false
        }
        if !loadedUnreadWindow {
            await loadLatest(limit: Self.pageSize, markRead: true)
        }
        await reloadTimelineActions()
    }

    func reload() async {
        if messages.isEmpty {
            await loadLatest(limit: Self.pageSize, markRead: true)
        } else {
            await refreshChangedMessages(includeRecent: true)
        }
        await reloadTimelineActions()
    }

    func jumpToLatest() async {
        await loadLatest(limit: Self.pageSize, markRead: true)
        await reloadTimelineActions()
    }

    func dismissPinnedPreview() {
        pinnedPreviewDismissed = true
        if let pinnedPreviewPreferenceKey {
            UserDefaults.standard.set(false, forKey: pinnedPreviewPreferenceKey)
        }
    }

    func markMentionRead(messageID: String) async {
        guard let account, let conversationID else {
            return
        }
        do {
            try await account.markMentionRead(
                conversationId: conversationID,
                messageId: messageID
            )
            unreadMentionMessageIDs.removeAll { $0 == messageID }
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func clearUnreadMentions() async {
        for messageID in unreadMentionMessageIDs {
            await markMentionRead(messageID: messageID)
        }
    }

    func send(
        _ content: String,
        quoteMessageID: String?,
        silent: Bool
    ) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        do {
            _ = try await account.sendText(
                conversationId: conversationID,
                content: content,
                quoteMessageId: quoteMessageID,
                silent: silent
            )
            try await account.updateDraft(
                conversationId: conversationID,
                draft: ""
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func sendPost(_ content: String) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        do {
            _ = try await account.sendPost(
                conversationId: conversationID,
                content: content
            )
            try await account.updateDraft(
                conversationId: conversationID,
                draft: ""
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func imageMessagesAround(
        messageID: String
    ) async throws -> [SwiftImageMessageItem] {
        guard let account, let conversationID else {
            return []
        }
        return try await account.imageMessagesAround(
            conversationId: conversationID,
            targetMessageId: messageID,
            before: 40,
            after: 40
        )
    }

    func openMessageAction(
        _ action: String,
        title: String,
        currency: String
    ) async {
        guard let account, let conversationID else {
            return
        }
        do {
            try await MessageActionHandler(
                account: account,
                conversationID: conversationID,
                currency: currency
            ).open(action, title: title)
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func handleStrangerAction(
        _ action: String,
        message: SwiftMessageItem,
        currency: String
    ) async -> Bool {
        guard !mutating, let account, let conversationID else {
            return false
        }
        mutating = true
        mutationError = nil
        defer { mutating = false }

        do {
            switch action {
            case "block":
                try await account.blockUser(userId: message.senderId)
                await reload()
            case "add_contact":
                try await account.addContact(
                    userId: message.senderId,
                    fullName: message.senderName
                )
                await reload()
            case "say_hi":
                _ = try await account.sendText(
                    conversationId: conversationID,
                    content: "Hi",
                    quoteMessageId: nil,
                    silent: false
                )
                await jumpToLatest()
            case "open_home":
                guard let appID = message.senderAppId?
                    .trimmingCharacters(in: .whitespacesAndNewlines),
                    !appID.isEmpty
                else {
                    return false
                }
                try await MessageActionHandler(
                    account: account,
                    conversationID: conversationID,
                    currency: currency
                ).openBotHome(appID: appID, title: message.senderName)
            default:
                return false
            }
            return true
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func forward(
        messageIDs: [String],
        targetConversationID: String,
        combined: Bool
    ) async -> Bool {
        guard !messageIDs.isEmpty, !mutating, let account else {
            return false
        }
        mutating = true
        mutationError = nil
        defer { mutating = false }
        do {
            if combined {
                guard messageIDs.count >= 2, messageIDs.count < 100 else {
                    return false
                }
                _ = try await account.combineForwardMessages(
                    targetConversationId: targetConversationID,
                    sourceMessageIds: messageIDs
                )
            } else {
                _ = try await account.forwardMessages(
                    targetConversationId: targetConversationID,
                    sourceMessageIds: messageIDs
                )
            }
            return true
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func openBotHome(
        appID: String,
        title: String,
        currency: String
    ) async {
        guard let account, let conversationID else {
            return
        }
        do {
            try await MessageActionHandler(
                account: account,
                conversationID: conversationID,
                currency: currency
            ).openBotHome(appID: appID, title: title)
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func sendAudio(
        _ recording: VoiceRecording,
        quoteMessageID: String?
    ) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        do {
            _ = try await account.sendAudio(
                conversationId: conversationID,
                path: recording.url.path,
                durationMillis: recording.durationMillis,
                waveform: Data(recording.waveform),
                quoteMessageId: quoteMessageID
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func sendAttachment(
        _ request: AttachmentSendRequest,
        quoteMessageID: String?
    ) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        let securityScoped = request.url.startAccessingSecurityScopedResource()
        defer {
            if securityScoped {
                request.url.stopAccessingSecurityScopedResource()
            }
        }
        let videoMetadata = await request.videoMetadata()
        let dimensions: (width: Int32, height: Int32)? = request.dimensions
            ?? videoMetadata.map { (width: $0.width, height: $0.height) }
        do {
            _ = try await account.sendAttachment(
                conversationId: conversationID,
                path: request.url.path,
                kind: request.kind,
                mimeType: request.mimeType,
                name: request.url.lastPathComponent,
                width: dimensions?.width,
                height: dimensions?.height,
                durationMillis: videoMetadata?.durationMillis,
                thumbnail: request.thumbnail,
                caption: request.caption,
                quoteMessageId: quoteMessageID,
                silent: request.silent
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func sendContact(userID: String, quoteMessageID: String?) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        do {
            _ = try await account.sendContact(
                conversationId: conversationID,
                sharedUserId: userID,
                quoteMessageId: quoteMessageID,
                silent: false
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func sendSticker(stickerID: String) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        do {
            _ = try await account.sendSticker(
                conversationId: conversationID,
                stickerId: stickerID
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func sendRemoteGIF(_ gif: GiphyItem) async -> Bool {
        guard !sending, let account, let conversationID else {
            return false
        }
        sending = true
        sendError = nil
        defer { sending = false }
        do {
            _ = try await account.sendRemoteImage(
                conversationId: conversationID,
                url: gif.url,
                previewUrl: gif.previewURL,
                width: gif.width,
                height: gif.height,
                mimeType: "image/gif",
                silent: false
            )
            await jumpToLatest()
            return true
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func attachmentProgress(messageID: String) -> Double {
        account?.attachmentProgress(messageId: messageID) ?? 0
    }

    func performAttachmentAction(
        _ message: SwiftMessageItem,
        sentByCurrentUser: Bool
    ) async {
        guard !mutating, let account else {
            return
        }
        mutating = true
        mutationError = nil
        defer { mutating = false }
        do {
            switch message.mediaStatus.uppercased() {
            case "CANCELED":
                if sentByCurrentUser {
                    try await account.retryAttachment(messageId: message.messageId)
                } else {
                    try await account.downloadAttachment(messageId: message.messageId)
                }
            case "PENDING":
                try await account.cancelAttachment(messageId: message.messageId)
            default:
                return
            }
            await reload()
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func markAudioRead(messageID: String) async {
        guard let account else {
            return
        }
        do {
            try await account.markAudioRead(messageId: messageID)
            await reload()
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func transcriptMessages(transcriptID: String) async throws -> [SwiftMessageItem] {
        guard let account else {
            return []
        }
        return try await account.transcriptMessages(transcriptId: transcriptID)
    }

    func performTranscriptAttachmentAction(
        transcriptID: String,
        message: SwiftMessageItem,
        sentByCurrentUser: Bool
    ) async {
        guard let account else {
            return
        }
        do {
            switch message.mediaStatus.uppercased() {
            case "CANCELED":
                if sentByCurrentUser {
                    try await account.retryTranscriptAttachment(
                        transcriptId: transcriptID
                    )
                } else {
                    try await account.downloadTranscriptAttachment(
                        transcriptId: transcriptID,
                        messageId: message.messageId
                    )
                }
            case "PENDING":
                try await account.cancelTranscriptAttachment(
                    transcriptId: transcriptID,
                    messageId: message.messageId
                )
            default:
                break
            }
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func setPinned(messageID: String, pinned: Bool) async {
        guard !mutating, let account, let conversationID else {
            return
        }
        mutating = true
        mutationError = nil
        defer { mutating = false }
        do {
            try await account.setMessagePinned(
                conversationId: conversationID,
                messageId: messageID,
                pinned: pinned
            )
            await reload()
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func delete(messageIDs: [String]) async -> Bool {
        await mutate(messageIDs: messageIDs) { account, conversationID, messageIDs in
            try await account.deleteMessages(
                conversationId: conversationID,
                messageIds: messageIDs
            )
        }
    }

    func recall(messages: [SwiftMessageItem]) async -> Bool {
        guard !messages.isEmpty,
              !mutating,
              let account,
              let conversationID
        else {
            return false
        }
        mutating = true
        mutationError = nil
        defer { mutating = false }
        do {
            try await account.recallMessages(
                conversationId: conversationID,
                messageIds: messages.map(\.messageId)
            )
            retainRecalledText(from: messages)
            await reload()
            return true
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func clearMutationError() {
        mutationError = nil
    }

    func saveDraft(_ draft: String) async {
        guard let account, let conversationID else {
            return
        }
        do {
            try await account.updateDraft(
                conversationId: conversationID,
                draft: draft
            )
        } catch {
            sendError = MixinErrorPresenter.message(for: error)
        }
    }

    func search(
        query: String,
        senderID: String?,
        categories: [String],
        append: Bool
    ) async {
        guard let account, let conversationID, !query.isEmpty else {
            clearSearch()
            return
        }
        if append, searching || !hasMoreSearchResults {
            return
        }
        searchVersion += 1
        let version = searchVersion
        searching = true
        searchError = nil
        let anchor = append ? searchResults.last?.messageId : nil
        do {
            let items = try await account.searchMessages(
                conversationId: conversationID,
                query: query,
                senderId: senderID,
                categories: categories,
                anchorMessageId: anchor,
                limit: 60
            )
            guard version == searchVersion, self.conversationID == conversationID else {
                return
            }
            searchResults = append ? deduplicated(searchResults + items) : items
            hasMoreSearchResults = items.count == 60
            searching = false
            await reloadMentionNames()
        } catch {
            guard version == searchVersion else {
                return
            }
            searching = false
            searchError = MixinErrorPresenter.message(for: error)
        }
    }

    func clearSearch() {
        searchVersion += 1
        searchResults = []
        searching = false
        searchError = nil
        hasMoreSearchResults = false
    }

    func locate(messageID: String) async -> Bool {
        guard let account, let conversationID else {
            return false
        }
        do {
            let items = try await account.messagesAround(
                conversationId: conversationID,
                targetMessageId: messageID,
                before: 30,
                after: 30
            )
            guard self.conversationID == conversationID,
                  items.contains(where: { $0.messageId == messageID })
            else {
                return false
            }
            replaceMessages(items.sorted(by: Self.isEarlier))
            hasOlderMessages = items.count >= 31
            let targetIndex = messages.firstIndex { $0.messageId == messageID }
            hasNewerMessages = targetIndex.map {
                messages.count - $0 - 1 >= 30
            } ?? false
            state = .ready
            initialScrollMessageID = messageID
            await reloadMentionNames()
            return true
        } catch {
            searchError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func loadOlder() async {
        guard !loadingOlder,
              hasOlderMessages,
              let account,
              let conversationID,
              let oldest = messages.first
        else {
            return
        }
        loadingOlder = true
        defer { loadingOlder = false }

        do {
            let older = try await account.messages(
                conversationId: conversationID,
                beforeCreatedAtMicros: oldest.createdAtMicros,
                beforeMessageId: oldest.messageId,
                limit: Int64(Self.pageSize)
            )
            hasOlderMessages = older.count == Self.pageSize
            merge(older, keeping: .oldest)
            await reloadMentionNames()
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func loadNewer() async {
        guard !loadingNewer,
              hasNewerMessages,
              let account,
              let conversationID,
              let newest = messages.last
        else {
            return
        }
        loadingNewer = true
        defer { loadingNewer = false }

        do {
            let page = try await account.messagesAround(
                conversationId: conversationID,
                targetMessageId: newest.messageId,
                before: 0,
                after: Int64(Self.pageSize)
            )
            let newer = page.filter { $0.messageId != newest.messageId }
            hasNewerMessages = newer.count == Self.pageSize
            merge(newer, keeping: .newest)
            await reloadMentionNames()
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func stop() {
        requestVersion += 1
        windowVersion += 1
        searchVersion += 1
        mentionRevision += 1
        conversationSubscriptionTask?.cancel()
        conversationSubscriptionTask = nil
        conversationSubscription?.cancel()
        conversationSubscription = nil
        messageSubscriptionTask?.cancel()
        messageSubscriptionTask = nil
        messageSubscription?.cancel()
        messageSubscription = nil
        recalledTextExpiryTask?.cancel()
        recalledTextExpiryTask = nil
        refreshInFlight = false
        refreshIncludesRecent = false
        refreshPending = false
        refreshRecentPending = false
        account = nil
        conversationID = nil
        mediaDirectory = nil
        currentUserRole = nil
        mentionNames = [:]
        mentionContentsKey = ""
        pinnedMessagesLoaded = false
        pinnedPreviewPreferenceKey = nil
        recalledTexts = [:]
    }

    private func retainRecalledText(from messages: [SwiftMessageItem]) {
        let expiresAt = Date().addingTimeInterval(Self.recalledTextLifetime)
        for message in messages where message.category.hasSuffix("_TEXT") {
            recalledTexts[message.messageId] = RecalledText(
                content: message.content,
                expiresAt: expiresAt
            )
        }
        if recalledTexts.count > Self.recalledTextLimit {
            let overflow = recalledTexts.count - Self.recalledTextLimit
            for messageID in recalledTexts
                .sorted(by: { $0.value.expiresAt < $1.value.expiresAt })
                .prefix(overflow)
                .map(\.key)
            {
                recalledTexts.removeValue(forKey: messageID)
            }
        }
        scheduleRecalledTextExpiry()
    }

    private func scheduleRecalledTextExpiry() {
        recalledTextExpiryTask?.cancel()
        guard let expiresAt = recalledTexts.values.map(\.expiresAt).min() else {
            recalledTextExpiryTask = nil
            return
        }
        let delay = max(expiresAt.timeIntervalSinceNow, 0)
        recalledTextExpiryTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(delay))
            guard !Task.isCancelled, let self else {
                return
            }
            let now = Date()
            recalledTexts = recalledTexts.filter { $0.value.expiresAt > now }
            scheduleRecalledTextExpiry()
        }
    }

    private func mutate(
        messageIDs: [String],
        operation: (
            SwiftAccountHandle,
            String,
            [String]
        ) async throws -> Void
    ) async -> Bool {
        guard !messageIDs.isEmpty,
              !mutating,
              let account,
              let conversationID
        else {
            return false
        }
        mutating = true
        mutationError = nil
        defer { mutating = false }
        do {
            try await operation(account, conversationID, messageIDs)
            await reload()
            return true
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    private func loadLatest(limit: Int, markRead: Bool) async {
        guard let account, let conversationID else {
            return
        }
        requestVersion += 1
        let version = requestVersion
        do {
            let items = try await account.messages(
                conversationId: conversationID,
                beforeCreatedAtMicros: nil,
                beforeMessageId: nil,
                limit: Int64(limit)
            )
            guard version == requestVersion, self.conversationID == conversationID else {
                return
            }
            replaceMessages(items.sorted(by: Self.isEarlier))
            hasOlderMessages = items.count == limit
            hasNewerMessages = false
            state = .ready
            initialScrollMessageID = messages.last?.messageId
            await reloadMentionNames()
            if markRead {
                try await account.markConversationRead(conversationId: conversationID)
            }
        } catch {
            guard version == requestVersion else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    private func loadUnreadWindow(
        lastReadMessageID: String,
        unseenCount: Int64
    ) async -> Bool {
        guard let account, let conversationID else {
            return false
        }
        requestVersion += 1
        let version = requestVersion
        do {
            let items = try await account.messagesAround(
                conversationId: conversationID,
                targetMessageId: lastReadMessageID,
                before: 30,
                after: min(max(unseenCount + 10, 60), 200)
            )
            guard version == requestVersion,
                  self.conversationID == conversationID,
                  let readIndex = items.firstIndex(where: {
                      $0.messageId == lastReadMessageID
                  })
            else {
                return false
            }
            replaceMessages(items.sorted(by: Self.isEarlier))
            hasOlderMessages = readIndex >= 30
            let requestedAfter = min(max(unseenCount + 10, 60), 200)
            let sortedReadIndex = messages.firstIndex {
                $0.messageId == lastReadMessageID
            }
            hasNewerMessages = sortedReadIndex.map {
                messages.count - $0 - 1 >= Int(requestedAfter)
            } ?? false
            state = .ready
            initialScrollMessageID = messages.first(where: {
                Self.isEarlier(items[readIndex], $0)
            })?.messageId ?? messages.last?.messageId
            try await account.markConversationRead(conversationId: conversationID)
            return true
        } catch {
            guard version == requestVersion else {
                return false
            }
            return false
        }
    }

    private func reloadTimelineActions() async {
        guard let account, let conversationID else {
            return
        }
        async let pinned = account.pinnedMessages(conversationId: conversationID)
        async let mentions = account.unreadMentionMessageIds(
            conversationId: conversationID
        )
        do {
            let (pinned, mentions) = try await (pinned, mentions)
            guard self.conversationID == conversationID else {
                return
            }
            let previousMessageIDs = Set(pinnedMessages.map(\.messageId))
            let pinnedMessageIDs = Set(pinned.map(\.messageId))
            if pinnedMessagesLoaded,
               !pinnedMessageIDs.subtracting(previousMessageIDs).isEmpty
            {
                pinnedPreviewDismissed = false
                if let pinnedPreviewPreferenceKey {
                    UserDefaults.standard.set(
                        true,
                        forKey: pinnedPreviewPreferenceKey
                    )
                }
            }
            pinnedMessages = pinned
            pinnedMessagesLoaded = true
            unreadMentionMessageIDs = mentions
            await reloadMentionNames()
        } catch {
            guard self.conversationID == conversationID else {
                return
            }
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func refreshChangedMessages(includeRecent: Bool) async {
        refreshRecentPending = refreshRecentPending || includeRecent
        if refreshInFlight {
            if includeRecent || !refreshIncludesRecent {
                refreshPending = true
            }
            return
        }

        refreshInFlight = true
        repeat {
            refreshPending = false
            let includeRecent = refreshRecentPending
            refreshRecentPending = false
            refreshIncludesRecent = includeRecent
            await performChangedMessageRefresh(includeRecent: includeRecent)
        } while refreshPending
        refreshIncludesRecent = false
        refreshInFlight = false
    }

    private func performChangedMessageRefresh(includeRecent: Bool) async {
        guard let account,
              let conversationID,
              !messages.isEmpty
        else {
            return
        }
        let version = windowVersion
        let currentMessages = messages
        let currentIDs = currentMessages.map(\.messageId)

        do {
            async let loaded = account.messageItemsByIds(messageIds: currentIDs)
            let recentItems: [SwiftMessageItem]
            if includeRecent, !hasNewerMessages {
                recentItems = try await account.messages(
                    conversationId: conversationID,
                    beforeCreatedAtMicros: nil,
                    beforeMessageId: nil,
                    limit: Int64(Self.pageSize)
                )
            } else {
                recentItems = []
            }
            let loadedItems = try await loaded
            guard version == windowVersion,
                  self.conversationID == conversationID
            else {
                refreshPending = true
                refreshRecentPending = refreshRecentPending || includeRecent
                return
            }

            let loadedByID = Dictionary(
                uniqueKeysWithValues: loadedItems.map { ($0.messageId, $0) }
            )
            let refreshed = currentIDs.compactMap { loadedByID[$0] }
            if includeRecent, !hasNewerMessages {
                var seen = Set<String>()
                var merged = (refreshed + recentItems)
                    .filter { seen.insert($0.messageId).inserted }
                    .sorted(by: Self.isEarlier)
                if merged.count > Self.maximumWindowSize {
                    merged.removeFirst(merged.count - Self.maximumWindowSize)
                    hasOlderMessages = true
                }
                replaceMessages(merged)
                hasNewerMessages = false
            } else {
                replaceMessages(refreshed)
            }
            await reloadMentionNames()
        } catch {
            guard self.conversationID == conversationID else {
                return
            }
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    private enum WindowEdge {
        case oldest
        case newest
    }

    private func merge(_ items: [SwiftMessageItem], keeping edge: WindowEdge) {
        let combined = items + messages
        var seen = Set<String>()
        var merged = combined
            .filter { seen.insert($0.messageId).inserted }
            .sorted(by: Self.isEarlier)
        if merged.count > Self.maximumWindowSize {
            let overflow = merged.count - Self.maximumWindowSize
            switch edge {
            case .oldest:
                merged.removeLast(overflow)
                hasNewerMessages = true
            case .newest:
                merged.removeFirst(overflow)
                hasOlderMessages = true
            }
        }
        replaceMessages(merged)
    }

    private func replaceMessages(_ items: [SwiftMessageItem]) {
        guard items != messages else {
            return
        }
        messages = items
        windowVersion += 1
        rows = items.enumerated().map { index, message in
            let previous = index > 0 ? items[index - 1] : nil
            let next = index + 1 < items.count ? items[index + 1] : nil
            let sameDayPrevious = previous?.createdAtMicros.calendarDay
                == message.createdAtMicros.calendarDay
            let sameDayNext = next?.createdAtMicros.calendarDay
                == message.createdAtMicros.calendarDay
            return ChatTimelineRow(
                message: message,
                startsNewDay: !sameDayPrevious,
                sameUserPrevious: sameDayPrevious
                    && previous?.senderId == message.senderId
                    && previous?.breaksMessageGrouping == false,
                sameUserNext: sameDayNext
                    && next?.senderId == message.senderId
                    && !message.breaksMessageGrouping
            )
        }
        imageMessages = items.filter {
            $0.category.hasSuffix("_IMAGE")
                && $0.mediaStatus.isComplete
                && $0.localMediaURL != nil
        }
        audioMessages = items.filter {
            $0.category.hasSuffix("_AUDIO")
                && ["DONE", "READ"].contains($0.mediaStatus.uppercased())
        }
    }

    private var needsMessageRevisionRefresh: Bool {
        messages.contains { message in
            message.mediaStatus.uppercased() == "PENDING"
                || (message.presentationKind == .sticker
                    && message.presentationImageURL == nil)
        }
    }

    private func reloadMentionNames() async {
        guard let account, let conversationID else {
            return
        }
        let contents = (messages + searchResults + pinnedMessages)
            .flatMap { message in
                [message.content, message.caption, message.quoteContent]
                    .compactMap { $0 }
            }
            .filter { $0.range(of: #"@\d{4,}"#, options: .regularExpression) != nil }
        let key = Array(Set(contents)).sorted().joined(separator: "\u{0}")
        guard key != mentionContentsKey else {
            return
        }
        mentionContentsKey = key
        mentionRevision += 1
        let revision = mentionRevision
        guard !contents.isEmpty else {
            mentionNames = [:]
            return
        }
        do {
            let names = try await account.mentionNames(contents: contents)
            guard revision == mentionRevision,
                  self.conversationID == conversationID
            else {
                return
            }
            mentionNames = names
        } catch {
            guard revision == mentionRevision,
                  self.conversationID == conversationID
            else {
                return
            }
            mentionContentsKey = ""
            mentionNames = [:]
        }
    }

    private func deduplicated(_ items: [SwiftMessageItem]) -> [SwiftMessageItem] {
        var seen = Set<String>()
        return items.filter { seen.insert($0.messageId).inserted }
    }

    private static func isEarlier(_ lhs: SwiftMessageItem, _ rhs: SwiftMessageItem) -> Bool {
        if lhs.createdAtMicros == rhs.createdAtMicros {
            return lhs.messageId < rhs.messageId
        }
        return lhs.createdAtMicros < rhs.createdAtMicros
    }
}

enum MessagePresentationKind {
    case text
    case image
    case sticker
    case video
    case audio
    case file
    case contact
    case special
    case transfer
    case system
    case unknown
}

extension SwiftMessageItem {
    var hasInvalidImagePayload: Bool {
        category.hasSuffix("_IMAGE") && (mediaWidth == nil || mediaHeight == nil)
    }

    var hasInvalidSpecialPayload: Bool {
        let category = category.uppercased()
        guard category.hasSuffix("_LOCATION")
            || category.hasSuffix("_TRANSCRIPT")
            || category == "APP_BUTTON_GROUP"
            || category == "APP_CARD"
        else {
            return false
        }
        guard let data = content.data(using: .utf8),
              let value = try? JSONSerialization.jsonObject(with: data)
        else {
            return true
        }
        if category.hasSuffix("_LOCATION") {
            guard let location = value as? [String: Any] else {
                return true
            }
            return !(location["latitude"] is NSNumber)
                || !(location["longitude"] is NSNumber)
        }
        if category.hasSuffix("_TRANSCRIPT") || category == "APP_BUTTON_GROUP" {
            guard let items = value as? [Any] else {
                return true
            }
            return items.contains { !($0 is [String: Any]) }
        }
        guard let card = value as? [String: Any] else {
            return true
        }
        guard let actions = card["actions"] else {
            return false
        }
        guard let items = actions as? [Any] else {
            return true
        }
        return items.contains { !($0 is [String: Any]) }
    }

    var isRichContent: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_IMAGE")
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_DATA")
            || category.hasSuffix("_TRANSCRIPT")
            || category.hasSuffix("_POST")
            || category.hasSuffix("_LIVE")
    }

    var presentationKind: MessagePresentationKind {
        let category = category.uppercased()
        if [
            "SYSTEM_ACCOUNT_SNAPSHOT",
            "SYSTEM_SAFE_SNAPSHOT",
            "SYSTEM_SAFE_INSCRIPTION",
        ].contains(category) {
            return .special
        }
        if category.hasPrefix("SYSTEM_") {
            return .system
        }
        if category.contains("STICKER") {
            return .sticker
        }
        if category.hasSuffix("_IMAGE") {
            return .image
        }
        if category.hasSuffix("_VIDEO") || category.hasSuffix("_LIVE") {
            return .video
        }
        if category.hasSuffix("_AUDIO") {
            return .audio
        }
        if category.hasSuffix("_DATA") {
            return .file
        }
        if category.hasSuffix("_CONTACT") {
            return .contact
        }
        if category.contains("TRANSFER") || snapshotAmount != nil {
            return .transfer
        }
        if category.hasSuffix("_TEXT") || category.contains("POST") {
            return .text
        }
        return .unknown
    }

    var isCenteredTimelineMessage: Bool {
        switch category.uppercased() {
        case "SYSTEM_CONVERSATION", "MESSAGE_PIN", "SECRET", "STRANGER":
            true
        default:
            false
        }
    }

    var breaksMessageGrouping: Bool {
        isCenteredTimelineMessage || category.uppercased().hasPrefix("SYSTEM_")
    }

    var overlaysMetadata: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_IMAGE")
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_LIVE")
    }

    var usesOuterMetadata: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_STICKER")
            || category.hasSuffix("_LOCATION")
            || category.hasSuffix("_CONTACT")
            || category.hasSuffix("_DATA")
            || category.hasSuffix("_AUDIO")
            || category.hasSuffix("_TRANSCRIPT")
            || category == "APP_CARD"
            || category == "SYSTEM_ACCOUNT_SNAPSHOT"
            || category == "SYSTEM_SAFE_SNAPSHOT"
            || category == "SYSTEM_SAFE_INSCRIPTION"
    }

    var forcesIncomingBubbleColor: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_AUDIO")
            || category == "SYSTEM_SAFE_SNAPSHOT"
            || category == "SYSTEM_SAFE_INSCRIPTION"
    }

    var hidesMetadata: Bool {
        category.uppercased() == "APP_BUTTON_GROUP"
    }

    var showsBubbleSurface: Bool {
        let category = category.uppercased()
        if quoteMessageId?.isEmpty == false || quoteContent?.isEmpty == false {
            return true
        }
        return !category.hasSuffix("_STICKER")
            && category != "APP_BUTTON_GROUP"
    }

    var messageContentInsets: EdgeInsets {
        let category = category.uppercased()
        if category.hasSuffix("_IMAGE")
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_LIVE")
            || category.hasSuffix("_LOCATION")
            || category.hasSuffix("_STICKER")
            || category == "APP_BUTTON_GROUP"
        {
            return EdgeInsets()
        }
        if category.hasSuffix("_TRANSCRIPT") {
            return EdgeInsets(top: 4, leading: 2, bottom: 2, trailing: 2)
        }
        return EdgeInsets(top: 8, leading: 8, bottom: 8, trailing: 8)
    }

    var isStandaloneSpecial: Bool {
        let category = category.uppercased()
        return category == "MESSAGE_PIN"
            || category == "SECRET"
            || category == "STRANGER"
            || category.hasSuffix("_LOCATION")
    }

    var presentationImageURL: URL? {
        [stickerAssetUrl, mediaUrl, thumbUrl]
            .compactMap { $0 }
            .compactMap(URL.init(string:))
            .first
    }

    var displayText: String {
        let text = content.trimmingCharacters(in: .whitespacesAndNewlines)
        if !text.isEmpty {
            return text
        }
        if let caption, !caption.isEmpty {
            return caption
        }
        if let hyperlink, !hyperlink.isEmpty {
            return hyperlink
        }
        return category
    }

    var formattedTime: String {
        Date(timeIntervalSince1970: Double(createdAtMicros) / 1_000_000)
            .formatted(date: .omitted, time: .shortened)
    }

    var selectionFormattedTime: String {
        Date(timeIntervalSince1970: Double(createdAtMicros) / 1_000_000)
            .formatted(date: .numeric, time: .standard)
    }

    var statusIcon: String {
        switch status.uppercased() {
        case "READ":
            "checkmark.circle.fill"
        case "DELIVERED":
            "checkmark.circle"
        case "SENDING":
            "clock"
        case "FAILED":
            "exclamationmark.circle"
        default:
            "checkmark"
        }
    }

    var statusAssetName: String? {
        switch status.uppercased() {
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

    func systemConversationText(currentUserID: String) -> String {
        let sender = senderId == currentUserID ? "You" : senderName
        let participant = participantFullName?.isEmpty == false
            ? participantFullName!
            : "a member"
        switch action?.uppercased() {
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
            if let seconds = Int64(content), seconds > 0 {
                return "\(sender) set disappearing messages to \(seconds.formattedDuration)"
            }
            return "\(sender) disabled disappearing messages"
        default:
            return content.isEmpty ? "Unsupported system message" : content
        }
    }
}

private extension Int64 {
    var messageDate: Date {
        Date(timeIntervalSince1970: Double(self) / 1_000_000)
    }

    var formattedDuration: String {
        if self % 86_400 == 0 {
            return "\(self / 86_400)d"
        }
        if self % 3_600 == 0 {
            return "\(self / 3_600)h"
        }
        if self % 60 == 0 {
            return "\(self / 60)m"
        }
        return "\(self)s"
    }

    var calendarDay: Date {
        Calendar.current.startOfDay(for: messageDate)
    }

    var dayChipTitle: String {
        if Calendar.current.isDateInToday(messageDate) {
            return "Today"
        }
        if Calendar.current.isDateInYesterday(messageDate) {
            return "Yesterday"
        }
        return messageDate.formatted(date: .abbreviated, time: .omitted)
    }
}
