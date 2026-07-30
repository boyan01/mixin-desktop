import Observation
import SwiftUI

struct GroupsInCommonView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var model = GroupsInCommonModel()
    let userID: String

    var body: some View {
        Group {
                switch model.state {
                case .loading:
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 24, height: 24)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case .failed:
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 24, height: 24)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case .ready where model.groups.isEmpty:
                    VStack(spacing: 0) {
                        Image("EmptyFile")
                            .resizable()
                            .renderingMode(.template)
                            .foregroundStyle(theme.secondaryText)
                            .frame(width: 80, height: 80)
                        Spacer().frame(height: 20)
                        Text("No Results")
                            .foregroundStyle(theme.secondaryText)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                case .ready:
                    AppListView(model.groups, id: \.conversationId) { group in
                        Button {
                            navigation.infoPresented = false
                            navigation.selectConversation(
                                group.conversationId,
                                name: group.name
                            )
                            dismiss()
                        } label: {
                            HStack(spacing: 12) {
                                UserAvatar(
                                    userID: group.conversationId,
                                    name: group.name,
                                    url: group.avatarUrl,
                                    size: 50
                                )
                                VStack(alignment: .leading, spacing: 0) {
                                    Text(group.name)
                                        .font(.system(size: 16))
                                        .foregroundStyle(theme.text)
                                        .lineLimit(1)
                                    Spacer()
                                    Text("\(group.participantCount) participants")
                                        .font(.system(size: 14))
                                        .foregroundStyle(theme.secondaryText)
                                        .lineLimit(1)
                                        .frame(height: 20, alignment: .top)
                                }
                                .padding(.vertical, 16)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            }
                            .padding(.horizontal, 12)
                            .frame(height: 78)
                            .padding(.horizontal, 8)
                        }
                        .buttonStyle(.plain)
                        .listRowInsets(.init())
                        .listRowBackground(Color.clear)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(theme.primary)
            .navigationTitle("Groups in Common")
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
    private(set) var groups: [GroupConversationItem] = []
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
