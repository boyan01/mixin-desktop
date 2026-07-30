import Observation
import SwiftUI

struct MessageSearchSenderSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var model = MessageSearchSenderModel()
    let account: SwiftAccountHandle
    let conversationID: String
    let isBot: Bool
    let onSelect: (ConversationParticipantItem?) -> Void

    var body: some View {
        NavigationStack {
            Group {
                if model.loading {
                    ProgressView()
                } else if let error = model.error {
                    ContentUnavailableView(
                        "Unable to load people",
                        systemImage: "exclamationmark.triangle",
                        description: Text(error)
                    )
                } else {
                    AppListView {
                        Button {
                            onSelect(nil)
                            dismiss()
                        } label: {
                            Label("Anyone", systemImage: "person.2")
                        }
                        ForEach(model.filtered, id: \.userId) { user in
                            Button {
                                onSelect(user)
                                dismiss()
                            } label: {
                                HStack(spacing: 16) {
                                    MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                                        image.resizable().scaledToFill()
                                    } placeholder: {
                                        AvatarPlaceholder(userID: user.userId, name: user.fullName)
                                    }
                                    .frame(width: 38, height: 38)
                                    .clipShape(Circle())
                                    Text(user.fullName)
                                        .font(.system(size: 16))
                                        .foregroundStyle(theme.text)
                                        .lineLimit(1)
                                    ProfileIdentityBadge(
                                        isVerified: user.isVerified,
                                        isBot: user.isBot,
                                        membership: user.membership
                                    )
                                    .padding(.horizontal, 4)
                                    Spacer()
                                }
                                .padding(.horizontal, 20)
                                .padding(.vertical, 6)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            .frame(minWidth: 380, minHeight: 480)
            .navigationTitle("Search by Sender")
            .searchable(text: $model.query)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
        }
        .task(id: model.query) {
            try? await Task.sleep(for: .milliseconds(150))
            guard !Task.isCancelled else {
                return
            }
            await model.load(
                account: account,
                conversationID: conversationID,
                isBot: isBot
            )
        }
    }
}

@MainActor
@Observable
final class MessageSearchSenderModel {
    private(set) var users: [ConversationParticipantItem] = []
    private(set) var loading = false
    private(set) var error: String?
    var query = ""
    private var loadedGroup = false
    private var requestVersion = 0

    var filtered: [ConversationParticipantItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return users
        }
        return users.filter {
            $0.fullName.localizedCaseInsensitiveContains(query)
                || $0.identityNumber.contains(query)
        }
    }

    func load(
        account: SwiftAccountHandle,
        conversationID: String,
        isBot: Bool
    ) async {
        if !isBot, loadedGroup {
            return
        }
        requestVersion += 1
        let version = requestVersion
        loading = true
        error = nil
        do {
            let users = if isBot {
                try await account.searchBotGroupUsers(
                    conversationId: conversationID,
                    keyword: query.trimmingCharacters(in: .whitespacesAndNewlines)
                )
            } else {
                try await account.conversationParticipants(
                    conversationId: conversationID
                )
            }
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            self.users = users
            loadedGroup = !isBot
            loading = false
        } catch {
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            self.error = MixinErrorPresenter.message(for: error)
            loading = false
        }
    }
}
