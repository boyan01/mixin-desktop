import Observation
import SwiftUI

struct ConversationSharedContentView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationSharedContentModel()
    @State private var selection = ConversationSharedContentKind.media

    let conversationID: String

    var body: some View {
        VStack(spacing: 0) {
            content

            HStack(spacing: 0) {
                ForEach(ConversationSharedContentKind.allCases) { kind in
                    Button {
                        selection = kind
                    } label: {
                        Text(kind.title)
                            .font(.system(size: 14))
                            .foregroundStyle(
                                selection == kind ? theme.accent : theme.secondaryText
                            )
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                    .buttonStyle(.plain)
                }
            }
            .frame(height: 56)
        }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Shared Media")
        .task(id: SharedContentTaskID(
            conversationID: conversationID,
            kind: selection
        )) {
            await model.start(
                account: session.handle,
                conversationID: conversationID,
                kind: selection
            )
        }
        .onDisappear {
            model.stop()
        }
    }

    @ViewBuilder
    private var content: some View {
        switch model.state {
        case .loading:
            emptyContent
        case .failed:
            emptyContent
        case .ready where model.messages.isEmpty:
            emptyContent
        case .ready:
            if selection == .media {
                mediaGrid
            } else {
                messageList
            }
        }
    }

    private var mediaGrid: some View {
        GeometryReader { proxy in
            let columnCount = proxy.size.width >= 600 ? 4 : 3
            AppScrollView {
                LazyVStack(alignment: .leading, spacing: 0, pinnedViews: [.sectionHeaders]) {
                    ForEach(model.groups) { group in
                        Section {
                            LazyVGrid(
                                columns: Array(
                                    repeating: GridItem(.flexible(), spacing: 5),
                                    count: columnCount
                                ),
                                spacing: 5
                            ) {
                                ForEach(group.messages, id: \.messageId) { message in
                                    SharedMediaGridItem(
                                        message: message,
                                        imageMessages: model.imageMessages,
                                        account: session.handle,
                                        currentUserID: session.profile.userId
                                    )
                                    .contextMenu {
                                        Button("Locate in Chat", systemImage: "scope") {
                                            locate(message)
                                        }
                                    }
                                    .onAppear {
                                        loadMoreIfNeeded(message)
                                    }
                                }
                            }
                            .padding(.horizontal, 10)
                        } header: {
                            dateHeader(group.day)
                        }
                    }

                    loadingFooter
                }
            }
        }
    }

    private var messageList: some View {
        AppScrollView {
            LazyVStack(alignment: .leading, spacing: 0, pinnedViews: [.sectionHeaders]) {
                ForEach(model.groups) { group in
                    Section {
                        ForEach(group.messages, id: \.messageId) { message in
                            sharedMessageRow(message)
                            .contextMenu {
                                Button("Locate in Chat", systemImage: "scope") {
                                    locate(message)
                                }
                            }
                            .onAppear {
                                loadMoreIfNeeded(message)
                            }
                        }
                    } header: {
                        dateHeader(group.day)
                    }
                }

                loadingFooter
            }
        }
    }

    @ViewBuilder
    private func sharedMessageRow(_ message: MessageItem) -> some View {
        switch selection {
        case .post:
            SharedPostMessageRow(message: message)
        case .file:
            SharedFileMessageRow(
                message: message,
                account: session.handle,
                currentUserID: session.profile.userId
            )
        case .media:
            EmptyView()
        }
    }

    private func dateHeader(_ date: Date) -> some View {
        Text(date.formatted(date: .abbreviated, time: .omitted))
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(10)
            .frame(height: 38)
            .background(theme.primary)
    }

    private var emptyContent: some View {
        VStack(spacing: 0) {
            Image(selection.emptyAssetName)
                .resizable()
                .renderingMode(.template)
                .foregroundStyle(theme.secondaryText.opacity(0.4))
                .frame(
                    width: selection == .media ? 74 : 72,
                    height: selection == .media ? 80 : 79
                )
            Spacer().frame(height: 24)
            Text(selection.emptyTitle)
                .font(.system(size: 12))
                .foregroundStyle(theme.secondaryText)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder
    private var loadingFooter: some View {
        if model.loadingMore {
            ProgressView()
                .frame(maxWidth: .infinity)
                .padding()
        }
    }

    private func loadMoreIfNeeded(_ message: MessageItem) {
        guard message.messageId == model.messages.last?.messageId else {
            return
        }
        Task {
            await model.loadMore(
                account: session.handle,
                conversationID: conversationID,
                kind: selection
            )
        }
    }

    private func locate(_ message: MessageItem) {
        navigation.locateMessage(
            conversationID: conversationID,
            messageID: message.messageId
        )
        dismiss()
    }
}

private struct SharedPostMessageRow: View {
    @Environment(\.mixinTheme) private var theme
    let message: MessageItem

    @State private var previewPresented = false

    var body: some View {
        PostMessageView(
            message: message,
            minimumHeight: 0,
            contentPadding: 8,
            background: theme.sidebarSelected
        ) {
            previewPresented = true
        }
        .padding(.leading, 10)
        .padding(.trailing, 10)
        .padding(.bottom, 10)
        .sheet(isPresented: $previewPresented) {
            PostMessagePreview(message: message)
        }
    }
}

private struct SharedFileMessageRow: View {
    let message: MessageItem
    let account: SwiftAccountHandle
    let currentUserID: String

    @State private var operationError: String?

    var body: some View {
        FileMessageView(
            message: message,
            outgoing: message.senderId == currentUserID,
            progress: {
                account.attachmentProgress(messageId: message.messageId)
            },
            onAttachmentAction: {
                Task {
                    await performAttachmentAction()
                }
            }
        )
        .padding(.vertical, 8)
        .padding(.horizontal, 16)
        .alert(
            "Attachment action failed",
            isPresented: Binding(
                get: { operationError != nil },
                set: { if !$0 { operationError = nil } }
            )
        ) {
            Button("OK") {
                operationError = nil
            }
        } message: {
            Text(operationError ?? "")
        }
    }

    private func performAttachmentAction() async {
        do {
            switch message.mediaStatus.uppercased() {
            case "CANCELED":
                if message.senderId == currentUserID {
                    try await account.retryAttachment(messageId: message.messageId)
                } else {
                    try await account.downloadAttachment(messageId: message.messageId)
                }
            case "PENDING":
                try await account.cancelAttachment(messageId: message.messageId)
            default:
                break
            }
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }
}

private struct SharedContentTaskID: Hashable {
    let conversationID: String
    let kind: ConversationSharedContentKind
}

enum ConversationSharedContentKind: String, CaseIterable, Identifiable {
    case media
    case post
    case file

    var id: String { rawValue }

    var title: String {
        switch self {
        case .media:
            "Media"
        case .post:
            "Post"
        case .file:
            "File"
        }
    }

    var emptyTitle: String {
        switch self {
        case .media:
            "No Media"
        case .post:
            "No Posts"
        case .file:
            "No Files"
        }
    }

    var emptyAssetName: String {
        switch self {
        case .media:
            "EmptyImage"
        case .post, .file:
            "EmptyFile"
        }
    }

    var pageSize: UInt32 {
        self == .media ? 60 : 30
    }
}

private struct SharedMediaGridItem: View {
    let message: MessageItem
    let imageMessages: [MessageItem]
    let account: SwiftAccountHandle
    let currentUserID: String

    @State private var previewPresented = false

    var body: some View {
        Button {
            previewPresented = true
        } label: {
            ZStack(alignment: .bottomLeading) {
                MessageMediaImage(
                    source: message.mediaStatus.isComplete
                        ? (message.category.hasSuffix("_IMAGE")
                            ? message.mediaUrl
                            : message.thumbUrl)
                        : nil,
                    thumbnail: message.thumbImage,
                    contentMode: .fill
                )
                .frame(maxWidth: .infinity)
                .aspectRatio(1, contentMode: .fit)
                .clipped()

                if message.category.hasSuffix("_VIDEO") {
                    HStack(spacing: 8) {
                        Image(systemName: "play.fill")
                            .font(.system(size: 12))
                        Text(AudioMessageView.format(
                            Int64(message.mediaDuration) ?? 0
                        ))
                        .font(.system(size: 12).monospacedDigit())
                    }
                    .foregroundStyle(.white)
                    .padding(.leading, 5)
                    .frame(maxWidth: .infinity, minHeight: 20, alignment: .leading)
                    .background(
                        LinearGradient(
                            colors: [.black.opacity(0.5), .clear],
                            startPoint: .bottom,
                            endPoint: .top
                        )
                    )
                }
            }
            .background(Color.secondary.opacity(0.08))
        }
        .buttonStyle(.plain)
        .sheet(isPresented: $previewPresented) {
            if message.category.hasSuffix("_VIDEO") {
                VideoMessagePreview(message: message)
            } else {
                ImageMessagePreview(
                    messages: imageMessages,
                    initialMessageID: message.messageId,
                    loadWindow: { messageID in
                        try await account.imageMessagesAround(
                            conversationId: message.conversationId,
                            targetMessageId: messageID,
                            before: 40,
                            after: 40
                        )
                    }
                )
            }
        }
    }
}

struct ConversationSharedContentGroup: Identifiable {
    let day: Date
    let messages: [MessageItem]

    var id: Date { day }
}

@MainActor
@Observable
final class ConversationSharedContentModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var messages: [MessageItem] = []
    private(set) var hasMore = true
    private(set) var loadingMore = false

    private var subscription: SwiftMessageSubscription?
    private var subscriptionTask: Task<Void, Never>?
    private var requestVersion = 0

    var imageMessages: [MessageItem] {
        messages.filter { $0.category.hasSuffix("_IMAGE") }
    }

    var groups: [ConversationSharedContentGroup] {
        let calendar = Calendar.current
        let grouped = Dictionary(grouping: messages) {
            calendar.startOfDay(for: $0.conversationContentDate)
        }
        return grouped.keys.sorted(by: >).map {
            ConversationSharedContentGroup(
                day: $0,
                messages: grouped[$0] ?? []
            )
        }
    }

    func start(
        account: SwiftAccountHandle,
        conversationID: String,
        kind: ConversationSharedContentKind
    ) async {
        stop()
        messages = []
        hasMore = true
        loadingMore = false
        state = .loading

        let subscription = account.messageChanges()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled, await subscription.next() != nil {
                await self?.refresh(
                    account: account,
                    conversationID: conversationID,
                    kind: kind
                )
            }
        }
        await loadMore(
            account: account,
            conversationID: conversationID,
            kind: kind
        )
    }

    func stop() {
        requestVersion += 1
        subscriptionTask?.cancel()
        subscriptionTask = nil
        subscription?.cancel()
        subscription = nil
    }

    func loadMore(
        account: SwiftAccountHandle,
        conversationID: String,
        kind: ConversationSharedContentKind
    ) async {
        guard hasMore, !loadingMore else {
            return
        }
        loadingMore = true
        defer { loadingMore = false }
        requestVersion += 1
        let version = requestVersion
        do {
            let page = try await account.sharedMessages(
                conversationId: conversationID,
                kind: kind.rawValue,
                offset: UInt32(messages.count),
                limit: kind.pageSize
            )
            guard version == requestVersion else {
                return
            }
            messages.append(contentsOf: page)
            hasMore = page.count == Int(kind.pageSize)
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            if messages.isEmpty {
                state = .failed(MixinErrorPresenter.message(for: error))
            }
        }
    }

    private func refresh(
        account: SwiftAccountHandle,
        conversationID: String,
        kind: ConversationSharedContentKind
    ) async {
        requestVersion += 1
        let version = requestVersion
        let targetCount = max(messages.count, Int(kind.pageSize))
        do {
            let loaded = try await account.sharedMessages(
                conversationId: conversationID,
                kind: kind.rawValue,
                offset: 0,
                limit: UInt32(targetCount)
            )
            guard version == requestVersion else {
                return
            }
            messages = loaded
            hasMore = loaded.count == targetCount
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            if messages.isEmpty {
                state = .failed(MixinErrorPresenter.message(for: error))
            }
        }
    }
}
