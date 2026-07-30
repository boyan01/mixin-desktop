import Observation
import SwiftUI

struct ConversationCreationSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationCreationModel()
    @State private var query = ""
    @State private var name = ""
    @State private var stage: Stage
    let kind: ConversationCreationKind
    let onComplete: (String?, String?) -> Void

    init(
        kind: ConversationCreationKind,
        onComplete: @escaping (String?, String?) -> Void
    ) {
        self.kind = kind
        self.onComplete = onComplete
        _stage = State(initialValue: kind == .circle ? .circleName : .selection)
    }

    var body: some View {
        Group {
            switch stage {
            case .selection:
                VStack(spacing: 0) {
                    creationHeader
                    searchField
                    selectedPreview
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
                            selectionList
                        }
                    }
                }
                .frame(minWidth: 480, minHeight: 600)
            case .groupConfirmation:
                groupConfirmation
            case .circleName:
                circleNameEntry
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
        .task(id: stage) {
            guard stage == .selection else {
                return
            }
            await model.load(account: session.handle, kind: kind)
        }
    }

    private var creationHeader: some View {
        HStack {
            Button { onComplete(nil, nil) } label: {
                Image(systemName: "xmark")
                    .font(.system(size: 20))
                    .frame(width: 40, height: 40)
            }
            .buttonStyle(.plain)
            Spacer()
            VStack(spacing: 0) {
                Text(title).font(.system(size: 16)).foregroundStyle(theme.text)
                if kind != .conversation {
                    Text("\(model.selectedIDs.count) / \(selectionTotal)")
                        .font(.system(size: 12)).foregroundStyle(theme.secondaryText)
                }
            }
            Spacer()
            if kind == .conversation {
                Color.clear.frame(width: 40, height: 40)
            } else {
                Button(actionTitle) {
                    if kind == .group {
                        stage = .groupConfirmation
                    } else {
                        Task {
                            if await model.create(account: session.handle, kind: kind, name: name) != nil,
                               model.state.isReady
                            {
                                onComplete(nil, nil)
                            }
                        }
                    }
                }
                .font(.system(size: 14))
                .foregroundStyle(theme.accent)
                .frame(width: 40, height: 40)
                .disabled(!canAdvanceSelection || model.creating)
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 16)
    }

    private var searchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass").font(.system(size: 16)).foregroundStyle(theme.secondaryText)
            TextField("Search", text: $query)
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .onChange(of: query) {
                    if query.count > 200 { query = String(query.prefix(200)) }
                }
        }
        .padding(.horizontal, 16)
        .frame(height: 32)
        .background(theme.background, in: Capsule())
        .padding(.top, 8)
        .padding(.horizontal, 24)
    }

    @ViewBuilder
    private var selectedPreview: some View {
        if kind != .conversation, !selectedChoices.isEmpty {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 4) {
                    ForEach(selectedChoices) { choice in
                        VStack(spacing: 0) {
                            Spacer().frame(height: 20)
                            ZStack(alignment: .topTrailing) {
                                MixinRemoteImage(url: URL(string: choice.avatarURL)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    AvatarPlaceholder(userID: choice.avatarUserID, name: choice.name)
                                }
                                .frame(width: 50, height: 50)
                                .clipShape(Circle())

                                Button {
                                    model.toggle(choice.id, single: false)
                                } label: {
                                    Circle()
                                        .fill(theme.popUp)
                                        .frame(width: 22, height: 22)
                                        .overlay {
                                            Circle()
                                                .fill(theme.divider)
                                                .frame(width: 16, height: 16)
                                                .overlay {
                                                    Image(systemName: "xmark")
                                                        .font(.system(size: 8, weight: .bold))
                                                        .foregroundStyle(.white.opacity(0.9))
                                                }
                                        }
                                }
                                .buttonStyle(.plain)
                            }
                            Spacer().frame(height: 10)
                            Text(choice.name)
                                .font(.system(size: 14))
                                .foregroundStyle(theme.text)
                                .multilineTextAlignment(.center)
                                .lineLimit(2)
                        }
                        .frame(width: 66, height: 120, alignment: .top)
                    }
                }
                .padding(.horizontal, 24)
            }
            .frame(height: 120)
        } else {
            Color.clear.frame(height: 8)
        }
    }

    private var groupConfirmation: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Groups")
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
            Spacer().frame(height: 24)
            VStack(spacing: 0) {
                GroupAvatarPuzzle(avatars: groupAvatars)
                    .frame(width: 60, height: 60)
                    .clipShape(Circle())
                Spacer().frame(height: 8)
                Text("\(model.selectedIDs.count + 1) participants")
                    .font(.system(size: 14))
                    .foregroundStyle(theme.secondaryText)
            }
            .frame(maxWidth: .infinity)
            Spacer().frame(height: 48)
            TextField("Group name", text: $name)
                .textFieldStyle(.roundedBorder)
                .onChange(of: name) {
                    if name.count > 40 { name = String(name.prefix(40)) }
                }
            Spacer().frame(height: 30)
            HStack(spacing: 4) {
                Spacer()
                Button("Cancel") { onComplete(nil, nil) }
                    .buttonStyle(.plain)
                    .foregroundStyle(theme.accent)
                    .padding(.vertical, 8)
                    .padding(.horizontal, 16)
                Button("Create") {
                    Task {
                        if let result = await model.create(account: session.handle, kind: .group, name: name) {
                            onComplete(result.id, result.name)
                        }
                    }
                }
                .buttonStyle(.borderedProminent)
                .tint(theme.accent)
                .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || model.creating)
            }
        }
        .padding(30)
        .frame(minWidth: 400, minHeight: 210)
    }

    private var circleNameEntry: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Circles")
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
            Spacer().frame(height: 48)
            TextField("Circle name", text: $name)
                .textFieldStyle(.roundedBorder)
                .onChange(of: name) {
                    if name.count > 64 { name = String(name.prefix(64)) }
                }
            Spacer().frame(height: 30)
            HStack(spacing: 4) {
                Spacer()
                Button("Cancel") { onComplete(nil, nil) }
                    .buttonStyle(.plain)
                    .foregroundStyle(theme.accent)
                    .padding(.vertical, 8)
                    .padding(.horizontal, 16)
                Button("Create") { stage = .selection }
                    .buttonStyle(.borderedProminent)
                    .tint(theme.accent)
                    .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(30)
        .frame(minWidth: 400, minHeight: 210)
    }

    @ViewBuilder
    private var selectionList: some View {
        let contactChoices = model.filteredContactChoices(query: query)
        let conversations = model.filteredConversations(query: query)
        AppListView {
            if kind == .circle {
                conversationSections(conversations)
            } else {
                let contacts = contactChoices.filter { !$0.isBot }
                let bots = contactChoices.filter(\.isBot)
                if !contacts.isEmpty {
                    selectionSectionHeader("Contacts")
                    groupRows(contacts)
                }
                if !bots.isEmpty {
                    selectionSectionHeader("Bots")
                    groupRows(bots)
                }
            }
        }
        .listStyle(.plain)
        .padding(.horizontal, 16)
    }

    @ViewBuilder
    private func conversationSections(_ conversations: [ConversationListData]) -> some View {
        let recent = conversations.filter { $0.status != -1 }
        let contacts = conversations.filter { $0.status == -1 && !$0.isBot }
        let bots = conversations.filter { $0.status == -1 && $0.isBot }
        if !recent.isEmpty {
            selectionSectionHeader("Recent Chats")
            conversationRows(recent)
        }
        if !contacts.isEmpty {
            selectionSectionHeader("Contacts")
            conversationRows(contacts)
        }
        if !bots.isEmpty {
            selectionSectionHeader("Bots")
            conversationRows(bots)
        }
    }

    private func groupRows(_ choices: [ConversationCreationChoice]) -> some View {
        ForEach(choices) { choice in
            choiceRow(
                id: choice.id,
                title: choice.name,
                subtitle: choice.identityNumber,
                avatarURL: choice.avatarURL,
                avatarUserID: choice.ownerID,
                conversation: nil,
                isVerified: choice.isVerified,
                isBot: choice.isBot,
                membership: choice.membership
            )
        }
    }

    private func conversationRows(_ conversations: [ConversationListData]) -> some View {
        ForEach(conversations, id: \.conversationId) { conversation in
            choiceRow(
                id: conversation.conversationId,
                title: conversation.name,
                subtitle: conversation.preview,
                avatarURL: conversation.avatarUrl,
                avatarUserID: conversation.ownerId,
                conversation: conversation,
                isVerified: conversation.isVerified,
                isBot: conversation.isBot,
                membership: conversation.membership
            )
        }
    }

    private func selectionSectionHeader(_ title: String) -> some View {
        Text(title)
            .font(.system(size: 16))
            .foregroundStyle(theme.text)
            .frame(maxWidth: .infinity, minHeight: 42, alignment: .leading)
            .padding(.leading, 14)
            .background(theme.popUp)
            .listRowInsets(EdgeInsets())
    }

    private func choiceRow(
        id: String,
        title: String,
        subtitle: String,
        avatarURL: String,
        avatarUserID: String,
        conversation: ConversationListData?,
        isVerified: Bool,
        isBot: Bool,
        membership: String?
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
            HStack(spacing: 0) {
                if kind != .conversation {
                    Circle()
                        .fill(
                            model.selectedIDs.contains(id)
                                ? theme.accent
                                : theme.secondaryText
                        )
                        .overlay {
                            if model.selectedIDs.contains(id) {
                                Image(systemName: "checkmark")
                                    .font(.system(size: 10, weight: .bold))
                                    .foregroundStyle(.white)
                            }
                        }
                        .frame(width: 16, height: 16)
                        .padding(.trailing, 20)
                }

                Group {
                    if let conversation {
                        ConversationAvatar(conversation: conversation, size: 50)
                    } else {
                        UserAvatar(
                            userID: avatarUserID,
                            name: title,
                            url: avatarURL,
                            size: 50
                        )
                    }
                }
                .frame(width: 50, height: 50)
                .clipShape(Circle())

                Spacer()
                    .frame(width: 16)

                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: 0) {
                        Text(title)
                            .font(.system(size: 16))
                            .foregroundStyle(theme.text)
                            .lineLimit(1)
                        ProfileIdentityBadge(
                            isVerified: isVerified,
                            isBot: isBot,
                            membership: membership
                        )
                        .padding(.horizontal, 4)
                    }
                }
                Spacer()
            }
            .contentShape(Rectangle())
            .frame(height: 70)
            .padding(.leading, 14)
            .padding(.trailing, 10)
        }
        .buttonStyle(MixinRowButtonStyle(selected: false))
    }

    private var canAdvanceSelection: Bool {
        switch kind {
        case .conversation:
            model.selectedIDs.count == 1
        case .group:
            !model.selectedIDs.isEmpty
        case .circle:
            true
        }
    }

    private var selectionTotal: Int {
        switch kind {
        case .conversation, .group:
            model.contactChoices.count
        case .circle:
            model.conversations.count
        }
    }

    private var selectedChoices: [SelectionPreview] {
        model.selectedIDOrder.compactMap { id in
            if kind == .circle {
                guard let conversation = model.conversations.first(where: { $0.conversationId == id }) else {
                    return nil
                }
                return SelectionPreview(
                    id: conversation.conversationId,
                    name: conversation.name,
                    avatarURL: conversation.avatarUrl,
                    avatarUserID: conversation.ownerId
                )
            }
            else {
                guard let choice = model.contactChoices.first(where: { $0.id == id }) else {
                    return nil
                }
                return SelectionPreview(
                    id: choice.id,
                    name: choice.name,
                    avatarURL: choice.avatarURL,
                    avatarUserID: choice.ownerID
                )
            }
        }
    }

    private var groupAvatars: [GroupAvatar] {
        ([
            GroupAvatar(
                userId: session.profile.userId,
                name: session.profile.fullName,
                avatarUrl: session.profile.avatarUrl
            ),
        ] + selectedChoices.map {
            GroupAvatar(userId: $0.avatarUserID, name: $0.name, avatarUrl: $0.avatarURL)
        }).prefix(4).map { $0 }
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

    private var actionTitle: String {
        "Next"
    }

    private struct SelectionPreview: Identifiable {
        let id: String
        let name: String
        let avatarURL: String
        let avatarUserID: String
    }

    private enum Stage: Equatable {
        case selection
        case groupConfirmation
        case circleName
    }
}

fileprivate struct ConversationCreationChoice: Identifiable {
    let id: String
    let ownerID: String
    let conversationID: String?
    let name: String
    let avatarURL: String
    let identityNumber: String
    let isVerified: Bool
    let isBot: Bool
    let membership: String?
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
    fileprivate var contactChoices: [ConversationCreationChoice] = []
    private(set) var conversations: [ConversationListData] = []
    private(set) var selectedIDs = Set<String>()
    private(set) var selectedIDOrder: [String] = []
    private(set) var creating = false

    func load(
        account: SwiftAccountHandle,
        kind: ConversationCreationKind
    ) async {
        state = .loading
        do {
            switch kind {
            case .conversation, .group:
                let contacts = try await loadConversations(
                    account: account,
                    category: "contacts"
                )
                let bots = try await loadConversations(
                    account: account,
                    category: "bots"
                )
                let existingOwnerIDs = Set((contacts + bots).map(\.ownerId))
                contactChoices = (contacts + bots).map {
                    ConversationCreationChoice(
                        id: "conversation:\($0.conversationId)",
                        ownerID: $0.ownerId,
                        conversationID: $0.conversationId,
                        name: $0.name,
                        avatarURL: $0.avatarUrl,
                        identityNumber: $0.identityNumber,
                        isVerified: $0.isVerified,
                        isBot: $0.isBot,
                        membership: $0.membership
                    )
                }
                contactChoices += try await account.selectableUsers()
                    .filter { !existingOwnerIDs.contains($0.userId) }
                    .map {
                        ConversationCreationChoice(
                            id: "user:\($0.userId)",
                            ownerID: $0.userId,
                            conversationID: nil,
                            name: $0.fullName,
                            avatarURL: $0.avatarUrl,
                            identityNumber: $0.identityNumber,
                            isVerified: $0.isVerified,
                            isBot: $0.isBot,
                            membership: $0.membership
                        )
                    }
            case .circle:
                conversations = try await loadConversations(account: account, category: "chats")
            }
            state = .ready
        } catch {
            AppLogger.error(
                "ConversationCreation load failed: kind=\(String(describing: kind))",
                error: error
            )
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func toggle(_ id: String, single: Bool) {
        if single {
            selectedIDs = [id]
            selectedIDOrder = [id]
        } else if selectedIDs.remove(id) == nil {
            selectedIDs.insert(id)
            selectedIDOrder.append(id)
        } else {
            selectedIDOrder.removeAll { $0 == id }
        }
    }

    fileprivate func filteredContactChoices(query: String) -> [ConversationCreationChoice] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else {
            return contactChoices
        }
        return contactChoices.filter {
            $0.name.lowercased().contains(query)
                || $0.identityNumber.contains(query)
        }
    }

    func filteredConversations(query: String) -> [ConversationListData] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else {
            return conversations
        }
        return conversations.filter {
            $0.name.lowercased().contains(query)
                || $0.identityNumber.contains(query)
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
                guard let selectedID = selectedIDs.first,
                      let choice = contactChoices.first(where: { $0.id == selectedID })
                else {
                    AppLogger.error(
                        "ConversationCreation create failed: selected user unavailable selected_ids=\(selectedIDs)"
                    )
                    return nil
                }
                if let conversationID = choice.conversationID {
                    return Result(id: conversationID, name: choice.name)
                }
                let id = try await account.openUserConversation(userId: choice.ownerID)
                return Result(id: id, name: choice.name)
            case .group:
                let trimmedName = name.trimmingCharacters(in: .whitespacesAndNewlines)
                let userIDs = selectedIDOrder.compactMap { id in
                    contactChoices.first(where: { $0.id == id })?.ownerID
                }
                guard userIDs.count == selectedIDs.count else {
                    AppLogger.error("ConversationCreation create failed: selected group members unavailable")
                    return nil
                }
                let id = try await account.createGroup(
                    name: trimmedName,
                    userIds: userIDs
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
            AppLogger.error(
                "ConversationCreation create failed: kind=\(String(describing: kind)) selected_count=\(selectedIDs.count)",
                error: error
            )
            state = .failed(MixinErrorPresenter.message(for: error))
            return nil
        }
    }

    private func loadConversations(
        account: SwiftAccountHandle,
        category: String
    ) async throws -> [ConversationListData] {
        var offset: Int64 = 0
        var result: [ConversationListData] = []
        while true {
            let page = try await account.conversations(
                category: category,
                circleId: nil,
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
