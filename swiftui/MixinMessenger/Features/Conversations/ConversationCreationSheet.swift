import Observation
import SwiftUI

struct ConversationCreationSheet: View {
    @Environment(AccountSession.self) private var session
    @State private var model = ConversationCreationModel()
    @State private var query = ""
    @State private var name = ""
    let kind: ConversationCreationKind
    let onComplete: (String?, String?) -> Void

    var body: some View {
        NavigationStack {
            Group {
                switch model.state {
                case .loading:
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load choices",
                        systemImage: "exclamationmark.triangle",
                        description: Text(message)
                    )
                case .ready:
                    VStack(spacing: 0) {
                        if kind != .conversation {
                            TextField(namePlaceholder, text: $name)
                                .textFieldStyle(.roundedBorder)
                                .padding()
                        }
                        selectionList
                    }
                }
            }
            .navigationTitle(title)
            .searchable(text: $query, prompt: "Search")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        onComplete(nil, nil)
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(actionTitle) {
                        Task {
                            if let result = await model.create(
                                account: session.handle,
                                kind: kind,
                                name: name
                            ) {
                                onComplete(result.id, result.name)
                            } else if kind == .circle, model.state.isReady {
                                onComplete(nil, nil)
                            }
                        }
                    }
                    .disabled(!canCreate || model.creating)
                }
            }
            .overlay(alignment: .bottom) {
                if model.creating {
                    ProgressView()
                        .padding(12)
                        .background(.regularMaterial, in: Capsule())
                        .padding()
                }
            }
        }
        .frame(minWidth: 480, minHeight: 580)
        .task {
            await model.load(account: session.handle, kind: kind)
        }
        .task(id: query) {
            guard kind != .circle else {
                return
            }
            try? await Task.sleep(for: .milliseconds(250))
            guard !Task.isCancelled else {
                return
            }
            await model.searchRemote(
                account: session.handle,
                query: query
            )
        }
    }

    @ViewBuilder
    private var selectionList: some View {
        let users = model.filteredUsers(query: query)
        let conversations = model.filteredConversations(query: query)
        List {
            if kind == .circle {
                ForEach(conversations, id: \.conversationId) { conversation in
                    choiceRow(
                        id: conversation.conversationId,
                        title: conversation.name,
                        subtitle: conversation.preview,
                        avatarURL: conversation.iconUrl
                    )
                }
            } else {
                ForEach(users, id: \.userId) { user in
                    choiceRow(
                        id: user.userId,
                        title: user.fullName,
                        subtitle: user.identityNumber,
                        avatarURL: user.avatarUrl
                    )
                }
            }
        }
        .listStyle(.inset)
    }

    private func choiceRow(
        id: String,
        title: String,
        subtitle: String,
        avatarURL: String
    ) -> some View {
        Button {
            model.toggle(id, single: kind == .conversation)
            if kind == .conversation {
                Task {
                    if let result = await model.create(
                        account: session.handle,
                        kind: kind,
                        name: name
                    ) {
                        onComplete(result.id, result.name)
                    }
                }
            }
        } label: {
            HStack(spacing: 12) {
                MixinRemoteImage(url: URL(string: avatarURL)) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Image(systemName: "person.crop.circle.fill")
                        .resizable()
                        .foregroundStyle(.secondary)
                }
                .frame(width: 42, height: 42)
                .clipShape(Circle())

                VStack(alignment: .leading, spacing: 3) {
                    Text(title)
                        .foregroundStyle(.primary)
                    if !subtitle.isEmpty {
                        Text(subtitle)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                }
                Spacer()
                if model.selectedIDs.contains(id) {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundStyle(Color.accentColor)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var canCreate: Bool {
        switch kind {
        case .conversation:
            model.selectedIDs.count == 1
        case .group:
            !model.selectedIDs.isEmpty
                && !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        case .circle:
            !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        }
    }

    private var title: String {
        switch kind {
        case .conversation:
            "New Conversation"
        case .group:
            "New Group"
        case .circle:
            "New Circle"
        }
    }

    private var namePlaceholder: String {
        kind == .group ? "Group name" : "Circle name"
    }

    private var actionTitle: String {
        kind == .conversation ? "Open" : "Create"
    }
}

@MainActor
@Observable
final class ConversationCreationModel {
    enum State {
        case loading
        case ready
        case failed(String)

        var isReady: Bool {
            if case .ready = self {
                return true
            }
            return false
        }
    }

    struct Result {
        let id: String?
        let name: String?
    }

    private(set) var state: State = .loading
    private(set) var users: [SwiftUserItem] = []
    private(set) var conversations: [SwiftConversationListItem] = []
    private(set) var selectedIDs = Set<String>()
    private(set) var creating = false
    private var searchVersion = 0

    func load(
        account: SwiftAccountHandle,
        kind: ConversationCreationKind
    ) async {
        state = .loading
        do {
            switch kind {
            case .conversation, .group:
                users = try await account.selectableUsers()
            case .circle:
                var offset: Int64 = 0
                var result: [SwiftConversationListItem] = []
                while true {
                    let page = try await account.conversations(
                        category: "chats",
                        circleId: nil,
                        keyword: "",
                        unseenOnly: false,
                        limit: 200,
                        offset: offset
                    )
                    result.append(contentsOf: page)
                    if page.count < 200 {
                        break
                    }
                    offset += Int64(page.count)
                }
                conversations = result
            }
            state = .ready
        } catch {
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func toggle(_ id: String, single: Bool) {
        if single {
            selectedIDs = [id]
        } else if selectedIDs.remove(id) == nil {
            selectedIDs.insert(id)
        }
    }

    func filteredUsers(query: String) -> [SwiftUserItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else {
            return users
        }
        return users.filter {
            $0.fullName.lowercased().contains(query)
                || $0.identityNumber.contains(query)
        }
    }

    func searchRemote(account: SwiftAccountHandle, query: String) async {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            searchVersion += 1
            return
        }
        searchVersion += 1
        let version = searchVersion
        do {
            let user = try await account.searchUser(query: query)
            guard version == searchVersion, !Task.isCancelled else {
                return
            }
            if let index = users.firstIndex(where: { $0.userId == user.userId }) {
                users[index] = user
            } else {
                users.insert(user, at: 0)
            }
        } catch {
            // A remote miss leaves the useful local result set visible.
        }
    }

    func filteredConversations(query: String) -> [SwiftConversationListItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else {
            return conversations
        }
        return conversations.filter {
            $0.name.lowercased().contains(query)
        }
    }

    func create(
        account: SwiftAccountHandle,
        kind: ConversationCreationKind,
        name: String
    ) async -> Result? {
        guard !creating else {
            return nil
        }
        creating = true
        defer { creating = false }
        do {
            switch kind {
            case .conversation:
                guard let userID = selectedIDs.first,
                      let user = users.first(where: { $0.userId == userID })
                else {
                    return nil
                }
                let id = try await account.openUserConversation(userId: userID)
                return Result(id: id, name: user.fullName)
            case .group:
                let trimmedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
                let id = try await account.createGroup(
                    name: trimmedName,
                    userIds: Array(selectedIDs)
                )
                return Result(id: id, name: trimmedName)
            case .circle:
                let circle = try await account.createCircle(
                    name: name.trimmingCharacters(in: .whitespacesAndNewlines)
                )
                for conversation in conversations
                    where selectedIDs.contains(conversation.conversationId)
                {
                    try await account.editCircleConversation(
                        circleId: circle.circleId,
                        conversationId: conversation.conversationId,
                        ownerId: conversation.ownerId,
                        category: conversation.category,
                        add: true
                    )
                }
                return Result(id: nil, name: nil)
            }
        } catch {
            state = .failed(MixinErrorPresenter.message(for: error))
            return nil
        }
    }
}
