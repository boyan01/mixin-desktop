import Observation
import SwiftUI

struct ForwardConversationSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var model = ForwardConversationModel()
    let account: SwiftAccountHandle
    let combined: Bool
    var title: String?
    let onSelect: (String) async -> Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                        .foregroundStyle(theme.icon)
                }
                .buttonStyle(MixinActionButtonStyle())
                .frame(maxWidth: .infinity, alignment: .leading)
                Text(title ?? (combined ? "Forward as Transcript" : "Forward"))
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                    .frame(maxWidth: .infinity)
                Color.clear
                    .frame(width: 36, height: 36)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .padding(.horizontal, 16)

            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 16))
                    .foregroundStyle(theme.secondaryText)
            TextField("Search", text: $model.query)
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)
                .onChange(of: model.query) {
                    if model.query.count > 200 {
                        model.query = String(model.query.prefix(200))
                    }
                }
            }
            .padding(.horizontal, 16)
            .frame(height: 32)
            .background(theme.background, in: Capsule())
            .padding(.top, 8)
            .padding(.horizontal, 24)

            Group {
                switch model.state {
                case .loading:
                    ProgressView()
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load conversations",
                        systemImage: "exclamationmark.triangle",
                        description: Text(message)
                    )
                case .ready:
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            destinationSections(model.filtered)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .padding(.top, 16)
        .frame(width: 480, height: 600)
        .background(theme.popUp)
        .task {
            await model.load(account: account)
        }
    }

    @ViewBuilder
    private func destinationSections(_ destinations: [ForwardDestination]) -> some View {
        let recent = destinations.filter { $0.status != -1 }
        let contacts = destinations.filter { $0.status == -1 && !$0.isBot }
        let bots = destinations.filter { $0.status == -1 && $0.isBot }
        if !recent.isEmpty {
            sectionHeader("Recent Chats")
            destinationRows(recent)
        }
        if !contacts.isEmpty {
            sectionHeader("Contacts")
            destinationRows(contacts)
        }
        if !bots.isEmpty {
            sectionHeader("Bots")
            destinationRows(bots)
        }
    }

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(.system(size: 16))
            .foregroundStyle(theme.text)
            .frame(maxWidth: .infinity, minHeight: 42, alignment: .leading)
            .padding(.leading, 14)
            .background(theme.popUp)
    }

    private func destinationRows(_ destinations: [ForwardDestination]) -> some View {
        ForEach(destinations) { destination in
            conversationRow(destination)
        }
    }

    private func conversationRow(_ destination: ForwardDestination) -> some View {
        Button {
            Task {
                model.sending = true
                if let conversationID = await model.resolveConversationID(
                    account: account,
                    destination: destination
                ), await onSelect(conversationID) {
                    dismiss()
                } else {
                    model.sending = false
                }
            }
        } label: {
            HStack(spacing: 0) {
                Group {
                    if destination.isGroup {
                        GroupAvatarPuzzle(avatars: Array(destination.groupAvatars.prefix(4)))
                    } else {
                        MixinRemoteImage(url: URL(string: destination.avatarURL)) { image in
                            image.resizable().scaledToFill()
                        } placeholder: {
                            AvatarPlaceholder(userID: destination.ownerID, name: destination.name)
                        }
                    }
                }
                .frame(width: 50, height: 50)
                .clipShape(Circle())
                Spacer().frame(width: 16)
                HStack(spacing: 0) {
                    Text(destination.name)
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                        .lineLimit(1)
                    ProfileIdentityBadge(
                        isVerified: destination.isVerified,
                        isBot: destination.isBot,
                        membership: destination.membership
                    )
                    .padding(.horizontal, 4)
                }
                Spacer()
                if model.sending {
                    ProgressView()
                        .controlSize(.small)
                }
            }
            .padding(.top, 10)
            .padding(.bottom, 10)
            .padding(.leading, 14)
            .padding(.trailing, 10)
            .frame(height: 70)
            .contentShape(RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(MixinRowButtonStyle(selected: false))
        .disabled(model.sending)
    }
}

fileprivate struct ForwardDestination: Identifiable {
    let id: String
    let conversationID: String?
    let ownerID: String
    let name: String
    let avatarURL: String
    let status: Int32
    let isGroup: Bool
    let isVerified: Bool
    let isBot: Bool
    let membership: String?
    let identityNumber: String
    let groupAvatars: [GroupAvatar]
}

@MainActor
@Observable
final class ForwardConversationModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state = State.loading
    fileprivate var destinations: [ForwardDestination] = []
    var query = ""
    var sending = false
    private var requestVersion = 0

    fileprivate var filtered: [ForwardDestination] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return destinations
        }
        return destinations.filter {
            $0.name.localizedCaseInsensitiveContains(query)
                || $0.identityNumber.contains(query)
        }
    }

    func load(account: SwiftAccountHandle) async {
        requestVersion += 1
        let version = requestVersion
        state = .loading
        do {
            let items = try await loadConversations(account: account)
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            let existingOwnerIDs = Set(items.map(\.ownerId))
            destinations = items.map {
                ForwardDestination(
                    id: "conversation:\($0.conversationId)",
                    conversationID: $0.conversationId,
                    ownerID: $0.ownerId,
                    name: $0.name,
                    avatarURL: $0.avatarUrl,
                    status: $0.status,
                    isGroup: $0.category == "GROUP",
                    isVerified: $0.isVerified,
                    isBot: $0.isBot,
                    membership: $0.membership,
                    identityNumber: $0.identityNumber,
                    groupAvatars: $0.groupAvatars
                )
            }
            destinations += try await account.selectableUsers()
                .filter { !existingOwnerIDs.contains($0.userId) }
                .map {
                    ForwardDestination(
                        id: "user:\($0.userId)",
                        conversationID: nil,
                        ownerID: $0.userId,
                        name: $0.fullName,
                        avatarURL: $0.avatarUrl,
                        status: -1,
                        isGroup: false,
                        isVerified: $0.isVerified,
                        isBot: $0.isBot,
                        membership: $0.membership,
                        identityNumber: $0.identityNumber,
                        groupAvatars: []
                    )
                }
            state = .ready
        } catch {
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    fileprivate func resolveConversationID(
        account: SwiftAccountHandle,
        destination: ForwardDestination
    ) async -> String? {
        if let conversationID = destination.conversationID {
            return conversationID
        }
        do {
            return try await account.openUserConversation(userId: destination.ownerID)
        } catch {
            AppLogger.error("Forward conversation open failed", error: error)
            return nil
        }
    }

    private func loadConversations(account: SwiftAccountHandle) async throws -> [ConversationListData] {
        var offset: Int64 = 0
        var result: [ConversationListData] = []
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
                return result
            }
            offset += Int64(page.count)
        }
    }
}
