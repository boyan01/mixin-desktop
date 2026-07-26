import Observation
import SwiftUI

struct ConversationSharedAppsView: View {
    @Environment(AccountSession.self) private var session
    @State private var model = ConversationSharedAppsModel()

    let conversationID: String
    let userID: String

    var body: some View {
        Group {
                if !model.loaded, model.apps.isEmpty {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if model.apps.isEmpty {
                    ContentUnavailableView(
                        "No Shared Apps",
                        systemImage: "square.grid.2x2"
                    )
                } else {
                    List(model.apps, id: \.appId) { app in
                        Button {
                            open(app)
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: app.iconUrl)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: "app.fill")
                                        .resizable()
                                        .padding(9)
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 50, height: 50)
                                .background(Color.secondary.opacity(0.08))
                                .clipShape(Circle())

                                VStack(alignment: .leading, spacing: 4) {
                                    Text(app.name)
                                        .font(.headline)
                                        .foregroundStyle(.primary)
                                    Text(app.description)
                                        .font(.subheadline)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(2)
                                }
                            }
                            .padding(.vertical, 6)
                        }
                        .buttonStyle(.plain)
                    }
                    .listStyle(.inset)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Shared Apps")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    if model.refreshing {
                        ProgressView()
                            .controlSize(.small)
                    }
                }
            }
        .task(id: userID) {
            await model.start(account: session.handle, userID: userID)
        }
        .alert(
            "Unable to load shared apps",
            isPresented: Binding(
                get: { model.errorMessage != nil },
                set: { if !$0 { model.errorMessage = nil } }
            )
        ) {
            Button("Retry") {
                Task {
                    await model.refresh(
                        account: session.handle,
                        userID: userID
                    )
                }
            }
            Button("OK", role: .cancel) {
                model.errorMessage = nil
            }
        } message: {
            Text(model.errorMessage ?? "")
        }
    }

    private func open(_ app: SwiftSharedAppItem) {
        guard let url = URL(string: app.homeUri), !app.homeUri.isEmpty else {
            model.errorMessage = "This app does not provide a valid home URL."
            return
        }
        BotWebViewWindow.open(
            url: url,
            title: app.name,
            conversationID: conversationID,
            currency: session.profile.fiatCurrency
        )
    }
}

@MainActor
@Observable
final class ConversationSharedAppsModel {
    private(set) var apps: [SwiftSharedAppItem] = []
    private(set) var loaded = false
    private(set) var refreshing = false
    var errorMessage: String?

    private var requestVersion = 0

    func start(account: SwiftAccountHandle, userID: String) async {
        requestVersion += 1
        let version = requestVersion
        loaded = false
        apps = []
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
            errorMessage = MixinErrorPresenter.message(for: error)
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
            errorMessage = nil
        } catch {
            guard version == requestVersion else {
                return
            }
            if apps.isEmpty {
                errorMessage = MixinErrorPresenter.message(for: error)
            }
        }
    }
}
