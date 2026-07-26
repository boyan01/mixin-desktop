import Observation
import SwiftUI

struct CircleNameSheet: View {
    @State private var name: String
    @State private var saving = false
    @State private var failurePresented = false
    let circle: SwiftCircleItem
    let onCancel: () -> Void
    let onSave: (String) async -> Bool

    init(
        circle: SwiftCircleItem,
        onCancel: @escaping () -> Void,
        onSave: @escaping (String) async -> Bool
    ) {
        self.circle = circle
        self.onCancel = onCancel
        self.onSave = onSave
        _name = State(initialValue: circle.name)
    }

    var body: some View {
        NavigationStack {
            Form {
                TextField("Circle name", text: $name)
                    .onSubmit(save)
            }
            .formStyle(.grouped)
            .navigationTitle("Rename Circle")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onCancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save", action: save)
                        .disabled(!canSave || saving)
                }
            }
        }
        .frame(width: 420, height: 180)
        .alert("Unable to Rename Circle", isPresented: $failurePresented) {
            Button("OK", role: .cancel) {}
        }
    }

    private var trimmedName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var canSave: Bool {
        !trimmedName.isEmpty
            && trimmedName.count <= 64
            && trimmedName != circle.name
    }

    private func save() {
        guard canSave, !saving else {
            return
        }
        saving = true
        Task {
            if await onSave(trimmedName) {
                onCancel()
            } else {
                saving = false
                failurePresented = true
            }
        }
    }
}

struct CircleConversationsSheet: View {
    @Environment(AccountSession.self) private var session
    @State private var model: CircleConversationsModel
    @State private var query = ""
    let onDismiss: () -> Void

    init(circle: SwiftCircleItem, onDismiss: @escaping () -> Void) {
        _model = State(initialValue: CircleConversationsModel(circle: circle))
        self.onDismiss = onDismiss
    }

    var body: some View {
        NavigationStack {
            Group {
                switch model.state {
                case .loading:
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case let .failed(message):
                    VStack(spacing: 12) {
                        ContentUnavailableView(
                            "Unable to load conversations",
                            systemImage: "exclamationmark.triangle",
                            description: Text(message)
                        )
                        Button("Retry") {
                            Task {
                                await model.load(account: session.handle)
                            }
                        }
                    }
                case .ready:
                    List(model.filteredConversations(query: query), id: \.conversationId) {
                        conversation in
                        Button {
                            model.toggle(conversation.conversationId)
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: conversation.iconUrl)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: conversation.category == "GROUP"
                                        ? "person.3.fill"
                                        : "person.crop.circle.fill")
                                        .resizable()
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 38, height: 38)
                                .clipShape(Circle())

                                VStack(alignment: .leading, spacing: 3) {
                                    Text(conversation.name)
                                        .foregroundStyle(.primary)
                                    if !conversation.preview.isEmpty {
                                        Text(conversation.preview)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                            .lineLimit(1)
                                    }
                                }
                                Spacer()
                                if model.selectedIDs.contains(conversation.conversationId) {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundStyle(Color.accentColor)
                                }
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    }
                    .listStyle(.inset)
                }
            }
            .navigationTitle(model.circle.name)
            .searchable(text: $query, prompt: "Search conversations")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onDismiss)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") {
                        Task {
                            if await model.save(account: session.handle) {
                                onDismiss()
                            }
                        }
                    }
                    .disabled(model.saving)
                }
            }
            .overlay(alignment: .bottom) {
                if model.saving {
                    ProgressView()
                        .padding(12)
                        .background(.regularMaterial, in: Capsule())
                        .padding()
                }
            }
        }
        .frame(minWidth: 520, minHeight: 600)
        .task {
            await model.load(account: session.handle)
        }
    }
}

@MainActor
@Observable
final class CircleConversationsModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    let circle: SwiftCircleItem
    private(set) var state: State = .loading
    private(set) var conversations: [SwiftConversationListItem] = []
    private(set) var selectedIDs = Set<String>()
    private(set) var saving = false
    private var initialIDs = Set<String>()

    init(circle: SwiftCircleItem) {
        self.circle = circle
    }

    func load(account: SwiftAccountHandle) async {
        state = .loading
        do {
            async let all = loadConversations(
                account: account,
                category: "chats",
                circleID: nil
            )
            async let selected = loadConversations(
                account: account,
                category: "circle",
                circleID: circle.circleId
            )
            conversations = try await all
            initialIDs = try await Set(selected.map(\.conversationId))
            selectedIDs = initialIDs
            state = .ready
        } catch {
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func toggle(_ conversationID: String) {
        if selectedIDs.remove(conversationID) == nil {
            selectedIDs.insert(conversationID)
        }
    }

    func filteredConversations(query: String) -> [SwiftConversationListItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else {
            return conversations
        }
        return conversations.filter {
            $0.name.lowercased().contains(query)
                || $0.preview.lowercased().contains(query)
        }
    }

    func save(account: SwiftAccountHandle) async -> Bool {
        guard !saving else {
            return false
        }
        saving = true
        defer { saving = false }
        do {
            let changedIDs = selectedIDs.symmetricDifference(initialIDs)
            for conversation in conversations
                where changedIDs.contains(conversation.conversationId)
            {
                try await account.editCircleConversation(
                    circleId: circle.circleId,
                    conversationId: conversation.conversationId,
                    ownerId: conversation.ownerId,
                    category: conversation.category,
                    add: selectedIDs.contains(conversation.conversationId)
                )
            }
            initialIDs = selectedIDs
            return true
        } catch {
            state = .failed(MixinErrorPresenter.message(for: error))
            return false
        }
    }

    private func loadConversations(
        account: SwiftAccountHandle,
        category: String,
        circleID: String?
    ) async throws -> [SwiftConversationListItem] {
        var offset: Int64 = 0
        var result: [SwiftConversationListItem] = []
        while true {
            let page = try await account.conversations(
                category: category,
                circleId: circleID,
                keyword: "",
                unseenOnly: false,
                limit: 200,
                offset: offset
            )
            result.append(contentsOf: page)
            if page.count < 200 {
                return result
            }
            offset += Int64(page.count)
        }
    }
}
