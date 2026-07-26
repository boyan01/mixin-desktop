import Observation
import SwiftUI

struct ForwardConversationSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var model = ForwardConversationModel()
    let account: SwiftAccountHandle
    let combined: Bool
    var title: String?
    let onSelect: (String) async -> Bool

    var body: some View {
        NavigationStack {
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
                case .ready where model.filtered.isEmpty:
                    ContentUnavailableView.search(text: model.query)
                case .ready:
                    List(model.filtered, id: \.conversationId) { conversation in
                        Button {
                            Task {
                                model.sending = true
                                if await onSelect(conversation.conversationId) {
                                    dismiss()
                                } else {
                                    model.sending = false
                                }
                            }
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: conversation.iconUrl)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: conversation.category == "GROUP"
                                        ? "person.3.fill"
                                        : "person.crop.circle.fill")
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 42, height: 42)
                                .clipShape(Circle())
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(conversation.name)
                                        .foregroundStyle(.primary)
                                    Text(conversation.lastMessage)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }
                                Spacer()
                                if model.sending {
                                    ProgressView()
                                        .controlSize(.small)
                                }
                            }
                        }
                        .buttonStyle(.plain)
                        .disabled(model.sending)
                    }
                }
            }
            .frame(minWidth: 420, minHeight: 520)
            .navigationTitle(
                title ?? (combined ? "Forward as Transcript" : "Forward")
            )
            .searchable(text: $model.query)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        Task {
                            await model.load(account: account)
                        }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                    }
                }
            }
        }
        .task {
            await model.load(account: account)
        }
    }
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
    private(set) var conversations: [SwiftConversationListItem] = []
    var query = ""
    var sending = false
    private var requestVersion = 0

    var filtered: [SwiftConversationListItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return conversations
        }
        return conversations.filter {
            $0.name.localizedCaseInsensitiveContains(query)
                || $0.lastMessage.localizedCaseInsensitiveContains(query)
        }
    }

    func load(account: SwiftAccountHandle) async {
        requestVersion += 1
        let version = requestVersion
        state = .loading
        do {
            let items = try await account.conversations(
                category: "all",
                circleId: nil,
                keyword: "",
                unseenOnly: false,
                limit: 500,
                offset: 0
            )
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            conversations = items
            state = .ready
        } catch {
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }
}
