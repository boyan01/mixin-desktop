import AppKit
import Observation
import SwiftUI

struct ConversationPinnedMessagesView: View {
    @Environment(AccountSession.self) private var session
    @Environment(AppModel.self) private var appModel
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationPinnedMessagesModel()
    @State private var unpinAllPresented = false

    let conversationID: String

    var body: some View {
        VStack(spacing: 0) {
            Group {
                switch model.state {
                case .loading:
                    EmptyView()
                case .failed:
                    EmptyView()
                case .ready where model.messages.isEmpty:
                    EmptyView()
                case .ready:
                    AppScrollView {
                        LazyVStack(spacing: 0) {
                            let messages = Array(model.messages.reversed())
                            ForEach(Array(messages.enumerated()), id: \.element.messageId) { index, message in
                                if index == 0 || !Calendar.current.isDate(
                                    message.conversationContentDate,
                                    inSameDayAs: messages[index - 1].conversationContentDate
                                ) {
                                    dayHeader(message)
                                }
                                PinnedMessageRow(
                                    message: message,
                                    previous: index > 0 ? messages[index - 1] : nil,
                                    next: index + 1 < messages.count
                                        ? messages[index + 1]
                                        : nil,
                                    messages: messages,
                                    model: model,
                                    account: session.handle,
                                    currentUserID: session.profile.userId,
                                    currentUserRole: model.currentUserRole,
                                    currency: session.profile.fiatCurrency,
                                    desktop: appModel.desktopHandle,
                                    onLocate: {
                                        navigation.locateMessage(
                                            conversationID: conversationID,
                                            messageID: message.messageId
                                        )
                                        dismiss()
                                    }
                                )
                                .contentShape(Rectangle())
                                .onTapGesture {
                                    navigation.locateMessage(
                                        conversationID: conversationID,
                                        messageID: message.messageId
                                    )
                                    dismiss()
                                }
                            }
                        }
                        .padding(.bottom, 16)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            Button("Unpin All") {
                unpinAllPresented = true
            }
            .buttonStyle(.plain)
            .font(.system(size: 16))
            .foregroundStyle(theme.accent)
            .frame(maxWidth: .infinity)
            .frame(height: 56)
            .disabled(model.mutating)
        }
        .background(theme.popUp)
        .navigationTitle("Pinned Messages (\(model.messages.count))")
        .task(id: conversationID) {
            await model.start(
                account: session.handle,
                conversationID: conversationID
            )
        }
        .onChange(of: model.shouldDismiss) {
            if $0 {
                dismiss()
            }
        }
        .onDisappear {
            model.stop()
        }
        .confirmationDialog(
            "Unpin all messages?",
            isPresented: $unpinAllPresented
        ) {
            Button("Unpin All", role: .destructive) {
                Task {
                    await model.unpinAll(
                        account: session.handle,
                        conversationID: conversationID
                    )
                }
            }
            Button("Cancel", role: .cancel) {}
        }
        .alert(
            "Unable to update pinned messages",
            isPresented: Binding(
                get: { model.operationError != nil },
                set: { if !$0 { model.operationError = nil } }
            )
        ) {
            Button("OK") {
                model.operationError = nil
            }
        } message: {
            Text(model.operationError ?? "")
        }
    }

    private func dayHeader(_ message: MessageItem) -> some View {
        Text(dayChipTitle(message.conversationContentDate))
            .font(.system(size: 14))
            .foregroundStyle(.black)
            .frame(minWidth: 64)
            .padding(.horizontal, 10)
            .padding(.vertical, 2)
            .background(theme.dateTime, in: RoundedRectangle(cornerRadius: 10))
            .padding(.top, 16)
            .padding(.bottom, 10)
            .frame(maxWidth: .infinity)
    }

    private func dayChipTitle(_ date: Date) -> String {
        if Calendar.current.isDateInToday(date) {
            return "Today"
        }
        let formatter = DateFormatter()
        formatter.locale = .current
        formatter.calendar = .current
        formatter.setLocalizedDateFormatFromTemplate(
            Calendar.current.isDate(date, equalTo: .now, toGranularity: .year)
                ? "MMMEd"
                : "yMMMEd"
        )
        return formatter.string(from: date)
    }
}

@MainActor
@Observable
final class ConversationPinnedMessagesModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var messages: [MessageItem] = []
    private(set) var mentionNames: [String: String] = [:]
    private(set) var currentUserRole: String?
    private(set) var mediaDirectory: URL?
    private(set) var mutating = false
    var operationError: String?

    private var subscription: SwiftMessageSubscription?
    private var subscriptionTask: Task<Void, Never>?
    private var requestVersion = 0

    var imageMessages: [MessageItem] {
        messages.filter { $0.category.hasSuffix("_IMAGE") }
    }

    var shouldDismiss: Bool {
        if case .ready = state {
            return messages.isEmpty
        }
        return false
    }

    func start(
        account: SwiftAccountHandle,
        conversationID: String
    ) async {
        stop()
        state = .loading
        mediaDirectory = (try? account.mediaDirectory()).map {
            URL(fileURLWithPath: $0, isDirectory: true)
        }
        let subscription = account.messageChanges()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled, await subscription.next() != nil {
                await self?.reload(
                    account: account,
                    conversationID: conversationID
                )
            }
        }
        do {
            currentUserRole = try await account.currentUserRole(
                conversationId: conversationID
            )
        } catch {
            currentUserRole = nil
        }
        await reload(account: account, conversationID: conversationID)
    }

    func stop() {
        requestVersion += 1
        subscriptionTask?.cancel()
        subscriptionTask = nil
        subscription?.cancel()
        subscription = nil
    }

    func unpin(
        account: SwiftAccountHandle,
        conversationID: String,
        messageID: String
    ) async {
        guard !mutating else {
            return
        }
        mutating = true
        defer { mutating = false }
        do {
            try await account.setMessagePinned(
                conversationId: conversationID,
                messageId: messageID,
                pinned: false
            )
            await reload(account: account, conversationID: conversationID)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func unpinAll(
        account: SwiftAccountHandle,
        conversationID: String
    ) async {
        guard !mutating else {
            return
        }
        mutating = true
        defer { mutating = false }
        do {
            for message in messages {
                try await account.setMessagePinned(
                    conversationId: conversationID,
                    messageId: message.messageId,
                    pinned: false
                )
            }
            await reload(account: account, conversationID: conversationID)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func performAttachmentAction(
        account: SwiftAccountHandle,
        message: MessageItem,
        currentUserID: String
    ) async {
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
                return
            }
            await reload(account: account, conversationID: message.conversationId)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func performTranscriptAttachmentAction(
        account: SwiftAccountHandle,
        transcriptID: String,
        message: MessageItem,
        currentUserID: String
    ) async {
        do {
            switch message.mediaStatus.uppercased() {
            case "CANCELED":
                if message.senderId == currentUserID {
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
                return
            }
            await reload(account: account, conversationID: message.conversationId)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func markAudioRead(
        account: SwiftAccountHandle,
        messageID: String,
        conversationID: String
    ) async {
        do {
            try await account.markAudioRead(messageId: messageID)
            await reload(account: account, conversationID: conversationID)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func reload(
        account: SwiftAccountHandle,
        conversationID: String
    ) async {
        requestVersion += 1
        let version = requestVersion
        do {
            let loaded = try await account.pinnedMessages(
                conversationId: conversationID
            )
            guard version == requestVersion else {
                return
            }
            messages = loaded
            let contents = loaded.flatMap { message in
                [message.content, message.caption, message.quoteContent]
                    .compactMap { $0 }
            }
            mentionNames = (try? await account.mentionNames(contents: contents)) ?? [:]
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            if messages.isEmpty {
                state = .failed(MixinErrorPresenter.message(for: error))
            } else {
                operationError = MixinErrorPresenter.message(for: error)
            }
        }
    }
}

private struct PinnedMessageRow: View {
    @State private var stickerDetailID: String?

    let message: MessageItem
    let previous: MessageItem?
    let next: MessageItem?
    let messages: [MessageItem]
    let model: ConversationPinnedMessagesModel
    let account: SwiftAccountHandle
    let currentUserID: String
    let currentUserRole: String?
    let currency: String
    let desktop: SwiftDesktopHandle?
    let onLocate: () -> Void

    var body: some View {
        MessageRow(
            message: message,
            mentionNames: model.mentionNames,
            mentionNamesRevision: model.mentionNames.hashValue,
            audioPlaylist: message.presentationKind == .audio ? messages : [],
            mediaIndexRevision: messages.hashValue,
            mediaDirectory: model.mediaDirectory,
            conversationName: nil,
            groupPresentation:
                message.conversationCategory == "GROUP"
                || message.senderId
                    != (message.conversationOwnerId ?? ""),
            outgoing: message.senderId == currentUserID,
            sameUserPrevious: isSameGroup(previous, before: message),
            sameUserNext: isSameGroup(next, after: message),
            policy: MessageActionPolicy(
                message: message,
                currentUserID: currentUserID,
                currentUserRole: currentUserRole,
                now: Date()
            ),
            recalledText: nil,
            selected: false,
            selectionActive: false,
            imageMessages: message.isRichContent
                ? messages.filter { $0.presentationKind == .image }
                : [],
            loadImageWindow: { messageID in
                try await account.imageMessagesAround(
                    conversationId: message.conversationId,
                    targetMessageId: messageID,
                    before: 40,
                    after: 40
                )
            },
            onReply: onLocate,
            onCopy: copyText,
            onSelect: {},
            onToggleSelection: {},
            onTogglePin: {
                Task {
                    await model.unpin(
                        account: account,
                        conversationID: message.conversationId,
                        messageID: message.messageId
                    )
                }
            },
            onRecall: {},
            onDelete: {},
            onReedit: { _ in },
            attachmentProgress: account.attachmentProgress(
                messageId: message.messageId
            ),
            onAttachmentAction: {
                Task {
                    await model.performAttachmentAction(
                        account: account,
                        message: message,
                        currentUserID: currentUserID
                    )
                }
            },
            loadTranscript: {
                try await account.transcriptMessages(transcriptId: message.messageId)
            },
            onTranscriptAttachmentAction: { item in
                await model.performTranscriptAttachmentAction(
                    account: account,
                    transcriptID: message.messageId,
                    message: item,
                    currentUserID: currentUserID
                )
            },
            onMarkAudioRead: { messageID in
                Task {
                    await model.markAudioRead(
                        account: account,
                        messageID: messageID,
                        conversationID: message.conversationId
                    )
                }
            },
            onShowStickerDetail: { stickerDetailID = $0 },
            onAppAction: { action, title in
                Task {
                    do {
                        try await MessageActionHandler(
                            account: account,
                            conversationID: message.conversationId,
                            currency: currency
                        ).open(action, title: title)
                    } catch {
                        model.operationError = MixinErrorPresenter.message(for: error)
                    }
                }
            },
            onStrangerAction: { _ in false },
            isPinnedPage: true
        )
        .sheet(
            isPresented: Binding(
                get: { stickerDetailID != nil },
                set: { if !$0 { stickerDetailID = nil } }
            )
        ) {
            if let stickerDetailID, let desktop {
                StickerMessageDetailSheet(
                    account: account,
                    desktop: desktop,
                    accountID: currentUserID,
                    stickerID: stickerDetailID
                )
            }
        }
    }

    private func isSameGroup(_ other: MessageItem?, before message: MessageItem) -> Bool {
        guard let other else { return false }
        return Calendar.current.isDate(
            other.conversationContentDate,
            inSameDayAs: message.conversationContentDate
        ) && other.senderId == message.senderId
            && other.breaksMessageGrouping == false
    }

    private func isSameGroup(_ other: MessageItem?, after message: MessageItem) -> Bool {
        guard let other else { return false }
        return Calendar.current.isDate(
            other.conversationContentDate,
            inSameDayAs: message.conversationContentDate
        ) && other.senderId == message.senderId
            && message.breaksMessageGrouping == false
    }

    private func copyText(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.writeObjects([text as NSString])
    }
}
