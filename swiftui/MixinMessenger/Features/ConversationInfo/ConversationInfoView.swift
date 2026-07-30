import AppKit
import Observation
import SwiftUI

struct ConversationInfoView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationInfoModel()
    @State private var shareContactPresented = false
    @State private var editTarget: ConversationEditTarget?
    @State private var pendingUserAction: ConversationInfoModel.RelationshipAction?
    @State private var mutePresented = false
    let conversation: ConversationListData

    var body: some View {
        @Bindable var navigation = navigation
        NavigationStack(path: $navigation.inspectorPath) {
            Group {
                switch model.state {
                case .loading:
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load conversation info",
                        systemImage: "exclamationmark.triangle",
                        description: Text(message)
                    )
                case .ready:
                    infoContent
                }
            }
            .navigationTitle("")
            .navigationDestination(for: ChatInspectorRoute.self) {
                inspectorDestination($0)
            }
        }
        .inspectorColumnWidth(min: 300, ideal: 340, max: 420)
        .task(id: conversation.conversationId) {
            await model.start(
                account: session.handle,
                conversation: conversation,
                currentUserID: session.profile.userId
            )
        }
        .onDisappear {
            model.stop()
        }
        .sheet(isPresented: $shareContactPresented) {
            ForwardConversationSheet(
                account: session.handle,
                combined: false,
                title: "Share Contact"
            ) { destinationID in
                await model.shareContact(
                    account: session.handle,
                    conversationID: destinationID
                )
            }
        }
        .sheet(item: $editTarget) { target in
            ConversationEditSheet(target: target) { value in
                Task {
                    await model.edit(
                        account: session.handle,
                        conversationID: conversation.conversationId,
                        target: target,
                        value: value
                    )
                }
            }
        }
        .alert(
            "Unable to update conversation",
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
        .confirmationDialog(
            pendingUserAction?.confirmationTitle(
                name: model.user?.fullName ?? conversation.name,
                isBot: model.user?.isBot ?? conversation.isBot
            ) ?? "",
            isPresented: Binding(
                get: { pendingUserAction != nil },
                set: { if !$0 { pendingUserAction = nil } }
            )
        ) {
            Button(
                pendingUserAction?.buttonTitle(
                    isBot: model.user?.isBot ?? conversation.isBot
                ) ?? "Confirm",
                role: .destructive
            ) {
                guard let action = pendingUserAction else {
                    return
                }
                pendingUserAction = nil
                Task {
                    await model.updateRelationship(
                        account: session.handle,
                        action: action
                    )
                }
            }
            Button("Cancel", role: .cancel) {}
        }
        .confirmationDialog(
            "Mute Notifications",
            isPresented: $mutePresented
        ) {
            Button("1 Hour") {
                setMuted(duration: 60 * 60)
            }
            Button("8 Hours") {
                setMuted(duration: 8 * 60 * 60)
            }
            Button("1 Week") {
                setMuted(duration: 7 * 24 * 60 * 60)
            }
            Button("1 Year") {
                setMuted(duration: 365 * 24 * 60 * 60)
            }
            Button("Cancel", role: .cancel) {}
        }
    }

    @ViewBuilder
    private func inspectorDestination(_ route: ChatInspectorRoute) -> some View {
        switch route {
        case .circles:
            ConversationCircleManagerView(conversation: conversation)
        case .participants:
            GroupParticipantsView(conversation: conversation)
        case .pinnedMessages:
            ConversationPinnedMessagesView(
                conversationID: conversation.conversationId
            )
        case .sharedContent:
            ConversationSharedContentView(
                conversationID: conversation.conversationId
            )
        case .sharedApps:
            ConversationSharedAppsView(
                conversationID: conversation.conversationId,
                userID: conversation.ownerId,
                isGroup: conversation.category == "GROUP"
            )
        case let .groupsInCommon(userID):
            GroupsInCommonView(userID: userID)
        case .disappearingMessages:
            DisappearingMessagesView(
                selectedSeconds: model.detail?.expireIn ?? 0
            ) { seconds in
                await model.setDisappearingMessages(
                    account: session.handle,
                    conversationID: conversation.conversationId,
                    seconds: seconds
                )
            }
        case let .userProfile(userID):
            MessageUserProfileView(userID: userID)
        }
    }

    private var infoContent: some View {
        AppScrollView {
            VStack(spacing: 0) {
                profileHeader

                if conversation.category == "GROUP" {
                    groupDetails
                } else {
                    userDetails
                }

                detailRows
                ConversationDestructiveActionsView(
                    conversationID: conversation.conversationId,
                    isGroup: conversation.category == "GROUP",
                    isExited: model.isGroupExited,
                    relationshipActions: {
                        if let user = model.user,
                           conversation.category != "GROUP"
                        {
                            if user.relationship == "BLOCKED" {
                                relationshipButton("Unblock", action: .unblock)
                            }
                            if user.relationship != "STRANGER" {
                                relationshipButton(
                                    user.isBot ? "Remove Bot" : "Remove Contact",
                                    action: .remove
                                )
                            } else {
                                relationshipButton("Block", action: .block)
                            }
                        }
                    }
                ) {
                    navigation.conversationDeleted(conversation.conversationId)
                }

                if conversation.category != "GROUP" {
                    InfoGroup {
                        relationshipButton("Report", action: .report)
                    }
                }

                if conversation.category == "GROUP",
                   let createdAt = model.detail?.createdAtMillis
                {
                    Text("Created \(createdAt.formattedDate)")
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                        .padding(.vertical, 16)
                }
            }
            .padding(.top, 8)
        }
        .scrollIndicators(.never)
    }

    private var profileHeader: some View {
        VStack(spacing: 0) {
            ConversationAvatar(conversation: conversation, size: 90)
            .onTapGesture {
                guard NSEvent.modifierFlags.contains(.option) else {
                    return
                }
                copyConversationLink()
            }

            Spacer()
                .frame(height: 10)

            HStack(spacing: 5) {
                Text(model.detail?.name.nonEmpty ?? conversation.name)
                    .font(.system(size: 18))
                    .multilineTextAlignment(.center)
                if conversation.category != "GROUP" {
                    ProfileIdentityBadge(
                        isVerified: model.user?.isVerified ?? conversation.isVerified,
                        isBot: model.user?.isBot ?? conversation.isBot,
                        membership: model.user?.membership ?? conversation.membership
                    )
                }
            }

            Spacer()
                .frame(height: 4)

            if conversation.category == "GROUP" {
                Text("\(model.participants.count) participants")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
            } else if let identity = model.user?.identityNumber {
                Text("Mixin ID: \(identity)")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if let user = model.user, user.relationship == "STRANGER" {
                Button(user.isBot ? "+ Add Bot" : "+ Add Contact") {
                    Task {
                        await model.updateRelationship(
                            account: session.handle,
                            action: .add
                        )
                    }
                }
                .buttonStyle(.plain)
                .font(.system(size: 12))
                .foregroundStyle(.tint)
                .padding(.horizontal, 15)
                .padding(.vertical, 7)
                .background(.tint.opacity(0.1), in: Capsule())
                .padding(.top, 12)
                .disabled(model.editing)
            }

            Spacer()
                .frame(height: 12)

            if let biography = model.user?.biography.nonEmpty
                ?? model.detail?.announcement.nonEmpty
            {
                ExpandableInfoText(text: biography)
                    .padding(.horizontal, 36)
            }
        }
        .padding(.bottom, 32)
    }

    @ViewBuilder
    private var userDetails: some View {
        InfoGroup {
            HStack(spacing: 0) {
                Button {
                    shareContactPresented = true
                } label: {
                    Text("Share Contact")
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.leading, 16)
                        .padding(.vertical, 17)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)

                Menu {
                    Button("Copy Link", systemImage: "doc.on.doc") {
                        guard let shareURL = model.shareURL else {
                            return
                        }
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(
                            shareURL,
                            forType: .string
                        )
                    }
                } label: {
                    Image("InviteShare")
                        .resizable()
                        .renderingMode(.template)
                        .foregroundStyle(theme.icon)
                        .frame(width: 30, height: 30)
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .padding(.trailing, 10)
                .disabled(model.shareURL == nil)
            }
        }
    }

    @ViewBuilder
    private var groupDetails: some View {
        if model.isGroupMember {
            InfoGroup {
                Button {
                    navigation.pushInspector(.participants)
                } label: {
                    InfoRow(title: "Group Participants")
                }
                .buttonStyle(.plain)
            }
        }
    }

    private var detailRows: some View {
        VStack(spacing: 0) {
            InfoGroup {
                VStack(spacing: 0) {
                    Button {
                        navigation.pushInspector(.sharedContent)
                    } label: {
                        InfoRow(title: "Shared Media")
                    }
                    .buttonStyle(.plain)

                    if conversation.category != "GROUP",
                       !model.sharedApps.isEmpty
                    {
                        Button {
                            navigation.pushInspector(.sharedApps)
                        } label: {
                            InfoRow(
                                title: "Shared Apps",
                                sharedApps: model.sharedApps
                            )
                        }
                        .buttonStyle(.plain)
                    }

                    Button {
                        navigation.infoPresented = false
                        navigation.inspectorPath = []
                        navigation.focusMessageSearch()
                    } label: {
                        InfoRow(title: "Search Conversation")
                    }
                    .buttonStyle(.plain)
                }
            }

            if !(conversation.category == "GROUP" && model.isGroupExited) {
                InfoGroup {
                    Button {
                        navigation.pushInspector(.disappearingMessages)
                    } label: {
                        InfoRow(
                            title: "Disappearing Messages",
                            value: model.detail?.expireIn.durationDescription ?? "Off",
                            showsArrow: conversation.category != "GROUP"
                                || model.canManageGroup
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(
                        conversation.category == "GROUP"
                            && !model.canManageGroup
                    )
                }
            }

            if conversation.category == "GROUP", model.canManageGroup {
                InfoGroup {
                    Button {
                        editTarget = .announcement(
                            model.detail?.announcement ?? ""
                        )
                    } label: {
                        InfoRow(
                            title: model.detail?.announcement.isEmpty == false
                                ? "Edit Group Description"
                                : "Add Group Description",
                            showsArrow: false
                        )
                    }
                    .buttonStyle(.plain)
                }
            }

            if !(conversation.category == "GROUP" && model.isGroupExited) {
                InfoGroup {
                    VStack(spacing: 0) {
                        Button {
                            if model.isMuted {
                                setMuted(duration: 0)
                            } else {
                                mutePresented = true
                            }
                        } label: {
                            InfoRow(
                                title: model.isMuted ? "Unmute" : "Mute",
                                value: model.isMuted
                                    ? model.detail?.muteUntilMillis
                                        .formattedMuteUntil
                                    : nil,
                                showsArrow: false
                            )
                        }
                        .buttonStyle(.plain)

                        if conversation.category != "GROUP"
                            || model.canManageGroup
                        {
                            Button {
                                editTarget = conversation.category == "GROUP"
                                    ? .name(model.detail?.name.nonEmpty ?? conversation.name)
                                    : .contactName(model.user?.fullName ?? conversation.name)
                            } label: {
                                InfoRow(title: "Edit Name", showsArrow: false)
                            }
                            .buttonStyle(.plain)
                            .disabled(model.editing || (
                                conversation.category != "GROUP" && model.user == nil
                            ))
                        }
                    }
                }
            }

            if conversation.category != "GROUP" {
                InfoGroup {
                    Button {
                        navigation.pushInspector(
                            .groupsInCommon(userID: conversation.ownerId)
                        )
                    } label: {
                        InfoRow(title: "Groups in Common")
                    }
                    .buttonStyle(.plain)
                }
            }

            if let developerID = model.developerID {
                InfoGroup {
                    Button {
                        navigation.pushInspector(
                            .userProfile(userID: developerID)
                        )
                    } label: {
                        InfoRow(title: "Developer", showsArrow: false)
                    }
                    .buttonStyle(.plain)
                }
            }

            InfoGroup {
                Button {
                    navigation.pushInspector(.circles)
                } label: {
                    InfoRow(title: "Edit Conversations")
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func relationshipButton(
        _ title: String,
        action: ConversationInfoModel.RelationshipAction
    ) -> some View {
        Button(role: .destructive) {
            pendingUserAction = action
        } label: {
            InfoRow(
                title: title,
                showsArrow: false,
                destructive: true
            )
        }
        .buttonStyle(.plain)
        .disabled(model.editing)
    }

    private func setMuted(duration: Int64) {
        Task {
            await model.setMuted(
                account: session.handle,
                conversation: conversation,
                duration: duration
            )
        }
    }

    private func copyConversationLink() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(
            "mixin://conversations/\(conversation.conversationId)",
            forType: .string
        )
    }
}

struct ExpandableInfoText: View {
    @Environment(\.mixinTheme) private var theme
    @State private var expanded = false
    @State private var availableWidth: CGFloat = 0

    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 14))
            .foregroundStyle(theme.text)
            .multilineTextAlignment(.center)
            .lineLimit(expanded ? nil : 6)
            .textSelection(.enabled)
            .frame(maxWidth: .infinity)
            .overlay(alignment: .bottomTrailing) {
                if overflows, !expanded {
                    Button("...More") {
                        expanded = true
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 14))
                    .foregroundStyle(theme.accent)
                    .padding(.leading, 8)
                    .background(
                        LinearGradient(
                            colors: [theme.primary.opacity(0), theme.primary],
                            startPoint: .leading,
                            endPoint: .center
                        )
                    )
                }
            }
            .onGeometryChange(for: CGFloat.self) { proxy in
                proxy.size.width
            } action: { width in
                availableWidth = width
            }
            .onChange(of: text) {
                expanded = false
            }
    }

    private var overflows: Bool {
        guard availableWidth > 0 else {
            return false
        }
        let font = NSFont.systemFont(ofSize: 14)
        let bounds = (text as NSString).boundingRect(
            with: CGSize(
                width: availableWidth,
                height: .greatestFiniteMagnitude
            ),
            options: [.usesLineFragmentOrigin, .usesFontLeading],
            attributes: [.font: font]
        )
        let lineHeight = font.ascender - font.descender + font.leading
        return bounds.height > lineHeight * 6 + 0.5
    }
}

private struct InfoGroup<Content: View>: View {
    @Environment(\.mixinTheme) private var theme

    @ViewBuilder let content: () -> Content

    var body: some View {
        content()
            .frame(maxWidth: .infinity)
            .background(theme.listSelected)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .padding(.horizontal, 10)
            .padding(.bottom, 10)
            .frame(maxWidth: 600)
    }
}

private struct InfoRow: View {
    @Environment(\.mixinTheme) private var theme

    let title: String
    var value: String?
    var sharedApps: [SharedAppItem] = []
    var showsArrow = true
    var destructive = false

    var body: some View {
        HStack(spacing: 4) {
            Text(title)
                .font(.system(size: 16))
                .foregroundStyle(destructive ? theme.destructive : theme.text)
            Spacer()
            if let value {
                Text(value)
                    .font(.system(size: 14))
                    .foregroundStyle(theme.secondaryText)
            }
            if !sharedApps.isEmpty {
                SharedAppIconStack(apps: sharedApps)
            }
            if showsArrow {
                Image("SettingsArrow")
                    .resizable()
                    .renderingMode(.template)
                    .foregroundStyle(theme.secondaryText)
                    .frame(width: 30, height: 30)
            }
        }
        .padding(.leading, 16)
        .padding(.trailing, 10)
        .padding(.vertical, 17)
        .contentShape(Rectangle())
    }
}

private struct SharedAppIconStack: View {
    @Environment(\.mixinTheme) private var theme

    let apps: [SharedAppItem]

    var body: some View {
        ZStack(alignment: .leading) {
            ForEach(Array(apps.enumerated().reversed()), id: \.element.appId) { index, app in
                MixinRemoteImage(url: URL(string: app.iconUrl)) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Color.clear
                }
                .frame(width: 24, height: 24)
                .background(theme.listSelected)
                .clipShape(Circle())
                .overlay(Circle().stroke(theme.popUp, lineWidth: 2))
                .offset(x: CGFloat(index * 14))
            }
        }
        .frame(width: 24 + CGFloat(max(0, apps.count - 1) * 14), height: 24)
    }
}

@MainActor
@Observable
final class ConversationInfoModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state: State = .loading
    private(set) var detail: ConversationDetailItem?
    private(set) var user: UserProfileItem?
    private(set) var participants: [ConversationParticipantItem] = []
    private(set) var sharedApps: [SharedAppItem] = []
    private(set) var developerID: String?
    private(set) var operationError: String?
    private(set) var currentUserID = ""
    private(set) var editing = false
    private(set) var isMuted = false
    private(set) var participantsLoaded = false
    private var requestVersion = 0
    private var subscription: SwiftConversationSubscription?
    private var subscriptionTask: Task<Void, Never>?

    var shareURL: String? {
        if let codeURL = detail?.codeUrl.nonEmpty {
            return codeURL
        }
        return user?.codeUrl.nonEmpty
    }

    var canManageGroup: Bool {
        guard let current = participants.first(where: { $0.userId == currentUserID }) else {
            return false
        }
        return current.role == "OWNER" || current.role == "ADMIN"
    }

    var isGroupMember: Bool {
        participants.contains { $0.userId == currentUserID }
    }

    var isGroupExited: Bool {
        participantsLoaded && !isGroupMember
    }

    func start(
        account: SwiftAccountHandle,
        conversation: ConversationListData,
        currentUserID: String
    ) async {
        stop()
        requestVersion += 1
        let version = requestVersion
        self.currentUserID = currentUserID
        state = .loading
        detail = nil
        user = nil
        participants = []
        sharedApps = []
        developerID = nil
        isMuted = conversation.isMuted
        participantsLoaded = false
        let subscription = account.conversationChanges()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let event = await subscription.next()
            {
                guard let self,
                      event.reloadAll
                        || event.conversationIds.contains(
                            conversation.conversationId
                        )
                else {
                    continue
                }
                await reloadConversationState(
                    account: account,
                    conversation: conversation,
                    version: version
                )
            }
        }

        do {
            detail = try await account.localConversationDetail(
                conversationId: conversation.conversationId
            )
            guard version == requestVersion else {
                return
            }
            updateMutedState()
            state = .ready

            if conversation.category == "GROUP" {
                participants = try await account.conversationParticipants(
                    conversationId: conversation.conversationId
                )
                participantsLoaded = true
            } else {
                user = try await account.userProfile(userId: conversation.ownerId)
                sharedApps = try await account.localSharedApps(
                    userId: conversation.ownerId
                )
                developerID = try await account.botCreatorId(
                    userId: conversation.ownerId
                )
            }
            guard version == requestVersion else {
                return
            }
            if let refreshed = try? await account.conversationDetail(
                conversationId: conversation.conversationId
            ), version == requestVersion {
                detail = refreshed
                updateMutedState()
            }
            if conversation.category != "GROUP",
               let remoteApps = try? await account.sharedApps(
                   userId: conversation.ownerId
               ),
               version == requestVersion
            {
                sharedApps = remoteApps
            }
        } catch {
            guard version == requestVersion else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func stop() {
        requestVersion += 1
        subscriptionTask?.cancel()
        subscriptionTask = nil
        subscription = nil
    }

    private func reloadConversationState(
        account: SwiftAccountHandle,
        conversation: ConversationListData,
        version: Int
    ) async {
        do {
            let loadedDetail = try await account.localConversationDetail(
                conversationId: conversation.conversationId
            )
            let loadedParticipants =
                if conversation.category == "GROUP" {
                    try await account.conversationParticipants(
                        conversationId: conversation.conversationId
                    )
                } else {
                    participants
                }
            guard version == requestVersion else {
                return
            }
            detail = loadedDetail
            participants = loadedParticipants
            participantsLoaded = conversation.category == "GROUP"
                || participantsLoaded
            updateMutedState()
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func updateMutedState() {
        guard let detail else {
            return
        }
        isMuted =
            detail.muteUntilMillis
            > Int64(Date().timeIntervalSince1970 * 1_000)
    }

    func edit(
        account: SwiftAccountHandle,
        conversationID: String,
        target: ConversationEditTarget,
        value: String
    ) async {
        if target.requiresGroupPermission, !canManageGroup {
            operationError = "Only a group owner or admin can edit group details."
            return
        }
        guard !editing else {
            return
        }
        let normalized: String
        switch target {
        case .name, .contactName:
            normalized = value
            guard !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  value.count <= 40
            else {
                operationError = "Names must contain 1 to 40 characters."
                return
            }
        case .announcement:
            normalized = value
            guard value.count <= 512 else {
                operationError = "Group descriptions can contain at most 512 characters."
                return
            }
        }
        editing = true
        defer { editing = false }
        do {
            switch target {
            case .name:
                try await account.editConversation(
                    conversationId: conversationID,
                    name: normalized,
                    announcement: nil
                )
            case .announcement:
                try await account.editConversation(
                    conversationId: conversationID,
                    name: nil,
                    announcement: normalized
                )
            case .contactName:
                guard let user else {
                    operationError = "The contact profile is unavailable."
                    return
                }
                try await account.addContact(
                    userId: user.userId,
                    fullName: normalized
                )
                self.user = try await account.userProfile(userId: user.userId)
            }
            if target.requiresGroupPermission {
                detail = try await account.conversationDetail(
                    conversationId: conversationID
                )
            }
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func dismissError() {
        operationError = nil
    }

    enum RelationshipAction: Identifiable {
        case add
        case block
        case remove
        case unblock
        case report

        var id: String {
            String(describing: self)
        }

        func buttonTitle(isBot: Bool) -> String {
            switch self {
            case .add:
                isBot ? "Add Bot" : "Add Contact"
            case .block:
                "Block"
            case .remove:
                isBot ? "Remove Bot" : "Remove Contact"
            case .unblock:
                "Unblock"
            case .report:
                "Report and Block"
            }
        }

        func confirmationTitle(name: String, isBot: Bool) -> String {
            "\(buttonTitle(isBot: isBot)) \(name)?"
        }
    }

    func updateRelationship(
        account: SwiftAccountHandle,
        action: RelationshipAction
    ) async {
        guard !editing, let user else {
            return
        }
        editing = true
        defer { editing = false }
        do {
            switch action {
            case .add:
                try await account.addContact(
                    userId: user.userId,
                    fullName: user.fullName
                )
            case .block:
                try await account.blockUser(userId: user.userId)
            case .remove:
                try await account.removeContact(userId: user.userId)
            case .unblock:
                try await account.unblockUser(userId: user.userId)
            case .report:
                try await account.reportUser(userId: user.userId)
            }
            self.user = try await account.userProfile(userId: user.userId)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func shareContact(
        account: SwiftAccountHandle,
        conversationID: String
    ) async -> Bool {
        guard let user else {
            return false
        }
        do {
            _ = try await account.sendContact(
                conversationId: conversationID,
                sharedUserId: user.userId,
                quoteMessageId: nil,
                silent: false
            )
            return true
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func setDisappearingMessages(
        account: SwiftAccountHandle,
        conversationID: String,
        seconds: Int64
    ) async -> Bool {
        do {
            try await account.setDisappearingMessages(
                conversationId: conversationID,
                duration: seconds
            )
            detail = try await account.localConversationDetail(
                conversationId: conversationID
            )
            return true
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func setMuted(
        account: SwiftAccountHandle,
        conversation: ConversationListData,
        duration: Int64
    ) async {
        guard !editing else {
            return
        }
        editing = true
        defer { editing = false }
        do {
            try await account.setConversationMuted(
                conversationId: conversation.conversationId,
                ownerId: conversation.ownerId,
                category: conversation.category,
                durationSeconds: duration
            )
            detail = try await account.localConversationDetail(
                conversationId: conversation.conversationId
            )
            updateMutedState()
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }
}

private struct DisappearingMessagesView: View {
    private static let presets: [(String, Int64)] = [
        ("Off", 0),
        ("30 seconds", 30),
        ("10 minutes", 600),
        ("2 hours", 7_200),
        ("1 day", 86_400),
        ("1 week", 604_800),
    ]

    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var selectedSeconds: Int64
    @State private var customPresented = false
    @State private var saving = false
    let onSelect: (Int64) async -> Bool

    init(
        selectedSeconds: Int64,
        onSelect: @escaping (Int64) async -> Bool
    ) {
        _selectedSeconds = State(initialValue: selectedSeconds)
        self.onSelect = onSelect
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                Spacer()
                    .frame(height: 30)

            Image("DisappearingMessage")
                .resizable()
                .frame(width: 70, height: 70)

            Spacer()
                .frame(height: 16)

            Text(disappearingHint)
                .font(.system(size: 14))
                .tint(theme.accent)
                .multilineTextAlignment(.center)
                .lineSpacing(7)
                .padding(.horizontal, 20)

            Spacer()
                .frame(height: 40)

            InfoGroup {
                ForEach(Self.presets, id: \.1) { label, seconds in
                    Button {
                        save(seconds)
                    } label: {
                        HStack {
                            InfoRow(title: label, showsArrow: false)
                            if selectedSeconds == seconds {
                                Image(systemName: "checkmark")
                                    .font(.system(size: 24, weight: .medium))
                                    .foregroundStyle(theme.accent)
                            }
                        }
                    }
                    .buttonStyle(.plain)
                    .disabled(selectedSeconds == seconds)
                }

                Button {
                    customPresented = true
                } label: {
                    InfoRow(title: "Custom Time")
                }
                .buttonStyle(.plain)
            }
            }
        }
        .navigationTitle("Disappearing Messages")
        .sheet(isPresented: $customPresented) {
            CustomDisappearingDurationSheet { seconds in
                save(seconds)
            }
        }
        .disabled(saving)
    }

    private func save(_ seconds: Int64) {
        saving = true
        Task {
            if await onSelect(seconds) {
                selectedSeconds = seconds
            }
            saving = false
        }
    }

    private var disappearingHint: AttributedString {
        var text = AttributedString(
            "When enabled, new messages sent and received in this chat will disappear after they have been seen. Read the document to "
        )
        text.foregroundColor = theme.secondaryText
        var link = AttributedString("learn more")
        link.foregroundColor = theme.accent
        link.link = URL(
            string: "https://support.mixin.one/en/article/how-to-enable-disappearing-messages-2nzaz8/"
        )
        text.append(link)
        var suffix = AttributedString(".")
        suffix.foregroundColor = theme.secondaryText
        text.append(suffix)
        return text
    }

}

private struct CustomDisappearingDurationSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var value = 1
    @State private var unit = CustomUnit.second

    let onSet: (Int64) -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Spacer()
                Text("Custom Time")
                    .font(.system(size: 16, weight: .semibold))
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                        .frame(width: 40, height: 40)
                }
                .buttonStyle(.plain)
            }
            .frame(height: 64)

            Spacer()
                .frame(height: 16)

            HStack(spacing: 16) {
                TextField("", value: $value, format: .number)
                    .textFieldStyle(.plain)
                    .multilineTextAlignment(.center)
                    .font(.system(size: 16))
                    .frame(width: 64, height: 46)
                    .background(theme.sidebarSelected, in: RoundedRectangle(cornerRadius: 8))
                    .onChange(of: value) {
                        value = min(max(value, 0), 99)
                    }

                Picker("", selection: $unit) {
                    ForEach(CustomUnit.allCases) { option in
                        Text(option.rawValue).tag(option)
                    }
                }
                .labelsHidden()
                .frame(width: 160, height: 46)
                .background(theme.sidebarSelected, in: RoundedRectangle(cornerRadius: 8))
            }

            Spacer()
                .frame(height: 28)

            Button("Set") {
                guard value > 0, value <= unit.maximumValue else {
                    return
                }
                onSet(unit.seconds * Int64(value))
                dismiss()
            }
            .buttonStyle(.borderedProminent)

            Spacer()
                .frame(height: 24)
        }
        .frame(width: 320)
    }

    private enum CustomUnit: String, CaseIterable, Identifiable {
        case second = "Seconds"
        case minute = "Minutes"
        case hour = "Hours"
        case day = "Days"
        case week = "Weeks"

        var id: String { rawValue }
        var maximumValue: Int {
            switch self {
            case .second, .minute: 59
            case .hour: 23
            case .day: 6
            case .week: 4
            }
        }
        var seconds: Int64 {
            switch self {
            case .second: 1
            case .minute: 60
            case .hour: 3_600
            case .day: 86_400
            case .week: 604_800
            }
        }
    }
}

enum ConversationEditTarget: Identifiable {
    case name(String)
    case contactName(String)
    case announcement(String)

    var id: String {
        switch self {
        case .name:
            "name"
        case .contactName:
            "contact-name"
        case .announcement:
            "announcement"
        }
    }

    var title: String {
        switch self {
        case .name:
            "Edit Group Name"
        case .contactName:
            "Edit Name"
        case let .announcement(value):
            value.isEmpty ? "Add Group Description" : "Edit Group Description"
        }
    }

    var initialValue: String {
        switch self {
        case let .name(value),
             let .contactName(value),
             let .announcement(value):
            value
        }
    }

    var maximumLength: Int {
        switch self {
        case .name, .contactName:
            40
        case .announcement:
            512
        }
    }

    var requiresGroupPermission: Bool {
        switch self {
        case .name, .announcement:
            true
        case .contactName:
            false
        }
    }

    var requiresNonEmptyValue: Bool {
        switch self {
        case .name, .contactName:
            true
        case .announcement:
            false
        }
    }
}

private struct ConversationEditSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var value: String
    let target: ConversationEditTarget
    let onSave: (String) -> Void

    init(
        target: ConversationEditTarget,
        onSave: @escaping (String) -> Void
    ) {
        self.target = target
        self.onSave = onSave
        _value = State(initialValue: target.initialValue)
    }

    var body: some View {
        NavigationStack {
            Group {
                switch target {
                case .name, .contactName:
                    TextField(
                        target.id == "name" ? "Group name" : "Contact name",
                        text: $value
                    )
                        .textFieldStyle(.roundedBorder)
                        .padding()
                case .announcement:
                    TextEditor(text: $value)
                        .font(.body)
                        .padding(8)
                        .overlay {
                            RoundedRectangle(cornerRadius: 6)
                                .stroke(.separator)
                        }
                        .padding()
                }
            }
            .navigationTitle(target.title)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        onSave(value)
                        dismiss()
                    }
                    .disabled(
                        value.count > target.maximumLength
                            || (target.requiresNonEmptyValue
                                && value.trimmingCharacters(
                                    in: .whitespacesAndNewlines
                                ).isEmpty)
                    )
                }
            }
            .safeAreaInset(edge: .bottom) {
                Text("\(value.count)/\(target.maximumLength)")
                    .font(.caption)
                    .foregroundStyle(
                        value.count > target.maximumLength ? .red : .secondary
                    )
                    .frame(maxWidth: .infinity, alignment: .trailing)
                    .padding()
            }
        }
        .frame(
            minWidth: 440,
            minHeight: target.id == "name" ? 170 : 360
        )
    }
}

extension String {
    var nonEmpty: String? {
        isEmpty ? nil : self
    }
}

private extension Int64 {
    var formattedDate: String {
        Date(timeIntervalSince1970: TimeInterval(self) / 1_000)
            .formatted(date: .abbreviated, time: .omitted)
    }

    var durationDescription: String {
        switch self {
        case ...0:
            return "Off"
        case ..<60:
            return "\(self) seconds"
        case ..<3_600:
            return "\(self / 60) minutes"
        case ..<86_400:
            return "\(self / 3_600) hours"
        default:
            return "\(self / 86_400) days"
        }
    }

    var formattedMuteUntil: String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy/MM/dd, hh:mm a"
        return formatter.string(
            from: Date(timeIntervalSince1970: TimeInterval(self) / 1_000)
        )
    }
}
