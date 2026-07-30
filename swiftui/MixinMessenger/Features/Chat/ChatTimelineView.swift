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

private struct ChatTimelineRenderKey: Equatable {
    let modelID: ObjectIdentifier
    let conversationID: String
    let conversationName: String?
    let lastReadMessageID: String?
    let unseenCount: Int64
    let selectedMessageIDs: Set<String>
    let showJumpToLatest: Bool
    let newMessageCount: Int
    let highlightedMessageID: String?
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

struct ChatTimelineView: View {
    @Environment(AppModel.self) private var appModel
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @State private var model = ChatTimelineModel()
    @State private var scrollCoordinator = ChatScrollCoordinator()
    @State private var scrollPosition = ScrollPosition(idType: String.self)
    @State private var draft: String
    @State private var searchPresented = false
    @State private var searchQuery = ""
    @State private var searchSender: ConversationParticipantItem?
    @State private var searchCategory = MessageSearchCategory.all
    @State private var searchSenderPresented = false
    @State private var replyMessage: MessageItem?
    @State private var selectedMessageIDs = Set<String>()
    @State private var pendingDeletion: [MessageItem] = []
    @State private var attachmentURLs: [URL] = []
    @State private var contactPickerPresented = false
    @State private var stickerPresented = false
    @State private var stickerButtonHovering = false
    @State private var stickerPanelHovering = false
    @State private var stickerOpenTask: Task<Void, Never>?
    @State private var stickerCloseTask: Task<Void, Never>?
    @State private var stickerDetailID: String?
    @State private var voiceRecorder = VoiceRecorderModel()
    @State private var discardRecordingPresented = false
    @State private var scamWarningVisible: Bool
    @State private var pinnedMessagesPresented = false
    @State private var forwardCombined: Bool?
    @State private var forwardMessageIDs: [String]?
    @State private var composerFocusRevision = 0
    @State private var mentionModel = MentionComposerModel()
    @State private var mentionSelectionRequest: ComposerSelectionRequest?
    @State private var dropTargeted = false
    @FocusState private var searchFocused: Bool
    let conversationID: String
    let conversationName: String?
    let conversationCategory: String?
    let conversationOwnerID: String?
    let conversationIsBot: Bool
    let conversationIsBotGroup: Bool
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
        conversationIsBotGroup: Bool,
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
        self.conversationIsBotGroup = conversationIsBotGroup
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
                        showJumpToLatest: scrollCoordinator.showJumpToLatest,
                        newMessageCount: scrollCoordinator.newMessageCount,
                        highlightedMessageID:
                            scrollCoordinator.highlightedMessageID,
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
                Text("You are no longer a participant in this group.")
                .font(.system(size: 14))
                .foregroundStyle(theme.secondaryText)
                .frame(maxWidth: .infinity)
                .frame(height: 56)
                .background(theme.primary)
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
            scrollPosition = ScrollPosition(idType: String.self)
            scrollCoordinator.reset(conversationID: conversationID)
            loadScamWarningPreference()
            let jumpRequest = navigation.messageJumpRequest.flatMap {
                $0.conversationID == conversationID ? $0 : nil
            }
            let savedPosition = navigation.chatViewportPosition(
                conversationID: conversationID
            )
            let initialAnchor: ChatTimelineAnchor? =
                if let jumpRequest {
                    .message(
                        id: jumpRequest.messageID,
                        alignment: .center,
                        offset: 0,
                        highlight: true
                    )
                } else if let savedPosition {
                    .message(
                        id: savedPosition.messageID,
                        alignment: .top,
                        offset: savedPosition.offset,
                        highlight: false
                    )
                } else {
                    nil
                }
            let resolvedAnchor = await model.start(
                account: session.handle,
                conversationID: conversationID,
                currentUserID: session.profile.userId,
                lastReadMessageID: lastReadMessageID,
                unseenCount: unseenCount,
                initialAnchor: initialAnchor
            )
            if let resolvedAnchor {
                scrollCoordinator.prepareInitialPosition(
                    resolvedAnchor,
                    position: $scrollPosition
                )
                model.presentInitialTimeline()
            }
            if let jumpRequest {
                navigation.consumeMessageJump(revision: jumpRequest.revision)
            }
        }
        .onDisappear {
            navigation.saveChatViewportPosition(
                scrollCoordinator.savedPosition(
                    hasNewerMessages: model.hasNewerMessages
                ),
                conversationID: conversationID
            )
            scrollCoordinator.stop()
            model.stop()
            voiceRecorder.dispose()
            stickerOpenTask?.cancel()
            stickerCloseTask?.cancel()
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
                    .stroke(
                        Color.accentColor,
                        style: StrokeStyle(lineWidth: 2, dash: [7])
                    )
                    .padding(12)
                    .overlay {
                        Label(
                            "Drop files to send",
                            systemImage: "tray.and.arrow.down.fill"
                        )
                        .font(.title3.weight(.semibold))
                    }
                    .allowsHitTesting(false)
            }
        }
        .overlay {
            if voiceRecorder.status == .recording,
               !discardRecordingPresented
            {
                GeometryReader { proxy in
                    Color.clear
                        .contentShape(Rectangle())
                        .onTapGesture {
                            discardRecordingPresented = true
                        }
                        .frame(
                            width: proxy.size.width,
                            height: max(0, proxy.size.height - 56)
                        )
                        .frame(maxHeight: .infinity, alignment: .top)
                }
            }
        }
        .overlay {
            if discardRecordingPresented {
                discardRecordingDialog
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
        .task(
            id: MessageSearchTaskID(
                query: searchQuery,
                senderID: searchSender?.userId,
                category: searchCategory
            )
        ) {
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
        .sheet(
            isPresented: Binding(
                get: { !attachmentURLs.isEmpty },
                set: { if !$0 { attachmentURLs = [] } }
            )
        ) {
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
                AppListView(model.pinnedMessages, id: \.messageId) { message in
                    Button {
                        pinnedMessagesPresented = false
                        Task {
                            _ = await locateMessage(message.messageId)
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
        .sheet(
            isPresented: Binding(
                get: { forwardCombined != nil },
                set: {
                    if !$0 {
                        forwardCombined = nil
                        forwardMessageIDs = nil
                    }
                }
            )
        ) {
            if let combined = forwardCombined {
                ForwardConversationSheet(
                    account: session.handle,
                    combined: combined
                ) { targetConversationID in
                    let succeeded = await model.forward(
                        messageIDs: forwardMessageIDs ?? [],
                        targetConversationID: targetConversationID,
                        combined: combined
                    )
                    if succeeded {
                        selectedMessageIDs.removeAll()
                        forwardCombined = nil
                        forwardMessageIDs = nil
                    }
                    return succeeded
                }
            }
        }
        .sheet(
            isPresented: Binding(
                get: { stickerDetailID != nil },
                set: { if !$0 { stickerDetailID = nil } }
            )
        ) {
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
                HStack(spacing: 10) {
                    ConversationAvatar(
                        conversation: navigation.selectedConversation,
                        size: 36
                    )

                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 4) {
                            Text(conversationName ?? "Conversation")
                                .font(.system(size: 16))
                                .foregroundStyle(theme.text)
                                .lineLimit(1)
                            if let conversation = navigation.selectedConversation {
                                ConversationIdentityBadge(conversation: conversation)
                            }
                        }

                        if !chatHeaderSubtitle.isEmpty {
                            Text(chatHeaderSubtitle)
                                .font(.system(size: 14))
                                .foregroundStyle(theme.secondaryText)
                                .lineLimit(1)
                        }
                    }
                }
            }
            .buttonStyle(.plain)

            Spacer(minLength: 8)

            if conversationIsBot, let appID = conversationOwnerID {
                AppIconButton(
                    assetName: "ChatHeaderBot",
                    iconSize: 20,
                    help: "Open bot app"
                ) {
                    Task {
                        await model.openBotHome(
                            appID: appID,
                            title: conversationName ?? "",
                            currency: session.profile.fiatCurrency
                        )
                    }
                }
            }

            if selectedMessageIDs.isEmpty {
                AppIconButton(
                    assetName: "ChatHeaderSearch",
                    iconSize: 20,
                    help: "Search messages",
                    action: showSearch
                )

                if !navigation.infoPresented {
                    AppIconButton(
                        assetName: "ChatHeaderInfo",
                        iconSize: 20,
                        help: "Conversation info",
                        action: navigation.toggleConversationInfo
                    )
                    .transition(.scale.combined(with: .opacity))
                }
            } else {
                Button("Cancel") {
                    selectedMessageIDs.removeAll()
                }
                .buttonStyle(.borderless)
                .font(.system(size: 14))
                .foregroundStyle(theme.accent)
                .padding(8)
                .frame(minWidth: 40, minHeight: 40)
            }
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
        return ""
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
                        Text(searchSender.map { "From: \($0.fullName)" } ?? "From")
                    }
                    .buttonStyle(
                        MessageSearchFilterStyle(selected: searchSender != nil)
                    )

                    ForEach(MessageSearchCategory.filterCases) { category in
                        Button(category.title) {
                            searchCategory = searchCategory == category ? .all : category
                        }
                        .buttonStyle(
                            MessageSearchFilterStyle(selected: searchCategory == category)
                        )
                    }
                    Spacer()
                }
                .padding(.leading, 16)
                .padding(.vertical, 8)
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
                AppScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(model.searchResults, id: \.messageId) { message in
                            Button {
                                Task {
                                    if await locateMessage(message.messageId) {
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
        _ = await locateMessage(request.messageID)
        navigation.consumeMessageJump(revision: request.revision)
    }

    private func locateMessage(_ messageID: String) async -> Bool {
        await model.locate(
            messageID: messageID,
            reuseLoadedWindow: scrollCoordinator.canAnimateJump(to: messageID)
        )
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
        let mentionPanelHeight = min(
            CGFloat(mentionModel.candidates.count) * 50,
            200
        )

        return VStack(alignment: .leading, spacing: 0) {
            if let replyMessage {
                ComposerQuotePreview(message: replyMessage) {
                    self.replyMessage = nil
                }
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
                    .frame(width: 40, height: 40, alignment: .center)
                    .disabled(model.sending)
                    .help("Share contact or attach files")

                    Spacer().frame(width: 6)

                    Button {
                        presentStickerPanel()
                    } label: {
                        ComposerIcon(assetName: "ComposerSticker")
                    }
                    .buttonStyle(.plain)
                    .disabled(model.sending)
                    .help("Emoji, stickers and GIFs")
                    .onHover {
                        setStickerButtonHovering($0)
                    }
                    .popover(isPresented: $stickerPresented, arrowEdge: .bottom) {
                        Group {
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
                                        dismissStickerPanel()
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
                        .onHover {
                            setStickerPanelHovering($0)
                        }
                    }

                    Spacer().frame(width: 16)

                    MentionComposer(
                        model: mentionModel,
                        account: session.handle,
                        conversationID: conversationID,
                        conversationCategory: conversationCategory,
                        conversationOwnerID: conversationOwnerID,
                        currentUserID: session.profile.userId,
                        encrypted: !conversationIsBot,
                        text: $draft,
                        disabled: model.sending,
                        focusRevision: composerFocusRevision,
                        selectionRequest: mentionSelectionRequest,
                        onSelectCandidate: selectMentionCandidate,
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
                                ComposerIcon(
                                    assetName: "ComposerSend",
                                    color: theme.accent
                                )
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
        .frame(maxWidth: .infinity)
        .background(alignment: .top) {
            if !mentionModel.candidates.isEmpty {
                MentionCandidatePanel(
                    users: mentionModel.candidates,
                    keyword: mentionModel.keyword,
                    selectedIndex: mentionModel.selectedIndex,
                    onSelect: selectMentionCandidate
                )
                .frame(maxWidth: .infinity)
                .frame(height: mentionPanelHeight)
                .offset(y: -mentionPanelHeight)
                .transition(.offset(y: mentionPanelHeight))
            }
        }
        .animation(
            reduceMotion ? nil : .easeOut(duration: 0.15),
            value: mentionModel.candidates.isEmpty
        )
        .animation(
            reduceMotion ? nil : .easeOut(duration: 0.16),
            value: replyMessage?.messageId
        )
    }

    private var discardRecordingDialog: some View {
        ZStack {
            Color.black.opacity(0.5)
                .ignoresSafeArea()
                .onTapGesture {
                    discardRecordingPresented = false
                }

            VStack(spacing: 0) {
                Text(
                    "Are you sure you want to stop recording and discard your voice message?"
                )
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(theme.text)
                .lineSpacing(16)
                .frame(maxWidth: .infinity, alignment: .leading)

                Spacer().frame(height: 36)

                HStack(spacing: 8) {
                    Spacer()
                    Button("Cancel") {
                        discardRecordingPresented = false
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(theme.accent)
                    .padding(.horizontal, 16)
                    .frame(height: 36)

                    Button("Discard") {
                        discardRecordingPresented = false
                        Task {
                            await voiceRecorder.cancel()
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(theme.accent)
                    .frame(height: 36)
                }
            }
            .padding(.horizontal, 30)
            .padding(.top, 40)
            .padding(.bottom, 30)
            .frame(width: 400)
            .background(theme.popUp, in: RoundedRectangle(cornerRadius: 11))
        }
        .transition(.opacity)
    }

    private func selectMentionCandidate(_ index: Int?) {
        guard
            let insertion = mentionModel.selectCandidate(
                at: index,
                in: draft
            )
        else {
            return
        }
        draft = insertion.text
        mentionSelectionRequest = ComposerSelectionRequest(
            offset: insertion.selectionUTF16,
            revision: (mentionSelectionRequest?.revision ?? 0) + 1
        )
    }

    private func presentStickerPanel() {
        stickerOpenTask?.cancel()
        stickerCloseTask?.cancel()
        withAnimation(stickerPanelAnimation) {
            stickerPresented = true
        }
    }

    private func dismissStickerPanel() {
        stickerOpenTask?.cancel()
        stickerCloseTask?.cancel()
        stickerButtonHovering = false
        stickerPanelHovering = false
        withAnimation(stickerPanelAnimation) {
            stickerPresented = false
        }
    }

    private func setStickerButtonHovering(_ hovering: Bool) {
        stickerButtonHovering = hovering
        updateStickerPanelPresentation()
    }

    private func setStickerPanelHovering(_ hovering: Bool) {
        stickerPanelHovering = hovering
        updateStickerPanelPresentation()
    }

    private func updateStickerPanelPresentation() {
        stickerOpenTask?.cancel()
        stickerCloseTask?.cancel()

        if stickerButtonHovering || stickerPanelHovering {
            guard !stickerPresented else {
                return
            }
            stickerOpenTask = Task {
                try? await Task.sleep(for: .milliseconds(50))
                guard !Task.isCancelled,
                    stickerButtonHovering || stickerPanelHovering
                else {
                    return
                }
                withAnimation(stickerPanelAnimation) {
                    stickerPresented = true
                }
            }
            return
        }

        guard stickerPresented else {
            return
        }
        stickerCloseTask = Task {
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled,
                !stickerButtonHovering,
                !stickerPanelHovering
            else {
                return
            }
            withAnimation(stickerPanelAnimation) {
                stickerPresented = false
            }
        }
    }

    private var stickerPanelAnimation: Animation? {
        reduceMotion ? nil : .easeOut(duration: 0.2)
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
        HStack(spacing: 0) {
            SelectionBarAction(
                title: "Combine and Forward",
                assetName: "MessageTranscriptForward",
                enabled: selectedMessages.count >= 2
                    && selectedMessages.count < 100
                    && selectedMessages.allSatisfy {
                        selectionPolicy(for: $0).canCombineForward
                    }
                    && !model.mutating
            ) {
                forwardMessageIDs = selectedMessages.map(\.messageId)
                forwardCombined = true
            }

            SelectionBarAction(
                title: "Forward One by One",
                assetName: "ContextMenuForward",
                enabled: !selectedMessages.isEmpty
                    && selectedMessages.count < 100
                    && selectedMessages.allSatisfy {
                        selectionPolicy(for: $0).canForward
                    }
                    && !model.mutating
            ) {
                forwardMessageIDs = selectedMessages.map(\.messageId)
                forwardCombined = false
            }

            SelectionBarAction(
                title: "Copy",
                assetName: "PreviewCopy",
                enabled: !selectedMessages.isEmpty
            ) {
                copySelectedMessages()
            }

            SelectionBarAction(
                title: "Delete",
                assetName: "ContextMenuDelete",
                enabled: !selectedMessages.isEmpty && !model.mutating
            ) {
                pendingDeletion = selectedMessages
            }
        }
        .frame(height: 80)
    }

    @ViewBuilder
    private var timelineContent: some View {
        switch model.state {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
            ContentUnavailableView(
                "Unable to load messages",
                systemImage: "exclamationmark.triangle",
                description: Text(message)
            )
        case .ready:
            @Bindable var coordinator = scrollCoordinator
            ZStack(alignment: .bottomTrailing) {
                    AppScrollView(scrollPosition: $scrollPosition) {
                        LazyVStack(spacing: 0) {
                            let unreadBoundaryID = unreadBoundaryMessageID
                            ForEach(model.rows) { row in
                                if let message = model.message(id: row.messageID) {
                                    VStack(spacing: 0) {
                                        if row.startsNewDay {
                                            dayChip(
                                                message.createdAtMicros
                                                    .dayChipTitle
                                            )
                                        }
                                        if message.messageId == unreadBoundaryID {
                                            Text("Unread Messages")
                                                .font(.system(size: 14))
                                                .foregroundStyle(theme.secondaryText)
                                                .frame(maxWidth: .infinity)
                                                .padding(.vertical, 4)
                                                .background(theme.background)
                                                .padding(.vertical, 6)
                                        }
                                        MessageRow(
                                            message: message,
                                            mentionNames: model.mentionNames,
                                            mentionNamesRevision:
                                                model.mentionNamesRevision,
                                            audioPlaylist: message.presentationKind == .audio
                                                ? model.audioMessages
                                                : [],
                                            mediaIndexRevision:
                                                message.presentationKind == .audio
                                                || message.isRichContent
                                                ? model.mediaIndexRevision
                                                : 0,
                                            mediaDirectory: model.mediaDirectory,
                                            conversationName: conversationName,
                                            groupPresentation:
                                                conversationCategory == "GROUP"
                                                || message.senderId
                                                    != (
                                                        message.conversationOwnerId
                                                        ?? conversationOwnerID
                                                        ?? ""
                                                    )
                                                || conversationIsBotGroup,
                                            outgoing: message.senderId == session.profile.userId,
                                            sameUserPrevious: row.sameUserPrevious,
                                            sameUserNext: row.sameUserNext,
                                            policy: policy(for: message),
                                            recalledText: model.recalledText(
                                                messageID: message.messageId
                                            ),
                                            selected: selectedMessageIDs.contains(
                                                message.messageId),
                                            selectionActive: !selectedMessageIDs.isEmpty,
                                            imageMessages: message.isRichContent
                                                ? model.imageMessages
                                                : [],
                                            loadImageWindow: model.imageMessagesAround,
                                            onReply: {
                                                replyMessage = message
                                            },
                                            onCopy: copyText,
                                            onForward: {
                                                forwardMessageIDs = [
                                                    message.messageId
                                                ]
                                                forwardCombined = false
                                            },
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
                                    }
                                    .background {
                                        if coordinator.highlightedMessageID
                                            == message.messageId
                                        {
                                            RoundedRectangle(cornerRadius: 8)
                                                .fill(Color.accentColor.opacity(0.16))
                                                .transition(.opacity)
                                        }
                                    }
                                    .id(message.messageId)
                                    .onGeometryChange(for: CGRect.self) { proxy in
                                        proxy.frame(
                                            in: .scrollView(axis: .vertical)
                                        )
                                    } action: { frame in
                                        coordinator.updateRowFrame(
                                            messageID: message.messageId,
                                            frame: frame,
                                            position: $scrollPosition
                                        )
                                    }
                                }
                            }
                        }
                        .scrollTargetLayout()
                        .padding(.bottom, 10)
                    }
                    .defaultScrollAnchor(.bottom)
                    .onScrollGeometryChange(
                        for: ChatScrollGeometry.self
                    ) { geometry in
                        ChatScrollGeometry(geometry)
                    } action: { _, geometry in
                        let direction = coordinator.updateGeometry(
                            geometry,
                            hasOlderMessages: model.hasOlderMessages,
                            hasNewerMessages: model.hasNewerMessages,
                            position: $scrollPosition
                        )
                        guard let direction else {
                            return
                        }
                        Task {
                            switch direction {
                            case .older:
                                await model.loadOlder()
                            case .newer:
                                await model.loadNewer()
                            }
                        }
                    }
                    .onScrollTargetVisibilityChange(
                        idType: String.self,
                        threshold: 0.01
                    ) { ids in
                        coordinator.updateVisibleMessageIDs(
                            ids,
                            orderedMessageIDs: model.rows.map(\.messageID),
                            dayStartMessageIDs: Set(
                                model.rows
                                    .filter(\.startsNewDay)
                                    .map(\.messageID)
                            )
                        )
                        model.updateVisibleMessageIDs(ids)
                    }
                    .onScrollPhaseChange { _, phase in
                        coordinator.updatePhase(phase)
                    }
                    .onChange(
                        of: model.timelineChange,
                        initial: true
                    ) { _, change in
                        coordinator.handle(
                            change,
                            hasNewerMessages: model.hasNewerMessages,
                            reduceMotion: reduceMotion,
                            position: $scrollPosition
                        )
                    }

                    if model.loadingOlder {
                        ProgressView()
                            .controlSize(.small)
                            .padding(8)
                            .background(.regularMaterial, in: Capsule())
                            .frame(
                                maxWidth: .infinity,
                                maxHeight: .infinity,
                                alignment: .top
                            )
                            .padding(.top, 8)
                            .allowsHitTesting(false)
                    } else if model.loadingNewer {
                        ProgressView()
                            .controlSize(.small)
                            .padding(8)
                            .background(.regularMaterial, in: Capsule())
                            .padding(.trailing, 16)
                            .padding(.bottom, 64)
                            .allowsHitTesting(false)
                    }

                    stickyDayOverlay
                    timelineOverlays
                    scamWarningOverlay
                    pinnedMessagesOverlay
                }
        }
    }

    private func dayChip(_ title: String) -> some View {
        Text(title)
            .font(.system(size: 14))
            .foregroundStyle(.black)
            .frame(minWidth: 64)
            .padding(.horizontal, 10)
            .padding(.vertical, 2)
            .background(
                theme.dateTime,
                in: RoundedRectangle(cornerRadius: 10)
            )
            .padding(.top, 16)
            .padding(.bottom, 10)
            .frame(maxWidth: .infinity)
    }

    @ViewBuilder
    private var stickyDayOverlay: some View {
        if let anchor = scrollCoordinator.stickyDayAnchor,
           let message = model.message(id: anchor.messageID)
        {
            dayChip(message.createdAtMicros.dayChipTitle)
                .offset(y: anchor.offset)
                .frame(
                    maxWidth: .infinity,
                    maxHeight: .infinity,
                    alignment: .top
                )
                .allowsHitTesting(false)
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
    private var scamWarningOverlay: some View {
        if scamWarningVisible {
            HStack(spacing: 0) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 26))
                    .foregroundStyle(theme.destructive)
                    .padding(.leading, 8)
                    .padding(.trailing, 16)
                    .padding(.vertical, 8)

                Text("Warning: this account has been reported as a scam.")
                    .font(.system(size: 14))
                    .foregroundStyle(theme.text)
                    .frame(maxWidth: .infinity, alignment: .leading)

                Button {
                    dismissScamWarning()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 20))
                        .frame(width: 40, height: 40)
                }
                .buttonStyle(.plain)
            }
            .padding(8)
            .background(
                colorScheme == .dark
                    ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
                    : Color.white,
                in: RoundedRectangle(cornerRadius: 4)
            )
            .shadow(color: .black.opacity(0.15), radius: 10, y: 2)
            .padding(.horizontal, 6)
            .padding(.bottom, 6)
            .frame(
                maxWidth: .infinity,
                maxHeight: .infinity,
                alignment: .bottom
            )
            .transition(.opacity)
        }
    }

    @ViewBuilder
    private var timelineOverlays: some View {
        VStack(alignment: .trailing, spacing: 8) {
            if !model.unreadMentionMessageIDs.isEmpty {
                Button {
                    Task {
                        guard let messageID = model.unreadMentionMessageIDs.first else {
                            return
                        }
                        if await locateMessage(messageID) {
                            await model.markMentionRead(messageID: messageID)
                        }
                    }
                } label: {
                    ZStack(alignment: .top) {
                        Text("@")
                            .font(.system(size: 17))
                            .foregroundStyle(theme.text)
                            .frame(width: 40, height: 40)
                            .background(timelineOverlaySurface, in: Circle())
                            .shadow(color: .black.opacity(0.15), radius: 10, y: 2)
                            .padding(.top, 12)

                        Text(
                            model.unreadMentionMessageIDs.count > 99
                                ? "99+"
                                : "\(model.unreadMentionMessageIDs.count)"
                        )
                        .font(.system(size: 14))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 4)
                        .frame(minWidth: 20, minHeight: 20)
                        .background(theme.accent, in: Capsule())
                    }
                    .frame(width: 40, height: 52)
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
            if scrollCoordinator.showJumpToLatest, !model.messages.isEmpty {
                Button {
                    Task {
                        if model.hasNewerMessages {
                            await model.jumpToLatest()
                        } else {
                            scrollCoordinator.scrollToLatest(
                                position: $scrollPosition,
                                animated: !reduceMotion
                            )
                        }
                    }
                } label: {
                    Image(systemName: "arrow.down")
                        .foregroundStyle(theme.text)
                        .frame(width: 40, height: 40)
                        .background(timelineOverlaySurface, in: Circle())
                        .shadow(color: .black.opacity(0.15), radius: 10, y: 2)
                }
                .buttonStyle(.plain)
                .help("Jump to latest")
            }
        }
        .padding(16)
    }

    private var timelineOverlaySurface: Color {
        colorScheme == .dark
            ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
            : .white
    }

    private var scamWarningPreferenceKey: String? {
        guard let conversationOwnerID else {
            return nil
        }
        return "scam_warning_\(session.profile.userId)_\(conversationOwnerID)"
    }

    private func loadScamWarningPreference() {
        guard conversationIsScam, let scamWarningPreferenceKey else {
            scamWarningVisible = false
            return
        }
        let dismissedUntil = UserDefaults.standard.double(
            forKey: scamWarningPreferenceKey
        )
        scamWarningVisible =
            dismissedUntil <= Date().timeIntervalSince1970 * 1_000
    }

    private func dismissScamWarning() {
        scamWarningVisible = false
        guard let scamWarningPreferenceKey else {
            return
        }
        UserDefaults.standard.set(
            Date().addingTimeInterval(24 * 60 * 60)
                .timeIntervalSince1970 * 1_000,
            forKey: scamWarningPreferenceKey
        )
    }

    @ViewBuilder
    private var pinnedMessagesOverlay: some View {
        if let pinned = model.pinnedMessages.first {
            HStack(spacing: 8) {
                if !model.pinnedPreviewDismissed {
                    PinMessagePreviewBubble(
                        text: "\(pinned.senderName) pinned \(pinned.displayText)",
                        onDismiss: model.dismissPinnedPreview,
                        onLocate: {
                            Task {
                                _ = await locateMessage(pinned.messageId)
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
                        .foregroundStyle(theme.text)
                        .frame(width: 34, height: 34)
                        .background(
                            timelineOverlaySurface,
                            in: Circle()
                        )
                        .shadow(color: .black.opacity(0.15), radius: 10, y: 2)
                }
                .buttonStyle(.plain)
                .help("Pinned Messages")
            }
            .frame(height: 64)
            .padding(.leading, 10)
            .padding(.trailing, 16)
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

    private var selectedMessages: [MessageItem] {
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

    private func policy(for message: MessageItem) -> MessageActionPolicy {
        MessageActionPolicy(
            message: message,
            currentUserID: session.profile.userId,
            currentUserRole: model.currentUserRole,
            now: Date()
        )
    }

    private func canRecall(_ message: MessageItem) -> Bool {
        selectionPolicy(for: message).canRecall
    }

    private func selectionPolicy(
        for message: MessageItem
    ) -> MessageActionPolicy {
        MessageActionPolicy(
            message: message,
            currentUserID: session.profile.userId,
            currentUserRole: nil,
            now: Date()
        )
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
            "\(message.senderName), (\(message.selectionFormattedTime)):\n\(message.selectionPreview)"
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
            let succeeded =
                recall
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

private struct ComposerQuotePreview: View {
    @Environment(\.mixinTheme) private var theme

    let message: MessageItem
    let onCancel: () -> Void

    var body: some View {
        HStack(spacing: 0) {
            HStack(spacing: 0) {
                Rectangle()
                    .fill(SenderNameColor.color(for: message.senderId) ?? theme.accent)
                    .frame(width: 6)

                VStack(alignment: .leading, spacing: 0) {
                    Text(message.senderName)
                        .font(.system(size: 14))
                        .foregroundStyle(SenderNameColor.color(for: message.senderId) ?? theme.accent)
                        .lineLimit(1)
                        .padding(.bottom, 4)

                    HStack(spacing: 4) {
                        if let icon = categoryIcon {
                            Image(systemName: icon)
                                .font(.system(size: 16))
                                .foregroundStyle(theme.secondaryText)
                                .frame(width: 16, height: 16)
                        }
                        Text(message.displayText)
                            .font(.system(size: 12))
                            .foregroundStyle(theme.secondaryText)
                            .lineLimit(1)
                    }
                }
                .padding(.leading, 6)
                .padding(.vertical, 6)

                Spacer(minLength: 8)

                previewImage
            }
            .frame(maxWidth: .infinity, minHeight: 50, alignment: .leading)

            Button(action: onCancel) {
                Image("ComposerClose")
                    .resizable()
                    .frame(width: 22, height: 22)
                    .padding(14)
            }
            .buttonStyle(.plain)
            .help("Cancel reply")
        }
        .background(theme.popUp)
    }

    @ViewBuilder
    private var previewImage: some View {
        if message.category.uppercased().hasSuffix("_CONTACT") {
            MixinRemoteImage(url: message.sharedUserAvatarUrl.flatMap(URL.init(string:))) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                AvatarPlaceholder(
                    userID: message.sharedUserId ?? "",
                    name: message.sharedUserFullName ?? ""
                )
            }
            .frame(width: 48, height: 48)
            .clipShape(RoundedRectangle(cornerRadius: 6))
        } else if let source = mediaSource {
            MessageMediaImage(
                source: source,
                thumbnail: message.thumbImage,
                contentMode: .fill
            )
            .frame(width: 48, height: 48)
            .clipped()
            .clipShape(RoundedRectangle(cornerRadius: 6))
        }
    }

    private var mediaSource: String? {
        let category = message.category.uppercased()
        if category.hasSuffix("_STICKER") {
            return message.stickerAssetUrl
        }
        return message.mediaUrl ?? message.thumbUrl
    }

    private var categoryIcon: String? {
        let category = message.category.uppercased()
        if category.contains("IMAGE") { return "photo" }
        if category.contains("VIDEO") || category.contains("CALL") { return "video" }
        if category.contains("LIVE") { return "dot.radiowaves.left.and.right" }
        if category.contains("AUDIO") { return "waveform" }
        if category.contains("STICKER") { return "face.smiling" }
        if category.contains("DATA") || category.contains("POST") || category.contains("TRANSCRIPT") {
            return "doc"
        }
        if category.contains("CONTACT") { return "person.crop.circle" }
        if category.contains("SNAPSHOT") || category.contains("TRANSFER") { return "arrow.left.arrow.right" }
        if category.contains("LOCATION") { return "location" }
        if category == "APP_CARD" || category == "APP_BUTTON_GROUP" { return "app" }
        if category.contains("RECALL") { return "arrow.uturn.backward" }
        return nil
    }
}

private enum SenderNameColor {
    private static let values: [UInt32] = [
        0x8C8DFF, 0x7983C2, 0x6D8DDE, 0x5979F0, 0x6695DF, 0x8F7AC5,
        0x9D77A5, 0x8A64D0, 0xAA66C3, 0xA75C96, 0xC8697D, 0xB74D62,
        0xBD637C, 0xB3798E, 0x9B6D77, 0xB87F7F, 0xC5595A, 0xAA4848,
        0xB0665E, 0xB76753, 0xBB5334, 0xC97B46, 0xBE6C2C, 0xCB7F40,
        0xA47758, 0xB69370, 0xA49373, 0xAA8A46, 0xAA8220, 0x76A048,
        0x9CAD23, 0xA19431, 0xAA9100, 0xA09555, 0xC49B4B, 0x5FB05F,
        0x6AB48F, 0x71B15C, 0xB3B357, 0xA3B561, 0x909F45, 0x93B289,
        0x3D98D0, 0x429AB6, 0x4EABAA, 0x6BC0CE, 0x64B5D9, 0x3E9CCB,
        0x2887C4, 0x52A98B,
    ]

    static func color(for userID: String) -> Color? {
        let parts = userID.trimmingCharacters(in: .whitespacesAndNewlines).split(separator: "-")
        guard parts.count == 5,
              let first = UInt64(parts[0], radix: 16),
              let second = UInt64(parts[1], radix: 16),
              let third = UInt64(parts[2], radix: 16),
              let fourth = UInt64(parts[3], radix: 16),
              let fifth = UInt64(parts[4], radix: 16)
        else {
            return nil
        }
        let mostSignificant = (first << 32) | (second << 16) | third
        let leastSignificant = (fourth << 48) | fifth
        let highLow = mostSignificant ^ leastSignificant
        let hash = Int32(bitPattern: UInt32(truncatingIfNeeded: (highLow >> 32) ^ highLow))
        let value = values[Int(hash.magnitude % UInt32(values.count))]
        return Color(
            red: Double((value >> 16) & 0xFF) / 255,
            green: Double((value >> 8) & 0xFF) / 255,
            blue: Double(value & 0xFF) / 255
        )
    }
}

private struct ComposerIcon: View {
    @Environment(\.mixinTheme) private var theme
    let assetName: String
    var size: CGFloat = 24
    var padding: CGFloat = 8
    var color: Color?

    var body: some View {
        Image(assetName)
            .resizable()
            .renderingMode(.template)
            .foregroundStyle(color ?? theme.icon)
            .frame(width: size, height: size)
            .padding(padding)
            .contentShape(Circle())
    }
}

private struct SelectionBarAction: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @State private var hovering = false

    let title: String
    let assetName: String
    let enabled: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(assetName)
                    .resizable()
                    .renderingMode(.template)
                    .frame(width: 24, height: 24)
                    .frame(width: 24, height: 24)
                Text(title)
                    .font(.system(size: 14))
                    .lineLimit(1)
            }
            .foregroundStyle(theme.text)
            .frame(maxWidth: .infinity)
            .padding(8)
            .background(hoverBackground)
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : 0.5)
        .onHover { hovering = $0 }
    }

    private var hoverBackground: Color {
        guard enabled && hovering else {
            return .clear
        }
        return colorScheme == .dark
            ? Color.white.opacity(0.2)
            : Color.black.opacity(0.03)
    }
}

private struct PinMessagePreviewBubble: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    let text: String
    let onDismiss: () -> Void
    let onLocate: () -> Void

    var body: some View {
        HStack(spacing: 4) {
            Button(action: onDismiss) {
                Image(systemName: "xmark")
                    .font(.system(size: 20))
                    .frame(width: 40, height: 40)
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
                .fill(
                    colorScheme == .dark
                        ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
                        : Color.white
                )
                .shadow(color: .black.opacity(0.15), radius: 10, y: 2)
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

struct MessageBubbleShape: Shape {
    let outgoing: Bool
    let showsNip: Bool

    func path(in rect: CGRect) -> Path {
        var incoming = Path()
        incoming.addRoundedRect(
            in: CGRect(
                x: 9,
                y: 0,
                width: max(rect.width - 9, 0),
                height: rect.height
            ),
            cornerSize: CGSize(width: 8, height: 8)
        )
        if showsNip {
            incoming.addPath(leftNip(in: rect))
        }
        guard outgoing else {
            return incoming
        }
        return incoming.applying(
            CGAffineTransform(
                a: -1,
                b: 0,
                c: 0,
                d: 1,
                tx: rect.width,
                ty: 0
            )
        )
    }

    private func leftNip(in rect: CGRect) -> Path {
        let width = 9.0
        let height = 12.0
        let y = max(rect.height - 21, 0)
        var path = Path()
        path.move(to: CGPoint(x: 0, y: y))
        path.addLine(to: CGPoint(x: width * 1.04, y: y + height))
        path.addCurve(
            to: CGPoint(x: width, y: y + height * 0.19),
            control1: CGPoint(x: width * 1.04, y: y + height),
            control2: CGPoint(x: width * 1.04, y: y)
        )
        path.addCurve(
            to: CGPoint(x: width * 0.14, y: y + height * 0.67),
            control1: CGPoint(x: width * 0.81, y: y + height * 0.41),
            control2: CGPoint(x: width / 2, y: y + height * 0.59)
        )
        path.addCurve(
            to: CGPoint(x: width * 0.13, y: y + height * 0.85),
            control1: CGPoint(x: width * 0.03, y: y + height * 0.69),
            control2: CGPoint(x: width * 0.01, y: y + height * 0.79)
        )
        path.addCurve(
            to: CGPoint(x: width * 0.91, y: y + height),
            control1: CGPoint(x: width * 0.36, y: y + height * 0.94),
            control2: CGPoint(x: width * 0.62, y: y + height)
        )
        path.closeSubpath()
        return path
    }
}

private struct SearchMessageRow: View {
    @Environment(\.mixinTheme) private var theme
    let message: MessageItem
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
            .frame(width: 50, height: 50)
            .clipShape(Circle())

            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 4) {
                    Text(message.senderName)
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                        .lineLimit(1)
                    Spacer()
                    Text(message.formattedTime)
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                }
                HStack(spacing: 2) {
                    if let icon = categoryIcon {
                        Image(systemName: icon)
                            .font(.system(size: 14))
                            .foregroundStyle(theme.secondaryText)
                    }
                    MessageRichText(
                        content: message.displayText,
                        baseFontSize: 14,
                        color: theme.secondaryText,
                        lineLimit: 1,
                        mentionNames: mentionNames,
                        highlight: highlight
                    )
                }
            }
        }
        .frame(minHeight: 72)
        .padding(.horizontal, 6)
        .padding(.vertical, 12)
    }

    private var categoryIcon: String? {
        let category = message.category.uppercased()
        if category.contains("IMAGE") { return "photo" }
        if category.contains("VIDEO") || category.contains("CALL") { return "video" }
        if category.contains("LIVE") { return "dot.radiowaves.left.and.right" }
        if category.contains("AUDIO") { return "waveform" }
        if category.contains("STICKER") { return "face.smiling" }
        if category.contains("DATA") || category.contains("POST") || category.contains("TRANSCRIPT") {
            return "doc"
        }
        if category.contains("CONTACT") { return "person.crop.circle" }
        if category.contains("SNAPSHOT") || category.contains("TRANSFER") { return "arrow.left.arrow.right" }
        if category.contains("LOCATION") { return "location" }
        if category == "APP_CARD" || category == "APP_BUTTON_GROUP" { return "app" }
        if category.contains("RECALL") { return "arrow.uturn.backward" }
        return nil
    }
}

private struct MessageSearchFilterStyle: ButtonStyle {
    @Environment(\.mixinTheme) private var theme

    let selected: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 16))
            .foregroundStyle(selected ? Color.white : theme.text)
            .frame(height: 32)
            .padding(.horizontal, 12)
            .background(selected ? theme.accent : theme.listSelected, in: Capsule())
            .opacity(configuration.isPressed ? 0.8 : 1)
    }
}

private struct MessageContentMetadataLayout: Layout {
    struct Cache {
        var measurement: Measurement?
    }

    struct Measurement {
        let proposedWidth: CGFloat?
        let size: CGSize
        let contentSize: CGSize
        let metadataSize: CGSize
        let contentOrigin: CGPoint
        let metadataOrigin: CGPoint
    }

    let inlineSpacing: CGFloat
    let stackedSpacing: CGFloat

    init(inlineSpacing: CGFloat = 6, stackedSpacing: CGFloat = 2) {
        self.inlineSpacing = inlineSpacing
        self.stackedSpacing = stackedSpacing
    }

    func makeCache(subviews _: Subviews) -> Cache {
        Cache()
    }

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout Cache
    ) -> CGSize {
        let measurement = measure(proposal: proposal, subviews: subviews)
        cache.measurement = measurement
        return measurement.size
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout Cache
    ) {
        guard subviews.count == 2 else {
            return
        }
        let width = proposal.width ?? bounds.width
        let measurement: Measurement
        if let cached = cache.measurement,
            cached.proposedWidth == width,
            abs(cached.size.width - bounds.width) < 0.5,
            abs(cached.size.height - bounds.height) < 0.5
        {
            measurement = cached
        } else {
            measurement = measure(
                proposal: ProposedViewSize(
                    width: width,
                    height: proposal.height ?? bounds.height
                ),
                subviews: subviews
            )
            cache.measurement = measurement
        }

        subviews[0].place(
            at: CGPoint(
                x: bounds.minX + measurement.contentOrigin.x,
                y: bounds.minY + measurement.contentOrigin.y
            ),
            anchor: .topLeading,
            proposal: ProposedViewSize(measurement.contentSize)
        )
        subviews[1].place(
            at: CGPoint(
                x: bounds.minX + measurement.metadataOrigin.x,
                y: bounds.minY + measurement.metadataOrigin.y
            ),
            anchor: .topLeading,
            proposal: ProposedViewSize(measurement.metadataSize)
        )
    }

    private func measure(
        proposal: ProposedViewSize,
        subviews: Subviews
    ) -> Measurement {
        guard subviews.count == 2 else {
            return Measurement(
                proposedWidth: proposal.width,
                size: .zero,
                contentSize: .zero,
                metadataSize: .zero,
                contentOrigin: .zero,
                metadataOrigin: .zero
            )
        }

        let content = subviews[0]
        let metadata = subviews[1]
        let metadataDimensions = metadata.dimensions(in: .unspecified)
        let metadataSize = CGSize(
            width: metadataDimensions.width,
            height: metadataDimensions.height
        )
        let idealContentDimensions = content.dimensions(in: .unspecified)
        let idealContentSize = CGSize(
            width: idealContentDimensions.width,
            height: idealContentDimensions.height
        )
        let availableWidth = proposal.width ?? .infinity

        if idealContentSize.width + inlineSpacing + metadataSize.width
            <= availableWidth
        {
            return inlineMeasurement(
                proposedWidth: proposal.width,
                contentDimensions: idealContentDimensions,
                contentSize: idealContentSize,
                metadataDimensions: metadataDimensions,
                metadataSize: metadataSize
            )
        }

        let contentWidth = max(availableWidth, metadataSize.width)
        let contentDimensions = content.dimensions(
            in: ProposedViewSize(
                width: contentWidth.isFinite ? contentWidth : nil,
                height: proposal.height
            )
        )
        let contentSize = CGSize(
            width: contentDimensions.width,
            height: contentDimensions.height
        )
        return Measurement(
            proposedWidth: proposal.width,
            size: CGSize(
                width: max(contentSize.width, metadataSize.width),
                height: contentSize.height + stackedSpacing + metadataSize.height
            ),
            contentSize: contentSize,
            metadataSize: metadataSize,
            contentOrigin: .zero,
            metadataOrigin: CGPoint(
                x: max(contentSize.width - metadataSize.width, 0),
                y: contentSize.height + stackedSpacing
            )
        )
    }

    private func inlineMeasurement(
        proposedWidth: CGFloat?,
        contentDimensions: ViewDimensions,
        contentSize: CGSize,
        metadataDimensions: ViewDimensions,
        metadataSize: CGSize
    ) -> Measurement {
        let contentBaseline = contentDimensions[.lastTextBaseline]
        let metadataBaseline = metadataDimensions[.lastTextBaseline]
        let baseline = max(contentBaseline, metadataBaseline)
        let contentY = baseline - contentBaseline
        let metadataY = baseline - metadataBaseline
        return Measurement(
            proposedWidth: proposedWidth,
            size: CGSize(
                width: contentSize.width + inlineSpacing + metadataSize.width,
                height: max(
                    contentY + contentSize.height,
                    metadataY + metadataSize.height
                )
            ),
            contentSize: contentSize,
            metadataSize: metadataSize,
            contentOrigin: CGPoint(x: 0, y: contentY),
            metadataOrigin: CGPoint(
                x: contentSize.width + inlineSpacing,
                y: metadataY
            )
        )
    }
}

struct MessageRow: View, Equatable {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(SettingsPreferencesModel.self) private var preferences
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @State private var qrPresentation: MessageQRPresentation?
    @State private var profilePresented = false
    @State private var addImageStickerPresented = false
    @State private var stickerActionError: String?
    let message: MessageItem
    let mentionNames: [String: String]
    let mentionNamesRevision: Int
    let audioPlaylist: [MessageItem]
    let mediaIndexRevision: Int
    let mediaDirectory: URL?
    let conversationName: String?
    let groupPresentation: Bool
    let outgoing: Bool
    let sameUserPrevious: Bool
    let sameUserNext: Bool
    let policy: MessageActionPolicy
    let recalledText: String?
    let selected: Bool
    let selectionActive: Bool
    let imageMessages: [MessageItem]
    let loadImageWindow: (String) async throws -> [ImageMessageView]
    let onReply: () -> Void
    let onCopy: (String) -> Void
    let onForward: () -> Void
    let onSelect: () -> Void
    let onToggleSelection: () -> Void
    let onTogglePin: () -> Void
    let onRecall: () -> Void
    let onDelete: () -> Void
    let onReedit: (String) -> Void
    let attachmentProgress: Double
    let onAttachmentAction: () -> Void
    let loadTranscript: () async throws -> [MessageItem]
    let onTranscriptAttachmentAction: (MessageItem) async -> Void
    let onMarkAudioRead: (String) -> Void
    let onShowStickerDetail: (String) -> Void
    let onAppAction: (String, String) -> Void
    let onStrangerAction: (String) async -> Bool
    let isPinnedPage: Bool

    init(
        message: MessageItem,
        mentionNames: [String: String],
        mentionNamesRevision: Int,
        audioPlaylist: [MessageItem],
        mediaIndexRevision: Int,
        mediaDirectory: URL?,
        conversationName: String?,
        groupPresentation: Bool,
        outgoing: Bool,
        sameUserPrevious: Bool,
        sameUserNext: Bool,
        policy: MessageActionPolicy,
        recalledText: String?,
        selected: Bool,
        selectionActive: Bool,
        imageMessages: [MessageItem],
        loadImageWindow: @escaping (String) async throws -> [ImageMessageView],
        onReply: @escaping () -> Void,
        onCopy: @escaping (String) -> Void,
        onForward: @escaping () -> Void = {},
        onSelect: @escaping () -> Void,
        onToggleSelection: @escaping () -> Void,
        onTogglePin: @escaping () -> Void,
        onRecall: @escaping () -> Void,
        onDelete: @escaping () -> Void,
        onReedit: @escaping (String) -> Void,
        attachmentProgress: Double,
        onAttachmentAction: @escaping () -> Void,
        loadTranscript: @escaping () async throws -> [MessageItem],
        onTranscriptAttachmentAction: @escaping (MessageItem) async -> Void,
        onMarkAudioRead: @escaping (String) -> Void,
        onShowStickerDetail: @escaping (String) -> Void,
        onAppAction: @escaping (String, String) -> Void,
        onStrangerAction: @escaping (String) async -> Bool,
        isPinnedPage: Bool = false
    ) {
        self.message = message
        self.mentionNames = mentionNames
        self.mentionNamesRevision = mentionNamesRevision
        self.audioPlaylist = audioPlaylist
        self.mediaIndexRevision = mediaIndexRevision
        self.mediaDirectory = mediaDirectory
        self.conversationName = conversationName
        self.groupPresentation = groupPresentation
        self.outgoing = outgoing
        self.sameUserPrevious = sameUserPrevious
        self.sameUserNext = sameUserNext
        self.policy = policy
        self.recalledText = recalledText
        self.selected = selected
        self.selectionActive = selectionActive
        self.imageMessages = imageMessages
        self.loadImageWindow = loadImageWindow
        self.onReply = onReply
        self.onCopy = onCopy
        self.onForward = onForward
        self.onSelect = onSelect
        self.onToggleSelection = onToggleSelection
        self.onTogglePin = onTogglePin
        self.onRecall = onRecall
        self.onDelete = onDelete
        self.onReedit = onReedit
        self.attachmentProgress = attachmentProgress
        self.onAttachmentAction = onAttachmentAction
        self.loadTranscript = loadTranscript
        self.onTranscriptAttachmentAction = onTranscriptAttachmentAction
        self.onMarkAudioRead = onMarkAudioRead
        self.onShowStickerDetail = onShowStickerDetail
        self.onAppAction = onAppAction
        self.onStrangerAction = onStrangerAction
        self.isPinnedPage = isPinnedPage
    }

    static func == (lhs: MessageRow, rhs: MessageRow) -> Bool {
        lhs.message == rhs.message
            && lhs.mentionNamesRevision == rhs.mentionNamesRevision
            && lhs.mediaIndexRevision == rhs.mediaIndexRevision
            && lhs.mediaDirectory == rhs.mediaDirectory
            && lhs.conversationName == rhs.conversationName
            && lhs.groupPresentation == rhs.groupPresentation
            && lhs.outgoing == rhs.outgoing
            && lhs.sameUserPrevious == rhs.sameUserPrevious
            && lhs.sameUserNext == rhs.sameUserNext
            && lhs.policy == rhs.policy
            && lhs.recalledText == rhs.recalledText
            && lhs.selected == rhs.selected
            && lhs.selectionActive == rhs.selectionActive
            && lhs.attachmentProgress == rhs.attachmentProgress
    }

    var body: some View {
        messageLayout
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
            .sheet(isPresented: $profilePresented) {
                NavigationStack {
                    MessageUserProfileView(userID: message.senderId)
                }
                .frame(minWidth: 420, minHeight: 560)
            }
            .sheet(isPresented: $addImageStickerPresented) {
                if let source = message.localMediaURL {
                    AddImageStickerView(
                        account: session.handle,
                        messageID: message.messageId,
                        source: source
                    )
                }
            }
            .alert(
                "Add Sticker Failed",
                isPresented: Binding(
                    get: { stickerActionError != nil },
                    set: { if !$0 { stickerActionError = nil } }
                )
            ) {
                Button("OK") {
                    stickerActionError = nil
                }
            } message: {
                Text(stickerActionError ?? "")
            }
    }

    @ViewBuilder
    private var messageLayout: some View {
        if message.isCenteredTimelineMessage {
            HStack {
                Spacer(minLength: 8)
                messageBody
                    .font(.system(size: 14 + preferences.chatFontSizeDelta))
                Spacer(minLength: 8)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .padding(.top, sameUserPrevious ? 0 : 8)
            .frame(maxWidth: .infinity)
        } else {
            standardMessageRow
        }
    }

    private var standardMessageRow: some View {
        HStack(alignment: .top, spacing: 0) {
            MessageSelectionIndicator(
                selected: selected,
                visible: selectionActive
            )

            if outgoing {
                Spacer(minLength: 65)
            } else if showsAvatar, !sameUserPrevious {
                avatar
            } else if showsAvatar {
                Spacer()
                    .frame(width: 32)
            } else {
                Spacer()
                    .frame(width: 8)
            }

            VStack(alignment: outgoing ? .trailing : .leading, spacing: 0) {
                if groupPresentation, !outgoing, !sameUserPrevious {
                    Button {
                        profilePresented = true
                    } label: {
                        HStack(alignment: .bottom, spacing: 0) {
                            Text(message.senderName)
                                .foregroundStyle(
                                    MessageNamePalette.color(
                                        for: message.senderId
                                    )
                                )
                            if preferences.showIdentityNumber,
                               let identity = message.senderIdentityNumber,
                               !identity.isEmpty,
                               identity != "0"
                            {
                                Spacer().frame(width: 2)
                                Text("@\(identity)")
                                    .font(.system(size: 10))
                                    .foregroundStyle(theme.text.opacity(0.5))
                                    .padding(.bottom, 2)
                            }
                            ProfileIdentityBadge(
                                isVerified: message.senderIsVerified,
                                isBot: message.senderIsBot,
                                membership: message.senderMembership
                            )
                            .padding(.bottom, 3)
                        }
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 12))
                    .padding(.leading, 10)
                    .padding(.bottom, 2)
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
        .padding(.leading, outgoing ? 0 : 8)
        .padding(.trailing, outgoing ? 16 : 0)
        .padding(.top, sameUserPrevious ? 0 : 8)
        .padding(.vertical, 2)
    }

    private var messageBubble: some View {
        let content = Group {
            if message.hidesMetadata {
                messageContentWithQuote
            } else if message.overlaysMetadata {
                messageContentWithQuote
                    .overlay(alignment: .bottomTrailing) {
                        messageMetadata
                            .foregroundStyle(.white)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 3)
                            .background(.black.opacity(0.3), in: Capsule())
                            .padding(6)
                    }
            } else if message.usesOuterMetadata {
                messageContentWithQuote
            } else {
                MessageContentMetadataLayout {
                    messageContentWithQuote
                    messageMetadata
                }
            }
        }
        let bubble = messageBubbleSurface(content)
        return HStack(spacing: 0) {
            if message.isDisappearing, outgoing {
                expiringIcon
            }
            bubble
            if message.isDisappearing, !outgoing {
                expiringIcon
            }
        }
        .font(.system(size: 16 + preferences.chatFontSizeDelta))
    }

    private func messageBubbleSurface<Content: View>(
        _ content: Content
    ) -> some View {
        let shape = MessageBubbleShape(
            outgoing: outgoing,
            showsNip: showsBubbleNip
        )
        let nipInsets =
            message.includesBubbleNipInContent
            ? EdgeInsets()
            : EdgeInsets(
                top: 0,
                leading: outgoing ? 0 : 9,
                bottom: 0,
                trailing: outgoing ? 9 : 0
            )
        return content
            .padding(nipInsets)
            .background {
                if message.showsBubbleSurface {
                    shape
                        .fill(messageBubbleColor)
                        .shadow(
                            color: .black.opacity(0.22),
                            radius: 0.6,
                            y: 0.3
                        )
                }
            }
            .clipShape(
                message.clipsBubbleContent
                    ? AnyShape(shape)
                    : AnyShape(Rectangle())
            )
    }

    private var showsBubbleNip: Bool {
        !sameUserNext && (!showsAvatar || outgoing)
    }

    private var expiringIcon: some View {
        Image(colorScheme == .dark ? "ExpiringDark" : "Expiring")
            .resizable()
            .frame(width: 16, height: 16)
            .padding(.horizontal, 10)
    }

    private var messageMetadata: some View {
        HStack(spacing: 0) {
            if message.pinned {
                Image(systemName: "pin.fill")
                    .font(.system(size: 8))
                    .frame(width: 8, height: 8)
                    .padding(.trailing, 4)
            }
            Text(message.formattedTime)
            if outgoing {
                Spacer()
                    .frame(width: 8)
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
        .frame(height: 12)
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
        if isPinnedPage {
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
            if message.category.hasSuffix("_IMAGE"),
               message.mediaStatus.isComplete,
               message.localMediaURL != nil
            {
                Button("Copy Image", systemImage: "photo.on.rectangle") {
                    MessageMediaInteraction.copyImage(message)
                }
            }
            Divider()
            Button("Locate in Chat", systemImage: "scope") {
                onReply()
            }
            Button("Unpin", systemImage: "pin.slash") {
                onTogglePin()
            }
            if policy.canSave, message.localMediaURL != nil {
                Divider()
                Button("Save Attachment As…", systemImage: "square.and.arrow.down") {
                    MessageMediaInteraction.save(message)
                }
            }
        } else if !selectionActive, policy.allowsMessageActions {
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
            if message.category.hasSuffix("_IMAGE"),
               message.mediaStatus.isComplete,
               message.localMediaURL != nil
            {
                Button("Copy Image", systemImage: "photo.on.rectangle") {
                    MessageMediaInteraction.copyImage(message)
                }
            }
            if policy.canForward || policy.canSelect || policy.canPin {
                Divider()
            }
            if policy.canForward {
                Button("Forward", systemImage: "arrowshape.turn.up.right") {
                    onForward()
                }
            }
            if policy.canSelect {
                Button("Select", systemImage: "checkmark.circle") {
                    onSelect()
                }
            }
            if policy.canPin {
                Button(
                    message.pinned ? "Unpin" : "Pin",
                    systemImage: message.pinned ? "pin.slash" : "pin"
                ) {
                    onTogglePin()
                }
            }
            if policy.canSave, message.localMediaURL != nil {
                Divider()
                Button("Save Attachment As…", systemImage: "square.and.arrow.down") {
                    MessageMediaInteraction.save(message)
                }
            }
            if validStickerID != nil || policy.canAddImageAsSticker {
                Divider()
                Button("Add Sticker", systemImage: "face.smiling") {
                    if let stickerID = validStickerID {
                        Task {
                            do {
                                try await session.handle.addSticker(
                                    stickerId: stickerID
                                )
                            } catch {
                                stickerActionError =
                                    MixinErrorPresenter.message(for: error)
                            }
                        }
                    } else {
                        addImageStickerPresented = true
                    }
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
        Button {
            profilePresented = true
        } label: {
            UserAvatar(
                userID: message.senderId,
                name: message.senderName,
                url: message.senderAvatarUrl,
                size: 32
            )
        }
        .buttonStyle(.plain)
    }

    private var showsAvatar: Bool {
        groupPresentation && preferences.showAvatar
    }

    @ViewBuilder
    private var messageBody: some View {
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
            onForward: onForward,
            loadTranscript: loadTranscript,
            onTranscriptAttachmentAction: onTranscriptAttachmentAction,
            onReedit: onReedit,
            onMarkAudioRead: onMarkAudioRead,
            onShowStickerDetail: onShowStickerDetail,
            onAppAction: onAppAction,
            onStrangerAction: onStrangerAction
        )
    }

    @ViewBuilder
    private var messageContentWithQuote: some View {
        if message.quoteMessageId?.isEmpty == false {
            VStack(alignment: .leading, spacing: 0) {
                MessageQuotePreview(
                    content: message.quoteContent ?? "",
                    mentionNames: mentionNames
                )
                messageBody
                    .padding(message.messageContentInsets)
            }
        } else {
            messageBody
                .padding(message.messageContentInsets)
        }
    }

    private var validStickerID: String? {
        guard policy.canAddSticker,
              let stickerID = message.stickerId?
                .trimmingCharacters(in: .whitespacesAndNewlines),
              !stickerID.isEmpty
        else {
            return nil
        }
        return stickerID
    }
}

private struct MessageQuotePreview: View {
    @Environment(\.mixinTheme) private var theme

    let content: String
    let mentionNames: [String: String]

    var body: some View {
        HStack(spacing: 0) {
            Rectangle()
                .fill(theme.accent)
                .frame(width: 6)

            Group {
                if content.isEmpty {
                    Text("Message not found")
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                } else {
                    MessageRichText(
                        content: content,
                        baseFontSize: 12,
                        color: .secondary,
                        lineLimit: 1,
                        mentionNames: mentionNames
                    )
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.leading, 6)
            .padding(.vertical, 6)
            .padding(.trailing, 8)
        }
        .frame(minHeight: 50)
        .background(.black.opacity(0.04))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

private struct AddImageStickerView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    let account: SwiftAccountHandle
    let messageID: String
    let source: URL
    @State private var saving = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            ZStack {
                Text("Add Sticker")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(theme.text)
                HStack {
                    Spacer()
                    Button {
                        dismiss()
                    } label: {
                        Image(systemName: "xmark")
                            .frame(width: 40, height: 40)
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 8)
            }
            .frame(height: 64)

            Spacer()
            ZStack {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(
                        theme.divider,
                        style: StrokeStyle(
                            lineWidth: 2,
                            dash: [8, 2]
                        )
                    )
                if let image = NSImage(contentsOf: source) {
                    Image(nsImage: image)
                        .resizable()
                        .scaledToFit()
                        .padding(30)
                }
            }
            .frame(width: 400, height: 400)
            Spacer()
            Button("Save") {
                save()
            }
            .buttonStyle(.plain)
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(.white)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
            .disabled(saving)
            .padding(.vertical, 30)
        }
        .frame(width: 480, height: 600)
        .background(theme.popUp)
        .alert(
            "Add Sticker Failed",
            isPresented: Binding(
                get: { error != nil },
                set: { if !$0 { error = nil } }
            )
        ) {
            Button("OK") {
                error = nil
            }
        } message: {
            Text(error ?? "")
        }
    }

    private func save() {
        guard !saving else {
            return
        }
        saving = true
        Task {
            do {
                try await account.addStickerFromFile(messageId: messageID)
                dismiss()
            } catch {
                self.error = MixinErrorPresenter.message(for: error)
                saving = false
            }
        }
    }
}

private struct MessageSelectionIndicator: View {
    let selected: Bool
    let visible: Bool

    var body: some View {
        Circle()
            .fill(selected ? Color.accentColor : Color.secondary)
            .frame(width: 16, height: 16)
            .overlay {
                Image(systemName: "checkmark")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundStyle(.white)
            }
            .frame(width: visible ? 48 : 0, height: 20)
            .opacity(visible ? 1 : 0)
            .clipped()
            .accessibilityHidden(!visible)
            .animation(.easeInOut(duration: 0.3), value: visible)
    }
}

private enum MessageNamePalette {
    private static let colors: [Color] = [
        color(0x8C8DFF), color(0x7983C2), color(0x6D8DDE),
        color(0x5979F0), color(0x6695DF), color(0x8F7AC5),
        color(0x9D77A5), color(0x8A64D0), color(0xAA66C3),
        color(0xA75C96), color(0xC8697D), color(0xB74D62),
        color(0xBD637C), color(0xB3798E), color(0x9B6D77),
        color(0xB87F7F), color(0xC5595A), color(0xAA4848),
        color(0xB0665E), color(0xB76753), color(0xBB5334),
        color(0xC97B46), color(0xBE6C2C), color(0xCB7F40),
        color(0xA47758), color(0xB69370), color(0xA49373),
        color(0xAA8A46), color(0xAA8220), color(0x76A048),
        color(0x9CAD23), color(0xA19431), color(0xAA9100),
        color(0xA09555), color(0xC49B4B), color(0x5FB05F),
        color(0x6AB48F), color(0x71B15C), color(0xB3B357),
        color(0xA3B561), color(0x909F45), color(0x93B289),
        color(0x3D98D0), color(0x429AB6), color(0x4EABAA),
        color(0x6BC0CE), color(0x64B5D9), color(0x3E9CCB),
        color(0x2887C4), color(0x52A98B),
    ]

    static func color(for userID: String) -> Color {
        colors[Int(hash(for: userID).magnitude % UInt64(colors.count))]
    }

    private static func hash(for userID: String) -> Int64 {
        let components = userID.trimmingCharacters(in: .whitespacesAndNewlines)
            .split(separator: "-")
        guard components.count == 5,
              let first = UInt64(components[0], radix: 16),
              let second = UInt64(components[1], radix: 16),
              let third = UInt64(components[2], radix: 16),
              let fourth = UInt64(components[3], radix: 16),
              let fifth = UInt64(components[4], radix: 16)
        else {
            return userID.utf8.reduce(0) { hash, byte in
                hash &* 31 &+ Int64(byte)
            }
        }
        let mostSignificantBits = first << 32 | second << 16 | third
        let leastSignificantBits = fourth << 48 | fifth
        let hilo = mostSignificantBits ^ leastSignificantBits
        return (Int64(bitPattern: hilo) >> 32)
            ^ Int64(Int32(truncatingIfNeeded: hilo))
    }

    private static func color(_ hex: UInt32) -> Color {
        Color(
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
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
    private var timelineStore = ChatTimelineStore()
    var messages: [MessageItem] { timelineStore.messages }
    fileprivate var rows: [ChatTimelineRow] { timelineStore.rows }
    fileprivate func message(id: String) -> MessageItem? {
        timelineStore.message(id: id)
    }
    var imageMessages: [MessageItem] {
        timelineStore.imageMessages
    }
    var audioMessages: [MessageItem] {
        timelineStore.audioMessages
    }
    var mediaIndexRevision: Int {
        timelineStore.mediaRevision
    }
    var mentionNamesRevision: Int {
        mentionPresentationRevision
    }
    private(set) var hasOlderMessages = false
    private(set) var hasNewerMessages = false
    private(set) var timelineChange: ChatTimelineChange?
    private(set) var sending = false
    private(set) var sendError: String?
    private(set) var searchResults: [MessageItem] = []
    private(set) var searching = false
    private(set) var searchError: String?
    private(set) var hasMoreSearchResults = false
    private(set) var mentionNames: [String: String] = [:]
    private(set) var currentUserRole: String?
    private(set) var pinnedMessages: [MessageItem] = []
    private(set) var unreadMentionMessageIDs: [String] = []
    private(set) var pinnedPreviewDismissed = false
    private(set) var mutating = false
    private(set) var mutationError: String?
    private(set) var mediaDirectory: URL?
    private var account: SwiftAccountHandle?
    private var conversationID: String?
    private var currentUserID: String?
    private var conversationSubscription: SwiftConversationSubscription?
    private var conversationSubscriptionTask: Task<Void, Never>?
    private var messageSubscription: SwiftMessageSubscription?
    private var messageSubscriptionTask: Task<Void, Never>?
    private var requestVersion = 0
    private var windowVersion = 0
    private var searchVersion = 0
    private var mentionRevision = 0
    private var mentionPresentationRevision = 0
    private var resolvedMentionContents = Set<String>()
    private var timelineRevision = 0
    private var visibleMessageIDs = Set<String>()
    private var messageRevision: UInt64 = 0
    private var refreshedMessageRevisions: [String: UInt64] = [:]
    private var visibleRefreshTask: Task<Void, Never>?
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
    private static let initialPageSize = 60
    private static let pageSize = 100
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
        currentUserID: String,
        lastReadMessageID: String?,
        unseenCount: Int64,
        initialAnchor: ChatTimelineAnchor?
    ) async -> ChatTimelineAnchor? {
        stop()
        timelineStore.removeAll()
        self.account = account
        self.conversationID = conversationID
        self.currentUserID = currentUserID
        pinnedMessages = []
        pinnedMessagesLoaded = false
        let pinnedPreviewPreferenceKey =
            "show_pin_message_\(currentUserID)_\(conversationID)"
        self.pinnedPreviewPreferenceKey = pinnedPreviewPreferenceKey
        pinnedPreviewDismissed =
            UserDefaults.standard.object(
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
            while !Task.isCancelled,
                let revision = await messageSubscription.next()
            {
                guard let self else {
                    continue
                }
                messageRevision = revision
                guard self.needsMessageRevisionRefresh else {
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
                return nil
            }
            currentUserRole = role
        } catch {
            guard self.conversationID == conversationID else {
                return nil
            }
            currentUserRole = nil
        }
        let loadedInitialWindow =
            if let initialAnchor {
                await loadAnchorWindow(
                    initialAnchor,
                    mutation: .reset(anchor: initialAnchor),
                    presentWhenLoaded: false
                )
            } else {
                false
            }
        let unreadAnchor: ChatTimelineAnchor?
        if !loadedInitialWindow,
            unseenCount > 0,
            let lastReadMessageID
        {
            unreadAnchor = await loadUnreadWindow(
                lastReadMessageID: lastReadMessageID,
                unseenCount: unseenCount,
                presentWhenLoaded: false
            )
        } else {
            unreadAnchor = nil
        }
        let resolvedAnchor: ChatTimelineAnchor?
        if loadedInitialWindow {
            resolvedAnchor = initialAnchor
        } else if let unreadAnchor {
            resolvedAnchor = unreadAnchor
        } else if await loadLatest(
            limit: Self.initialPageSize,
            markRead: true,
            presentWhenLoaded: false
        ) {
            resolvedAnchor = .latest
        } else {
            resolvedAnchor = nil
        }
        await reloadTimelineActions()
        return resolvedAnchor
    }

    func presentInitialTimeline() {
        guard case .loading = state else {
            return
        }
        state = .ready
    }

    func reload() async {
        if messages.isEmpty {
            _ = await loadLatest(limit: Self.initialPageSize, markRead: true)
        } else {
            await refreshChangedMessages(includeRecent: true)
        }
        await reloadTimelineActions()
    }

    func jumpToLatest() async {
        _ = await loadLatest(limit: Self.initialPageSize, markRead: true)
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
    ) async throws -> [ImageMessageView] {
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
        message: MessageItem,
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
                guard
                    let appID = message.senderAppId?
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
        _ recording: VoiceRecordingDraft,
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
        let dimensions: (width: Int32, height: Int32)? =
            request.dimensions
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
        _ message: MessageItem,
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

    func transcriptMessages(transcriptID: String) async throws -> [MessageItem] {
        guard let account else {
            return []
        }
        return try await account.transcriptMessages(transcriptId: transcriptID)
    }

    func performTranscriptAttachmentAction(
        transcriptID: String,
        message: MessageItem,
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

    func recall(messages: [MessageItem]) async -> Bool {
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

    func locate(
        messageID: String,
        reuseLoadedWindow: Bool
    ) async -> Bool {
        guard account != nil, conversationID != nil else {
            return false
        }
        let anchor = ChatTimelineAnchor.message(
            id: messageID,
            alignment: .center,
            offset: 0,
            highlight: true
        )
        if reuseLoadedWindow, timelineStore.contains(messageID) {
            publishTimelineChange(.jump(anchor: anchor))
            return true
        }
        return await loadAnchorWindow(
            anchor,
            mutation: .jump(anchor: anchor)
        )
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
            prependMessages(Array(older.reversed()), mutation: .prepend)
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
            appendMessages(
                newer,
                mutation: .append(source: .history, count: newer.count)
            )
            await reloadMentionNames()
        } catch {
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    func updateVisibleMessageIDs(_ messageIDs: [String]) {
        visibleMessageIDs = Set(messageIDs)
        guard messageRevision > 0 else {
            return
        }
        let staleIDs = messageIDs.filter {
            refreshedMessageRevisions[$0] != messageRevision
        }
        guard !staleIDs.isEmpty else {
            return
        }
        let revision = messageRevision
        visibleRefreshTask?.cancel()
        visibleRefreshTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(80))
            guard !Task.isCancelled, let self else {
                return
            }
            await refreshMessageIDs(staleIDs, revision: revision)
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
        visibleRefreshTask?.cancel()
        visibleRefreshTask = nil
        recalledTextExpiryTask?.cancel()
        recalledTextExpiryTask = nil
        refreshInFlight = false
        refreshIncludesRecent = false
        refreshPending = false
        refreshRecentPending = false
        account = nil
        conversationID = nil
        currentUserID = nil
        visibleMessageIDs = []
        messageRevision = 0
        refreshedMessageRevisions = [:]
        mediaDirectory = nil
        currentUserRole = nil
        mentionNames = [:]
        mentionPresentationRevision = 0
        resolvedMentionContents = []
        pinnedMessagesLoaded = false
        pinnedPreviewPreferenceKey = nil
        recalledTexts = [:]
    }

    private func retainRecalledText(from messages: [MessageItem]) {
        let expiresAt = Date().addingTimeInterval(Self.recalledTextLifetime)
        for message in messages
        where message.category.hasSuffix("_TEXT")
            && message.senderId == currentUserID
        {
            recalledTexts[message.messageId] = RecalledText(
                content: message.content,
                expiresAt: expiresAt
            )
        }
        if recalledTexts.count > Self.recalledTextLimit {
            let overflow = recalledTexts.count - Self.recalledTextLimit
            for messageID
                in recalledTexts
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

    private func loadAnchorWindow(
        _ anchor: ChatTimelineAnchor,
        mutation: ChatTimelineMutation,
        presentWhenLoaded: Bool = true
    ) async -> Bool {
        guard case .message(let messageID, _, _, _) = anchor,
            let account,
            let conversationID
        else {
            return false
        }
        requestVersion += 1
        let version = requestVersion
        let halfPage: Int64 = 30
        do {
            let items = try await account.messagesAround(
                conversationId: conversationID,
                targetMessageId: messageID,
                before: halfPage,
                after: halfPage
            )
            guard version == requestVersion,
                self.conversationID == conversationID,
                items.contains(where: { $0.messageId == messageID })
            else {
                return false
            }
            replaceMessages(
                items,
                mutation: mutation
            )
            let targetIndex = messages.firstIndex {
                $0.messageId == messageID
            }
            hasOlderMessages = targetIndex.map { $0 >= Int(halfPage) } ?? false
            hasNewerMessages =
                targetIndex.map {
                    messages.count - $0 - 1 >= Int(halfPage)
                } ?? false
            if presentWhenLoaded {
                state = .ready
            }
            await reloadMentionNames()
            return true
        } catch {
            guard version == requestVersion else {
                return false
            }
            searchError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    private func loadLatest(
        limit: Int,
        markRead: Bool,
        presentWhenLoaded: Bool = true
    ) async -> Bool {
        guard let account, let conversationID else {
            return false
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
                return false
            }
            replaceMessages(
                Array(items.reversed()),
                mutation: .reset(anchor: .latest)
            )
            hasOlderMessages = items.count == limit
            hasNewerMessages = false
            if presentWhenLoaded {
                state = .ready
            }
            await reloadMentionNames()
            if markRead {
                try await account.markConversationRead(conversationId: conversationID)
            }
            return true
        } catch {
            guard version == requestVersion else {
                return false
            }
            state = .failed(MixinErrorPresenter.message(for: error))
            return false
        }
    }

    private func loadUnreadWindow(
        lastReadMessageID: String,
        unseenCount _: Int64,
        presentWhenLoaded: Bool
    ) async -> ChatTimelineAnchor? {
        guard let account, let conversationID else {
            return nil
        }
        requestVersion += 1
        let version = requestVersion
        do {
            let items = try await account.messagesAround(
                conversationId: conversationID,
                targetMessageId: lastReadMessageID,
                before: 30,
                after: 30
            )
            guard version == requestVersion,
                self.conversationID == conversationID,
                let readIndex = items.firstIndex(where: {
                    $0.messageId == lastReadMessageID
                })
            else {
                return nil
            }
            let initialMessageID =
                items.indices.contains(readIndex + 1)
                ? items[readIndex + 1].messageId
                : items.last?.messageId
            let anchor =
                initialMessageID.map {
                    ChatTimelineAnchor.message(
                        id: $0,
                        alignment: .focus,
                        offset: 0,
                        highlight: false
                    )
                } ?? .latest
            replaceMessages(
                items,
                mutation: .reset(anchor: anchor)
            )
            hasOlderMessages = readIndex >= 30
            let sortedReadIndex = messages.firstIndex {
                $0.messageId == lastReadMessageID
            }
            hasNewerMessages =
                sortedReadIndex.map {
                    messages.count - $0 - 1 >= 30
                } ?? false
            if presentWhenLoaded {
                state = .ready
            }
            try await account.markConversationRead(conversationId: conversationID)
            return anchor
        } catch {
            guard version == requestVersion else {
                return nil
            }
            return nil
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
        let refreshIDs = visibleMessageIDs.union(mutableMessageIDs)

        do {
            async let loaded = account.messageItemsByIds(
                messageIds: Array(refreshIDs)
            )
            let recentItems: [MessageItem]
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

            let loadedIDs = Set(loadedItems.map(\.messageId))
            let removedIDs = refreshIDs.subtracting(loadedIDs)
            for messageID in refreshIDs {
                refreshedMessageRevisions[messageID] = messageRevision
            }
            if includeRecent, !hasNewerMessages {
                let newestID = currentMessages.last?.messageId
                let boundaryIndex = newestID.flatMap { newestID in
                    recentItems.firstIndex {
                        $0.messageId == newestID
                    }
                }
                let recentUpdates = recentItems.filter {
                    timelineStore.contains($0.messageId)
                }
                let liveItems =
                    boundaryIndex.map {
                        Array(recentItems[..<$0].reversed())
                    } ?? []
                let sentByCurrentUser = liveItems.contains {
                    $0.senderId == currentUserID
                }
                let updated = timelineStore.update(
                    loadedItems + recentUpdates,
                    removingIDs: removedIDs
                )
                let appended = timelineStore.append(liveItems)
                if updated || appended {
                    windowVersion += 1
                    publishTimelineChange(
                        liveItems.isEmpty
                            ? .update
                            : .append(
                                source: .live(
                                    sentByCurrentUser: sentByCurrentUser
                                ),
                                count: liveItems.count
                            )
                    )
                }
                hasNewerMessages = boundaryIndex == nil
            } else {
                updateMessages(
                    loadedItems,
                    removingIDs: removedIDs,
                    mutation: .update
                )
            }
            await reloadMentionNames()
        } catch {
            guard self.conversationID == conversationID else {
                return
            }
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func refreshMessageIDs(
        _ messageIDs: [String],
        revision: UInt64
    ) async {
        guard let account,
            let conversationID,
            !messageIDs.isEmpty
        else {
            return
        }
        let version = windowVersion
        do {
            let loaded = try await account.messageItemsByIds(
                messageIds: messageIDs
            )
            guard self.conversationID == conversationID,
                version == windowVersion
            else {
                return
            }
            let requested = Set(messageIDs)
            let loadedIDs = Set(loaded.map(\.messageId))
            for messageID in requested {
                refreshedMessageRevisions[messageID] = revision
            }
            updateMessages(
                loaded,
                removingIDs: requested.subtracting(loadedIDs),
                mutation: .update
            )
        } catch {
            guard self.conversationID == conversationID else {
                return
            }
            mutationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func prependMessages(
        _ items: [MessageItem],
        mutation: ChatTimelineMutation
    ) {
        guard timelineStore.prepend(items) else {
            return
        }
        windowVersion += 1
        publishTimelineChange(mutation)
    }

    private func appendMessages(
        _ items: [MessageItem],
        mutation: ChatTimelineMutation
    ) {
        guard timelineStore.append(items) else {
            return
        }
        windowVersion += 1
        publishTimelineChange(mutation)
    }

    private func updateMessages(
        _ items: [MessageItem],
        removingIDs: Set<String> = [],
        mutation: ChatTimelineMutation
    ) {
        guard timelineStore.update(items, removingIDs: removingIDs) else {
            return
        }
        windowVersion += 1
        publishTimelineChange(mutation)
    }

    private func replaceMessages(
        _ items: [MessageItem],
        mutation: ChatTimelineMutation
    ) {
        guard timelineStore.reset(with: items) else {
            switch mutation {
            case .reset, .jump:
                publishTimelineChange(mutation)
            case .prepend, .append, .update:
                break
            }
            return
        }
        windowVersion += 1
        publishTimelineChange(mutation)
    }

    private func publishTimelineChange(
        _ mutation: ChatTimelineMutation
    ) {
        timelineRevision += 1
        timelineChange = ChatTimelineChange(
            revision: timelineRevision,
            mutation: mutation
        )
    }

    private var needsMessageRevisionRefresh: Bool {
        !visibleMessageIDs.isEmpty || !timelineStore.mutableMessageIDs.isEmpty
    }

    private var mutableMessageIDs: Set<String> {
        timelineStore.mutableMessageIDs
    }

    private func reloadMentionNames() async {
        guard let account, let conversationID else {
            return
        }
        let auxiliaryContents = (searchResults + pinnedMessages)
            .flatMap { message in
                [message.content, message.caption, message.quoteContent]
                    .compactMap { $0 }
            }
            .filter { $0.range(of: #"@\d{4,}"#, options: .regularExpression) != nil }
        let contents = timelineStore.mentionContents.union(auxiliaryContents)
        let unresolved = contents.subtracting(resolvedMentionContents)
        guard !unresolved.isEmpty else {
            return
        }
        mentionRevision += 1
        let revision = mentionRevision
        do {
            let names = try await account.mentionNames(
                contents: Array(unresolved)
            )
            guard revision == mentionRevision,
                self.conversationID == conversationID
            else {
                return
            }
            let previousNames = mentionNames
            mentionNames.merge(names) { _, new in new }
            if mentionNames != previousNames {
                mentionPresentationRevision += 1
            }
            resolvedMentionContents.formUnion(unresolved)
        } catch {
            guard revision == mentionRevision,
                self.conversationID == conversationID
            else {
                return
            }
        }
    }

    private func deduplicated(_ items: [MessageItem]) -> [MessageItem] {
        var seen = Set<String>()
        return items.filter { seen.insert($0.messageId).inserted }
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

extension MessageItem {
    var hasInvalidImagePayload: Bool {
        category.hasSuffix("_IMAGE") && (mediaWidth == nil || mediaHeight == nil)
    }

    var hasInvalidSpecialPayload: Bool {
        let category = category.uppercased()
        guard
            category.hasSuffix("_LOCATION")
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
            || category.hasSuffix("_POST")
            || category.hasSuffix("_TRANSCRIPT")
    }

    var usesOuterMetadata: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_STICKER")
            || category.hasSuffix("_LOCATION")
            || category.hasSuffix("_CONTACT")
            || category.hasSuffix("_DATA")
            || category.hasSuffix("_AUDIO")
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
        if category.hasSuffix("_IMAGE"),
           caption?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
               != false
        {
            return false
        }
        if isActionsAppCard {
            return false
        }
        return !category.hasSuffix("_STICKER")
            && category != "APP_BUTTON_GROUP"
    }

    var includesBubbleNipInContent: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_IMAGE")
            && caption?.trimmingCharacters(
                in: .whitespacesAndNewlines
            ).isEmpty != false
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_LIVE")
            || category.hasSuffix("_LOCATION")
            || category == "SYSTEM_SAFE_INSCRIPTION"
    }

    var clipsBubbleContent: Bool {
        let category = category.uppercased()
        return category.hasSuffix("_IMAGE")
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_LIVE")
            || category.hasSuffix("_STICKER")
            || category.hasSuffix("_LOCATION")
            || category == "SYSTEM_SAFE_INSCRIPTION"
    }

    var isDisappearing: Bool {
        (expireIn ?? 0) > 0
    }

    var messageContentInsets: EdgeInsets {
        let category = category.uppercased()
        if category.hasSuffix("_IMAGE")
            || category.hasSuffix("_VIDEO")
            || category.hasSuffix("_LIVE")
            || category.hasSuffix("_LOCATION")
            || category.hasSuffix("_STICKER")
            || category == "APP_BUTTON_GROUP"
            || isActionsAppCard
        {
            return EdgeInsets()
        }
        if category.hasSuffix("_TRANSCRIPT") {
            return EdgeInsets(top: 4, leading: 2, bottom: 2, trailing: 2)
        }
        return EdgeInsets(top: 8, leading: 8, bottom: 8, trailing: 8)
    }

    private var isActionsAppCard: Bool {
        guard category.uppercased() == "APP_CARD",
              let card = try? JSONDecoder().decode(
                AppCardContent.self,
                from: Data(content.utf8)
              )
        else {
            return false
        }
        return card.action.isEmpty
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

    var selectionPreview: String {
        let category = category.uppercased()
        if category.hasSuffix("_TEXT") {
            return content
        }
        if category.contains("SNAPSHOT") { return "[Transfer]" }
        if category.hasSuffix("_STICKER") { return "[Sticker]" }
        if category.hasSuffix("_IMAGE") { return "[Image]" }
        if category.hasSuffix("_VIDEO") { return "[Video]" }
        if category.hasSuffix("_LIVE") { return "[Live]" }
        if category.hasSuffix("_DATA") { return "[File]" }
        if category.hasSuffix("_POST") {
            let value = content.trimmingCharacters(
                in: .whitespacesAndNewlines
            )
            return value.isEmpty ? "Post" : value
        }
        if category.hasSuffix("_LOCATION") { return "[Location]" }
        if category.hasSuffix("_AUDIO") { return "[Audio]" }
        if category.hasSuffix("_CONTACT") { return "[Contact]" }
        if category.hasSuffix("_TRANSCRIPT") { return "[Transcript]" }
        if category.contains("INSCRIPTION") { return "[Collectible]" }
        if category == "APP_BUTTON_GROUP",
           let data = content.data(using: .utf8),
           let buttons = try? JSONSerialization.jsonObject(
               with: data
           ) as? [[String: Any]]
        {
            return buttons.map {
                "[\($0["label"] as? String ?? "")]"
            }.joined()
        }
        if category == "APP_CARD",
           let data = content.data(using: .utf8),
           let card = try? JSONSerialization.jsonObject(
               with: data
           ) as? [String: Any]
        {
            return "[\(card["title"] as? String ?? "Card")]"
        }
        return "Unsupported message"
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
        let participant =
            participantFullName?.isEmpty == false
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

extension Int64 {
    fileprivate var messageDate: Date {
        Date(timeIntervalSince1970: Double(self) / 1_000_000)
    }

    fileprivate var formattedDuration: String {
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

    fileprivate var calendarDay: Date {
        Calendar.current.startOfDay(for: messageDate)
    }

    fileprivate var dayChipTitle: String {
        if Calendar.current.isDateInToday(messageDate) {
            return "Today"
        }

        let formatter = DateFormatter()
        formatter.locale = .current
        formatter.calendar = .current
        let format = Calendar.current.isDate(messageDate, equalTo: .now, toGranularity: .year)
            ? "MMMEd"
            : "yMMMEd"
        formatter.setLocalizedDateFormatFromTemplate(format)
        return formatter.string(from: messageDate)
    }
}
