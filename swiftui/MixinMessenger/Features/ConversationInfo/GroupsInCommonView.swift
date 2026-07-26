import Observation
import SwiftUI

struct GroupsInCommonView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @State private var model = GroupsInCommonModel()
    let userID: String

    var body: some View {
        Group {
                switch model.state {
                case .loading:
                    ProgressView()
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load groups",
                        systemImage: "exclamationmark.triangle",
                        description: Text(message)
                    )
                case .ready where model.groups.isEmpty:
                    ContentUnavailableView(
                        "No Results",
                        systemImage: "person.3"
                    )
                case .ready:
                    List(model.groups, id: \.conversationId) { group in
                        Button {
                            navigation.infoPresented = false
                            navigation.selectConversation(
                                group.conversationId,
                                name: group.name
                            )
                            dismiss()
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: group.avatarUrl)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: "person.3.fill")
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 50, height: 50)
                                .clipShape(Circle())
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(group.name)
                                        .foregroundStyle(.primary)
                                        .lineLimit(1)
                                    Text("\(group.participantCount) participants")
                                        .font(.callout)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            .frame(minHeight: 62)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Groups in Common")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        Task {
                            await model.load(
                                account: session.handle,
                                userID: userID
                            )
                        }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                    }
                }
            }
        .task(id: userID) {
            await model.load(account: session.handle, userID: userID)
        }
    }
}

@MainActor
@Observable
final class GroupsInCommonModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state = State.loading
    private(set) var groups: [SwiftGroupConversationItem] = []
    private var requestVersion = 0

    func load(account: SwiftAccountHandle, userID: String) async {
        requestVersion += 1
        let version = requestVersion
        state = .loading
        do {
            let groups = try await account.groupsInCommon(userId: userID)
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            self.groups = groups
            state = .ready
        } catch {
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }
}
