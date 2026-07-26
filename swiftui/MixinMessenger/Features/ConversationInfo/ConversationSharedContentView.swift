import Observation
import SwiftUI

struct ConversationSharedContentView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @State private var model = ConversationSharedContentModel()
    @State private var selection = ConversationSharedContentKind.media

    let conversationID: String

    var body: some View {
        VStack(spacing: 0) {
                Picker("Content type", selection: $selection) {
                    ForEach(ConversationSharedContentKind.allCases) { kind in
                        Text(kind.title).tag(kind)
                    }
                }
                .pickerStyle(.segmented)
                .padding(12)

                Divider()

                content
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
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(message):
            ContentUnavailableView(
                "Unable to load \(selection.title.lowercased())",
                systemImage: "exclamationmark.triangle",
                description: Text(message)
            )
        case .ready where model.messages.isEmpty:
            ContentUnavailableView(
                selection.emptyTitle,
                systemImage: selection.systemImage
            )
        case .ready:
            if selection == .media {
                mediaGrid
            } else {
                messageList
            }
        }
    }

    private var mediaGrid: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 12, pinnedViews: [.sectionHeaders]) {
                ForEach(model.groups) { group in
                    Section {
                        LazyVGrid(
                            columns: [
                                GridItem(.adaptive(minimum: 112), spacing: 6),
                            ],
                            spacing: 6
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

    private var messageList: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0, pinnedViews: [.sectionHeaders]) {
                ForEach(model.groups) { group in
                    Section {
                        ForEach(group.messages, id: \.messageId) { message in
                            ConversationContentMessageRow(
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
                            .padding(.horizontal, 12)
                            .onAppear {
                                loadMoreIfNeeded(message)
                            }
                            Divider()
                                .padding(.leading, 56)
                        }
                    } header: {
                        dateHeader(group.day)
                    }
                }

                loadingFooter
            }
        }
    }

    private func dateHeader(_ date: Date) -> some View {
        Text(date.formatted(date: .abbreviated, time: .omitted))
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 12)
            .padding(.vertical, 9)
            .background(.bar)
    }

    @ViewBuilder
    private var loadingFooter: some View {
        if model.loadingMore {
            ProgressView()
                .frame(maxWidth: .infinity)
                .padding()
        }
    }

    private func loadMoreIfNeeded(_ message: SwiftMessageItem) {
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

    private func locate(_ message: SwiftMessageItem) {
        navigation.locateMessage(
            conversationID: conversationID,
            messageID: message.messageId
        )
        dismiss()
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
            "Posts"
        case .file:
            "Files"
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

    var systemImage: String {
        switch self {
        case .media:
            "photo.on.rectangle.angled"
        case .post:
            "doc.richtext"
        case .file:
            "folder"
        }
    }

    var pageSize: UInt32 {
        self == .media ? 60 : 30
    }
}

private struct SharedMediaGridItem: View {
    let message: SwiftMessageItem
    let imageMessages: [SwiftMessageItem]
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
                    Label(
                        message.mediaDuration,
                        systemImage: "play.fill"
                    )
                    .font(.caption2)
                    .foregroundStyle(.white)
                    .padding(6)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(
                        LinearGradient(
                            colors: [.black.opacity(0.55), .clear],
                            startPoint: .bottom,
                            endPoint: .top
                        )
                    )
                }
            }
            .background(Color.secondary.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: 5))
        }
        .buttonStyle(.plain)
        .sheet(isPresented: $previewPresented) {
            NavigationStack {
                ConversationContentMessageRow(
                    message: message,
                    imageMessages: imageMessages,
                    account: account,
                    currentUserID: currentUserID
                )
                .padding(20)
                .frame(minWidth: 420, minHeight: 360)
                .navigationTitle(message.mediaName ?? "Media")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Close") {
                            previewPresented = false
                        }
                    }
                }
            }
        }
    }
}

struct ConversationSharedContentGroup: Identifiable {
    let day: Date
    let messages: [SwiftMessageItem]

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
    private(set) var messages: [SwiftMessageItem] = []
    private(set) var hasMore = true
    private(set) var loadingMore = false

    private var subscription: SwiftMessageSubscription?
    private var subscriptionTask: Task<Void, Never>?
    private var requestVersion = 0

    var imageMessages: [SwiftMessageItem] {
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
