import Observation
import SwiftUI

struct ConversationPinnedMessagesView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @State private var model = ConversationPinnedMessagesModel()
    @State private var unpinAllPresented = false

    let conversationID: String

    var body: some View {
        Group {
                switch model.state {
                case .loading:
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load pinned messages",
                        systemImage: "exclamationmark.triangle",
                        description: Text(message)
                    )
                case .ready where model.messages.isEmpty:
                    ContentUnavailableView(
                        "No Pinned Messages",
                        systemImage: "pin"
                    )
                case .ready:
                    List(model.messages, id: \.messageId) { message in
                        ConversationContentMessageRow(
                            message: message,
                            imageMessages: model.imageMessages,
                            account: session.handle,
                            currentUserID: session.profile.userId
                        )
                        .contentShape(Rectangle())
                        .onTapGesture {
                            navigation.locateMessage(
                                conversationID: conversationID,
                                messageID: message.messageId
                            )
                            dismiss()
                        }
                        .contextMenu {
                            Button("Locate in Chat", systemImage: "scope") {
                                navigation.locateMessage(
                                    conversationID: conversationID,
                                    messageID: message.messageId
                                )
                                dismiss()
                            }
                            Button("Unpin", systemImage: "pin.slash") {
                                Task {
                                    await model.unpin(
                                        account: session.handle,
                                        conversationID: conversationID,
                                        messageID: message.messageId
                                    )
                                }
                            }
                        }
                    }
                    .listStyle(.inset)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Pinned Messages (\(model.messages.count))")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button("Unpin All") {
                        unpinAllPresented = true
                    }
                    .disabled(model.messages.isEmpty || model.mutating)
                }
            }
        .task(id: conversationID) {
            await model.start(
                account: session.handle,
                conversationID: conversationID
            )
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
    private(set) var messages: [SwiftMessageItem] = []
    private(set) var mutating = false
    var operationError: String?

    private var subscription: SwiftMessageSubscription?
    private var subscriptionTask: Task<Void, Never>?
    private var requestVersion = 0

    var imageMessages: [SwiftMessageItem] {
        messages.filter { $0.category.hasSuffix("_IMAGE") }
    }

    func start(
        account: SwiftAccountHandle,
        conversationID: String
    ) async {
        stop()
        state = .loading
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
