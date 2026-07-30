import Observation
import SwiftUI

struct ConversationSharedAppsView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationSharedAppsModel()

    let conversationID: String
    let userID: String
    let isGroup: Bool

    var body: some View {
        AppScrollView {
            VStack(spacing: 0) {
                Spacer().frame(height: 6)
                ForEach(model.apps, id: \.appId) { app in
                    Button {
                        navigation.inspectorPath.append(.userProfile(userID: app.appId))
                    } label: {
                        HStack(spacing: 10) {
                            MixinRemoteImage(url: URL(string: app.iconUrl)) { image in
                                image.resizable().scaledToFill()
                            } placeholder: {
                                Circle().fill(theme.listSelected)
                            }
                            .frame(width: 50, height: 50)
                            .clipShape(Circle())

                            VStack(alignment: .leading, spacing: 3) {
                                Text(app.name)
                                    .font(.system(size: 16))
                                    .foregroundStyle(theme.text)
                                Text(app.description)
                                    .font(.system(size: 14))
                                    .foregroundStyle(theme.secondaryText)
                                    .lineLimit(1)
                            }
                            Spacer()
                        }
                        .padding(.vertical, 10)
                        .padding(.horizontal, 16)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .background(theme.primary)
        .navigationTitle("Shared Apps")
        .task(id: SharedAppsTaskID(userID: userID, isGroup: isGroup)) {
            await model.start(
                account: session.handle,
                userID: userID,
                isGroup: isGroup
            )
        }
    }

}

private struct SharedAppsTaskID: Hashable {
    let userID: String
    let isGroup: Bool
}

@MainActor
@Observable
final class ConversationSharedAppsModel {
    private(set) var apps: [SharedAppItem] = []
    private(set) var loaded = false
    private(set) var refreshing = false
    private var requestVersion = 0

    func start(
        account: SwiftAccountHandle,
        userID: String,
        isGroup: Bool
    ) async {
        requestVersion += 1
        let version = requestVersion
        loaded = false
        apps = []
        guard !isGroup else {
            loaded = true
            return
        }
        do {
            apps = try await account.localSharedApps(userId: userID)
            guard version == requestVersion else {
                return
            }
            loaded = true
        } catch {
            guard version == requestVersion else {
                return
            }
            loaded = true
            // Flutter keeps the blank list if local shared apps are unavailable.
            return
        }
        await refresh(account: account, userID: userID)
    }

    func refresh(account: SwiftAccountHandle, userID: String) async {
        guard !refreshing else {
            return
        }
        refreshing = true
        defer { refreshing = false }
        requestVersion += 1
        let version = requestVersion
        do {
            let remote = try await account.sharedApps(userId: userID)
            guard version == requestVersion else {
                return
            }
            apps = remote
            loaded = true
        } catch {
            guard version == requestVersion else {
                return
            }
        }
    }
}
