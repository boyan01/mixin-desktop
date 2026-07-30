import AppKit
import Observation
import SwiftUI

struct GroupParticipantsView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var model = GroupParticipantsModel()
    @State private var query = ""
    @FocusState private var searchFocused: Bool
    @State private var addPresented = false
    @State private var invitePresented = false
    @State private var pendingRemoval: ConversationParticipantItem?
    let conversation: ConversationListData

    var body: some View {
        VStack(spacing: 0) {
            MixinSearchField(
                text: $query,
                focus: $searchFocused,
                placeholder: "Search name or Mixin ID"
            )
            .padding(.horizontal, 16)

            if model.currentParticipant != nil {
                participantList
            }
        }
        .background(theme.primary)
        .navigationTitle("Participants")
        .toolbar {
            if model.canManage {
                ToolbarItem(placement: .primaryAction) {
                    Menu {
                        Button("Add Participants", systemImage: "person.badge.plus") {
                            addPresented = true
                        }
                        Button("Invite via Link", systemImage: "link") {
                            invitePresented = true
                        }
                    } label: {
                        Image(systemName: "plus")
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task(id: conversation.conversationId) {
            await model.start(
                account: session.handle,
                conversationID: conversation.conversationId,
                currentUserID: session.profile.userId
            )
        }
        .onDisappear {
            model.stop()
        }
        .sheet(isPresented: $addPresented) {
            AddGroupParticipantsSheet(
                model: model,
                account: session.handle
            )
        }
        .sheet(isPresented: $invitePresented) {
            GroupInviteSheet(conversation: conversation)
        }
        .confirmationDialog(
            "Remove \(pendingRemoval?.fullName ?? "participant") from this group?",
            isPresented: Binding(
                get: { pendingRemoval != nil },
                set: { if !$0 { pendingRemoval = nil } }
            )
        ) {
            Button("Remove Participant", role: .destructive) {
                guard let participant = pendingRemoval else {
                    return
                }
                pendingRemoval = nil
                Task {
                    await model.update(
                        account: session.handle,
                        action: .remove,
                        participant: participant
                    )
                }
            }
            Button("Cancel", role: .cancel) {
                pendingRemoval = nil
            }
        }
        .alert(
            "Unable to update group",
            isPresented: Binding(
                get: { model.operationError != nil },
                set: { if !$0 { model.dismissError() } }
            )
        ) {
            Button("OK") {
                model.dismissError()
            }
        } message: {
            Text(model.operationError ?? "")
        }
    }

    private var participantList: some View {
        AppListView(model.filteredParticipants(query: query), id: \.userId) { participant in
            Button {
                navigation.pushInspector(
                    .userProfile(userID: participant.userId)
                )
            } label: {
                participantRow(participant)
            }
            .buttonStyle(.plain)
            .listRowInsets(.init())
            .listRowBackground(Color.clear)
            .contextMenu {
                if participant.userId != session.profile.userId {
                    Button("Message \(participant.fullName)") {
                        Task {
                            guard let conversationID = await model.openConversation(
                                account: session.handle,
                                participant: participant
                            ) else {
                                return
                            }
                            dismiss()
                            navigation.infoPresented = false
                            navigation.selectConversation(
                                conversationID,
                                name: participant.fullName
                            )
                        }
                    }
                    if model.canChangeRole(of: participant) {
                        Divider()
                        Button(participant.role == "ADMIN"
                            ? "Dismiss as Admin"
                            : "Make Group Admin")
                        {
                            Task {
                                await model.update(
                                    account: session.handle,
                                    action: participant.role == "ADMIN"
                                        ? .dismissAdmin
                                        : .makeAdmin,
                                    participant: participant
                                )
                            }
                        }
                    }
                    if model.canRemove(participant) {
                        Divider()
                        Button("Remove from Group", role: .destructive) {
                            pendingRemoval = participant
                        }
                    }
                }
            }
        }
        .padding(.top, 8)
    }

    private func participantRow(
        _ participant: ConversationParticipantItem
    ) -> some View {
        HStack(spacing: 12) {
            UserAvatar(
                userID: participant.userId,
                name: participant.fullName,
                url: participant.avatarUrl,
                size: 50
            )

            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 4) {
                    ParticipantNameText(
                        name: participant.fullName,
                        query: query
                    )
                    ProfileIdentityBadge(
                        isVerified: participant.isVerified,
                        isBot: participant.isBot,
                        membership: participant.membership
                    )
                }
                Text(participant.identityNumber)
                    .font(.system(size: 14))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()
            if participant.role == "OWNER" || participant.role == "ADMIN" {
                Text(participant.role == "OWNER" ? "Owner" : "Admin")
                    .font(.system(size: 14))
                    .foregroundStyle(.secondary)
            }
            if model.actingUserIDs.contains(participant.userId) {
                ProgressView()
                    .controlSize(.small)
            }
        }
        .contentShape(Rectangle())
        .padding(.vertical, 10)
        .padding(.horizontal, 16)
    }
}

private struct ParticipantNameText: View {
    @Environment(\.mixinTheme) private var theme
    let name: String
    let query: String

    var body: some View {
        highlightedName
            .font(.system(size: 16))
            .lineLimit(1)
    }

    private var highlightedName: Text {
        let value = name.isEmpty ? "?" : name
        let keyword = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !keyword.isEmpty else {
            return Text(value).foregroundColor(theme.text)
        }

        var result = Text("")
        var remainder = value[...]
        while let range = remainder.range(
            of: keyword,
            options: [.caseInsensitive, .diacriticInsensitive]
        ) {
            result = result + Text(remainder[..<range.lowerBound])
                .foregroundColor(theme.text)
            result = result + Text(remainder[range])
                .foregroundColor(theme.accent)
            remainder = remainder[range.upperBound...]
        }
        return result + Text(remainder).foregroundColor(theme.text)
    }
}

private struct AddGroupParticipantsSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @Bindable var model: GroupParticipantsModel
    let account: SwiftAccountHandle
    @State private var query = ""
    @State private var selectedUserIDs = Set<String>()
    @FocusState private var queryFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                        .frame(width: 40, height: 40)
                }
                .buttonStyle(.plain)
                .foregroundStyle(theme.icon)
                .frame(maxWidth: .infinity, alignment: .leading)
                VStack(spacing: 0) {
                    Text("Add Participants")
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                    Text("\(selectedUserIDs.count) / \(model.candidates.count)")
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                }
                .frame(maxWidth: .infinity)
                Button("Next") {
                    Task {
                        if await model.add(account: account, userIDs: Array(selectedUserIDs)) {
                            dismiss()
                        }
                    }
                }
                .buttonStyle(.plain)
                .font(.system(size: 16, weight: .medium))
                .foregroundStyle(theme.accent)
                .disabled(selectedUserIDs.isEmpty || model.acting)
                .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .padding(.horizontal, 16)
            .padding(.top, 16)

            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(theme.secondaryText)
                TextField("Search", text: $query)
                    .textFieldStyle(.plain)
                    .font(.system(size: 14))
                    .foregroundStyle(theme.text)
                    .focused($queryFocused)
            }
            .padding(.horizontal, 16)
            .frame(height: 32)
            .background(theme.background, in: Capsule())
            .padding(.top, 8)
            .padding(.horizontal, 24)

            if selectedUserIDs.isEmpty {
                Spacer().frame(height: 8)
            } else {
                AppScrollView(.horizontal, showsIndicator: false) {
                    HStack(spacing: 4) {
                        ForEach(selectedUsers, id: \.userId) { user in
                            VStack(spacing: 10) {
                                ZStack(alignment: .topTrailing) {
                                    avatar(user, size: 50)
                                    Image(systemName: "xmark.circle.fill")
                                        .font(.system(size: 16))
                                        .foregroundStyle(theme.secondaryText)
                                        .background(theme.popUp, in: Circle())
                                        .offset(x: 4, y: -4)
                                }
                                Text(user.fullName)
                                    .font(.system(size: 14))
                                    .foregroundStyle(theme.text)
                                    .lineLimit(2)
                                    .multilineTextAlignment(.center)
                            }
                            .frame(width: 66)
                            .onTapGesture {
                                selectedUserIDs.remove(user.userId)
                            }
                        }
                    }
                    .padding(.horizontal, 24)
                }
                .frame(height: 120)
            }

            if model.loadingCandidates {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                AppScrollView {
                    LazyVStack(spacing: 0) {
                        candidateSection("Contacts", users: contacts)
                        candidateSection("Bots", users: bots)
                    }
                    .padding(.horizontal, 16)
                }
            }
        }
        .background(theme.popUp)
        .frame(width: 480, height: 600)
        .task {
            await model.loadCandidates(account: account)
        }
    }

    private var filtered: [UserProfileItem] { model.filteredCandidates(query: query) }
    private var contacts: [UserProfileItem] { filtered.filter { !$0.isBot } }
    private var bots: [UserProfileItem] { filtered.filter(\.isBot) }
    private var selectedUsers: [UserProfileItem] {
        model.candidates.filter { selectedUserIDs.contains($0.userId) }
    }

    @ViewBuilder
    private func candidateSection(_ title: String, users: [UserProfileItem]) -> some View {
        if !users.isEmpty {
            Text(title)
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
                .frame(maxWidth: .infinity, alignment: .leading)
                .frame(height: 42)
                .padding(.leading, 14)
            ForEach(users, id: \.userId) { user in
                Button {
                    if selectedUserIDs.remove(user.userId) == nil,
                        selectedUserIDs.count < model.remainingCapacity
                    {
                        selectedUserIDs.insert(user.userId)
                    }
                } label: {
                    HStack(spacing: 0) {
                        Circle()
                            .fill(selectedUserIDs.contains(user.userId) ? theme.accent : theme.secondaryText)
                            .frame(width: 16, height: 16)
                            .overlay {
                                if selectedUserIDs.contains(user.userId) {
                                    Image(systemName: "checkmark")
                                        .font(.system(size: 10, weight: .bold))
                                        .foregroundStyle(.white)
                                }
                            }
                            .padding(.trailing, 20)
                        avatar(user, size: 50)
                        Text(user.fullName.isEmpty ? "?" : user.fullName)
                            .font(.system(size: 16))
                            .foregroundStyle(theme.text)
                            .lineLimit(1)
                            .padding(.leading, 16)
                        ProfileIdentityBadge(
                            isVerified: user.isVerified,
                            isBot: user.isBot,
                            membership: user.membership
                        )
                        Spacer()
                    }
                    .frame(height: 70)
                    .padding(.leading, 14)
                    .padding(.trailing, 10)
                    .contentShape(RoundedRectangle(cornerRadius: 8))
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func avatar(_ user: UserProfileItem, size: CGFloat) -> some View {
        MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
            image.resizable().scaledToFill()
        } placeholder: {
            Image(systemName: "person.crop.circle.fill")
                .resizable()
                .foregroundStyle(theme.secondaryText)
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
    }
}

private struct GroupInviteSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.dismiss) private var dismiss
    @State private var model = GroupInviteModel()
    @Environment(\.mixinTheme) private var theme
    let conversation: ConversationListData

    var body: some View {
        ZStack(alignment: .top) {
            if let detail = model.detail {
                VStack(spacing: 0) {
                    Spacer().frame(height: 120)
                    ConversationAvatar(conversation: conversation, size: 90)
                    Spacer().frame(height: 16)
                    Text(detail.name)
                        .font(.system(size: 18, weight: .semibold))
                        .foregroundStyle(theme.text)
                        .lineLimit(1)
                    Spacer().frame(height: 12)
                    Text(detail.codeUrl)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                        .multilineTextAlignment(.center)
                        .lineSpacing(7)
                        .textSelection(.enabled)
                        .frame(width: 320)
                    Spacer().frame(height: 8)
                    Text("Anyone with this link can request to join the group. Reset it if the link was shared unexpectedly.")
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                        .multilineTextAlignment(.center)
                        .lineSpacing(6)
                        .frame(width: 338)
                    Spacer().frame(height: 61)
                    HStack(spacing: 0) {
                        GroupInviteActionButton(
                            label: "Share Link",
                            systemImage: "square.and.arrow.up"
                        ) {
                            ShareLink(item: detail.codeUrl) {
                                EmptyView()
                            }
                        }
                        .disabled(detail.codeUrl.isEmpty)
                        Spacer()
                        GroupInviteActionButton(
                            label: "Copy Invite",
                            systemImage: "doc.on.doc"
                        ) {
                            Button {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(
                                    detail.codeUrl,
                                    forType: .string
                                )
                            } label: {
                                EmptyView()
                            }
                            .buttonStyle(.plain)
                        }
                        .disabled(detail.codeUrl.isEmpty)
                        Spacer()
                        GroupInviteActionButton(
                            label: "Reset Link",
                            systemImage: "arrow.clockwise"
                        ) {
                            Button {
                                Task {
                                    await model.rotate(
                                        account: session.handle,
                                        conversationID: conversation.conversationId
                                    )
                                }
                            } label: {
                                EmptyView()
                            }
                            .buttonStyle(.plain)
                        }
                        .disabled(model.acting)
                    }
                    .padding(.horizontal, 72)
                }
            }
            Text("Invite to Group via Link")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(theme.text)
                .padding(.top, 30)
            Button {
                dismiss()
            } label: {
                Image(systemName: "xmark")
                    .frame(width: 40, height: 40)
            }
            .buttonStyle(.plain)
            .foregroundStyle(theme.icon)
            .padding(.top, 12)
            .padding(.trailing, 10)
            .frame(maxWidth: .infinity, alignment: .topTrailing)
        }
        .background(theme.popUp)
        .frame(width: 480, height: 600)
        .task(id: conversation.conversationId) {
            await model.load(
                account: session.handle,
                conversationID: conversation.conversationId
            )
        }
        .alert(
            "Unable to update invite link",
            isPresented: Binding(
                get: { model.error != nil },
                set: { if !$0 { model.error = nil } }
            )
        ) {
            Button("OK") {
                model.error = nil
            }
        } message: {
            Text(model.error ?? "")
        }
    }
}

private struct GroupInviteActionButton<Content: View>: View {
    @Environment(\.mixinTheme) private var theme
    let label: String
    let systemImage: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        content()
            .overlay {
                VStack(spacing: 15) {
                    Image(systemName: systemImage)
                        .font(.system(size: 24))
                        .foregroundStyle(theme.icon)
                    Text(label)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                }
                .padding(8)
                .allowsHitTesting(false)
            }
            .frame(minWidth: 72, minHeight: 63)
    }
}

@MainActor
@Observable
final class GroupParticipantsModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var participants: [ConversationParticipantItem] = []
    private(set) var candidates: [UserProfileItem] = []
    private(set) var loadingCandidates = false
    private(set) var actingUserIDs = Set<String>()
    private(set) var operationError: String?
    private(set) var conversationID = ""
    private(set) var currentUserID = ""
    private var requestVersion = 0
    private var subscription: SwiftConversationSubscription?
    private var subscriptionTask: Task<Void, Never>?

    var acting: Bool {
        !actingUserIDs.isEmpty
    }

    var currentParticipant: ConversationParticipantItem? {
        participants.first { $0.userId == currentUserID }
    }

    var canManage: Bool {
        currentParticipant?.role == "OWNER" || currentParticipant?.role == "ADMIN"
    }

    var remainingCapacity: Int {
        max(0, 1_024 - participants.count)
    }

    func start(
        account: SwiftAccountHandle,
        conversationID: String,
        currentUserID: String
    ) async {
        stop()
        requestVersion += 1
        let version = requestVersion
        self.conversationID = conversationID
        self.currentUserID = currentUserID
        let subscription = account.conversationChanges()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let event = await subscription.next()
            {
                guard let self,
                      event.reloadAll || event.conversationIds.contains(conversationID)
                else {
                    continue
                }
                await reload(account: account)
            }
        }
        state = .loading
        do {
            let loaded = try await account.conversationParticipants(
                conversationId: conversationID
            )
            guard version == requestVersion else {
                return
            }
            participants = loaded
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func filteredParticipants(query: String) -> [ConversationParticipantItem] {
        let keyword = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !keyword.isEmpty else {
            return participants
        }
        return participants.filter {
            $0.fullName.lowercased().contains(keyword)
                || $0.identityNumber.contains(keyword)
        }
    }

    func filteredCandidates(query: String) -> [UserProfileItem] {
        let keyword = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !keyword.isEmpty else {
            return candidates
        }
        return candidates.filter {
            $0.fullName.lowercased().contains(keyword)
                || $0.identityNumber.contains(keyword)
        }
    }

    func loadCandidates(account: SwiftAccountHandle) async {
        guard canManage else {
            operationError = "Only a group owner or admin can add participants."
            return
        }
        loadingCandidates = true
        defer { loadingCandidates = false }
        do {
            let existing = Set(participants.map(\.userId))
            candidates = try await account.selectableUsers().filter {
                !existing.contains($0.userId)
            }
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func add(account: SwiftAccountHandle, userIDs: [String]) async -> Bool {
        guard canManage else {
            operationError = "Only a group owner or admin can add participants."
            return false
        }
        guard !userIDs.isEmpty, userIDs.count <= remainingCapacity else {
            operationError = "This group can contain at most 1,024 participants."
            return false
        }
        actingUserIDs.formUnion(userIDs)
        defer { actingUserIDs.subtract(userIDs) }
        do {
            try await account.updateParticipants(
                conversationId: conversationID,
                action: .add,
                userIds: userIDs
            )
            await reload(account: account)
            return true
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func canChangeRole(of participant: ConversationParticipantItem) -> Bool {
        currentParticipant?.role == "OWNER"
            && participant.userId != currentUserID
            && participant.role != "OWNER"
    }

    func canRemove(_ participant: ConversationParticipantItem) -> Bool {
        guard participant.userId != currentUserID else {
            return false
        }
        if currentParticipant?.role == "OWNER" {
            return participant.role != "OWNER"
        }
        return currentParticipant?.role == "ADMIN" && participant.role == nil
    }

    func update(
        account: SwiftAccountHandle,
        action: ParticipantAction,
        participant: ConversationParticipantItem
    ) async {
        switch action {
        case .add:
            operationError = "Use Add Participants to select group members."
            return
        case .remove:
            guard canRemove(participant) else {
                operationError = "You do not have permission to remove this participant."
                return
            }
        case .makeAdmin, .dismissAdmin:
            guard canChangeRole(of: participant) else {
                operationError = "Only the group owner can change administrator roles."
                return
            }
        }

        actingUserIDs.insert(participant.userId)
        defer { actingUserIDs.remove(participant.userId) }
        do {
            try await account.updateParticipants(
                conversationId: conversationID,
                action: action,
                userIds: [participant.userId]
            )
            await reload(account: account)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func openConversation(
        account: SwiftAccountHandle,
        participant: ConversationParticipantItem
    ) async -> String? {
        do {
            return try await account.openUserConversation(userId: participant.userId)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return nil
        }
    }

    func dismissError() {
        operationError = nil
    }

    func stop() {
        subscription?.cancel()
        subscription = nil
        subscriptionTask?.cancel()
        subscriptionTask = nil
    }

    private func reload(account: SwiftAccountHandle) async {
        do {
            participants = try await account.conversationParticipants(
                conversationId: conversationID
            )
            let existing = Set(participants.map(\.userId))
            candidates.removeAll { existing.contains($0.userId) }
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }
}

@MainActor
@Observable
private final class GroupInviteModel {
    private(set) var detail: ConversationDetailItem?
    private(set) var loading = false
    private(set) var acting = false
    var error: String?

    func load(account: SwiftAccountHandle, conversationID: String) async {
        loading = true
        defer { loading = false }
        do {
            detail = try await account.conversationDetail(
                conversationId: conversationID
            )
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    func rotate(account: SwiftAccountHandle, conversationID: String) async {
        guard !acting else {
            return
        }
        acting = true
        defer { acting = false }
        do {
            try await account.rotateGroupInvite(conversationId: conversationID)
            detail = try await account.conversationDetail(
                conversationId: conversationID
            )
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }
}
