import AppKit
import Observation
import SwiftUI

struct GroupParticipantsView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @State private var model = GroupParticipantsModel()
    @State private var query = ""
    @State private var addPresented = false
    @State private var invitePresented = false
    @State private var pendingRemoval: SwiftConversationParticipantItem?
    let conversation: SwiftConversationListItem

    var body: some View {
        Group {
                switch model.state {
                case .loading:
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load participants",
                        systemImage: "person.3.sequence",
                        description: Text(message)
                    )
                case .ready:
                    if model.currentParticipant == nil {
                        ContentUnavailableView(
                            "You are no longer a member",
                            systemImage: "person.3"
                        )
                    } else {
                        participantList
                    }
                }
            }
            .navigationTitle("Participants")
            .searchable(text: $query, prompt: "Search name or Mixin ID")
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
        List(model.filteredParticipants(query: query), id: \.userId) { participant in
            Button {
                navigation.pushInspector(
                    .userProfile(userID: participant.userId)
                )
            } label: {
                participantRow(participant)
            }
            .buttonStyle(.plain)
            .contextMenu {
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
        .overlay {
            if model.participants.isEmpty {
                ContentUnavailableView(
                    "No participants",
                    systemImage: "person.3"
                )
            } else if model.filteredParticipants(query: query).isEmpty {
                ContentUnavailableView.search(text: query)
            }
        }
    }

    private func participantRow(
        _ participant: SwiftConversationParticipantItem
    ) -> some View {
        HStack(spacing: 12) {
            MixinRemoteImage(url: URL(string: participant.avatarUrl)) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Image(systemName: "person.crop.circle.fill")
                    .resizable()
                    .foregroundStyle(.secondary)
            }
            .frame(width: 44, height: 44)
            .clipShape(Circle())

            VStack(alignment: .leading, spacing: 3) {
                Text(participant.fullName.isEmpty ? "Unknown" : participant.fullName)
                    .lineLimit(1)
                Text(participant.identityNumber)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()
            if let role = participant.role {
                Text(role.capitalized)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if model.actingUserIDs.contains(participant.userId) {
                ProgressView()
                    .controlSize(.small)
            }
        }
        .contentShape(Rectangle())
        .padding(.vertical, 4)
    }
}

private struct AddGroupParticipantsSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Bindable var model: GroupParticipantsModel
    let account: SwiftAccountHandle
    @State private var query = ""
    @State private var selectedUserIDs = Set<String>()

    var body: some View {
        NavigationStack {
            Group {
                if model.loadingCandidates {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    List(model.filteredCandidates(query: query), id: \.userId) { user in
                        Button {
                            if selectedUserIDs.remove(user.userId) == nil {
                                selectedUserIDs.insert(user.userId)
                            }
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: "person.crop.circle.fill")
                                        .resizable()
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 40, height: 40)
                                .clipShape(Circle())
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(user.fullName)
                                    Text(user.identityNumber)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Spacer()
                                if selectedUserIDs.contains(user.userId) {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundStyle(Color.accentColor)
                                }
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    }
                    .overlay {
                        if model.candidates.isEmpty {
                            ContentUnavailableView(
                                "No contacts to add",
                                systemImage: "person.badge.plus"
                            )
                        } else if model.filteredCandidates(query: query).isEmpty {
                            ContentUnavailableView.search(text: query)
                        }
                    }
                }
            }
            .navigationTitle("Add Participants")
            .searchable(text: $query, prompt: "Search")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Add") {
                        Task {
                            if await model.add(
                                account: account,
                                userIDs: Array(selectedUserIDs)
                            ) {
                                dismiss()
                            }
                        }
                    }
                    .disabled(
                        selectedUserIDs.isEmpty
                            || model.acting
                            || selectedUserIDs.count > model.remainingCapacity
                    )
                }
            }
        }
        .frame(minWidth: 480, minHeight: 560)
        .task {
            await model.loadCandidates(account: account)
        }
    }
}

private struct GroupInviteSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.dismiss) private var dismiss
    @State private var model = GroupInviteModel()
    @State private var resetConfirmationPresented = false
    let conversation: SwiftConversationListItem

    var body: some View {
        VStack(spacing: 18) {
            HStack {
                Spacer()
                Button("Close") {
                    dismiss()
                }
            }

            Image(systemName: "person.3.fill")
                .font(.system(size: 58))
                .foregroundStyle(.secondary)
            Text(model.detail?.name.nonEmpty ?? conversation.name)
                .font(.title3.weight(.semibold))
            if let codeURL = model.detail?.codeUrl.nonEmpty {
                Text(codeURL)
                    .font(.callout)
                    .multilineTextAlignment(.center)
                    .textSelection(.enabled)

                HStack(spacing: 18) {
                    ShareLink(item: codeURL) {
                        Label("Share Link", systemImage: "square.and.arrow.up")
                    }
                    Button("Copy Link", systemImage: "doc.on.doc") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(codeURL, forType: .string)
                    }
                    Button("Reset Link", systemImage: "arrow.clockwise") {
                        resetConfirmationPresented = true
                    }
                    .disabled(model.acting)
                }
            } else if model.loading {
                ProgressView()
            } else {
                ContentUnavailableView(
                    "Invite link unavailable",
                    systemImage: "link.badge.plus"
                )
            }
            Text("Anyone with this link can request to join the group. Reset it if the link was shared unexpectedly.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(28)
        .frame(width: 520, height: 430)
        .task(id: conversation.conversationId) {
            await model.load(
                account: session.handle,
                conversationID: conversation.conversationId
            )
        }
        .confirmationDialog(
            "Reset this invite link?",
            isPresented: $resetConfirmationPresented
        ) {
            Button("Reset Link", role: .destructive) {
                Task {
                    await model.rotate(
                        account: session.handle,
                        conversationID: conversation.conversationId
                    )
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("The current link will stop working.")
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

@MainActor
@Observable
final class GroupParticipantsModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var participants: [SwiftConversationParticipantItem] = []
    private(set) var candidates: [SwiftUserItem] = []
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

    var currentParticipant: SwiftConversationParticipantItem? {
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

    func filteredParticipants(query: String) -> [SwiftConversationParticipantItem] {
        let keyword = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !keyword.isEmpty else {
            return participants
        }
        return participants.filter {
            $0.fullName.lowercased().contains(keyword)
                || $0.identityNumber.contains(keyword)
        }
    }

    func filteredCandidates(query: String) -> [SwiftUserItem] {
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

    func canChangeRole(of participant: SwiftConversationParticipantItem) -> Bool {
        currentParticipant?.role == "OWNER"
            && participant.userId != currentUserID
            && participant.role != "OWNER"
    }

    func canRemove(_ participant: SwiftConversationParticipantItem) -> Bool {
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
        action: SwiftParticipantAction,
        participant: SwiftConversationParticipantItem
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
        participant: SwiftConversationParticipantItem
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
    private(set) var detail: SwiftConversationDetailItem?
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
