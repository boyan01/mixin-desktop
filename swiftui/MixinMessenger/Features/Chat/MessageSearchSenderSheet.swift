import Observation
import SwiftUI

struct MessageSearchSenderSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var model = MessageSearchSenderModel()
    let account: SwiftAccountHandle
    let conversationID: String
    let isBot: Bool
    let onSelect: (SwiftConversationParticipantItem?) -> Void

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
                    List {
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
                                HStack(spacing: 12) {
                                    MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                                        image.resizable().scaledToFill()
                                    } placeholder: {
                                        Image(systemName: "person.crop.circle.fill")
                                            .foregroundStyle(.secondary)
                                    }
                                    .frame(width: 38, height: 38)
                                    .clipShape(Circle())
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(user.fullName)
                                            .foregroundStyle(.primary)
                                        Text(user.identityNumber)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                    }
                                }
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
    private(set) var users: [SwiftConversationParticipantItem] = []
    private(set) var loading = false
    private(set) var error: String?
    var query = ""
    private var loadedGroup = false
    private var requestVersion = 0

    var filtered: [SwiftConversationParticipantItem] {
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
