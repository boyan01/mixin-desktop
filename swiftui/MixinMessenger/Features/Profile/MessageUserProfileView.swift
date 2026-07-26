import AppKit
import Observation
import SwiftUI

struct MessageUserProfileView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @State private var model = MessageUserProfileModel()
    @State private var sharePresented = false

    let userID: String

    var body: some View {
        Group {
            switch model.state {
            case .loading:
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case let .failed(message):
                ContentUnavailableView(
                    "Unable to Load Profile",
                    systemImage: "person.crop.circle.badge.exclamationmark",
                    description: Text(message)
                )
            case .ready:
                profileContent
            }
        }
        .navigationTitle("Profile")
        .task(id: userID) {
            await model.load(account: session.handle, userID: userID)
        }
        .sheet(isPresented: $sharePresented) {
            ForwardConversationSheet(
                account: session.handle,
                combined: false,
                title: "Share Contact"
            ) { conversationID in
                await model.share(
                    account: session.handle,
                    conversationID: conversationID
                )
            }
        }
        .alert(
            "Unable to Update Profile",
            isPresented: Binding(
                get: { model.operationError != nil },
                set: { if !$0 { model.operationError = nil } }
            )
        ) {
            Button("OK") {
                model.operationError = nil
            }
        } message: {
            Text(model.operationError ?? "")
        }
    }

    @ViewBuilder
    private var profileContent: some View {
        if let user = model.user {
            ScrollView {
                VStack(spacing: 16) {
                    MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                        image.resizable().scaledToFill()
                    } placeholder: {
                        Image(systemName: "person.crop.circle.fill")
                            .resizable()
                            .foregroundStyle(.secondary)
                    }
                    .frame(width: 90, height: 90)
                    .clipShape(Circle())
                    .contentShape(Circle())
                    .onTapGesture {
                        guard NSEvent.modifierFlags.contains(.option) else {
                            return
                        }
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(
                            "mixin://users/\(user.userId)",
                            forType: .string
                        )
                    }
                    .help("Option-click to copy the user link")

                    HStack(spacing: 5) {
                        Text(user.fullName)
                            .font(.title3.weight(.semibold))
                            .textSelection(.enabled)
                        if user.identityNumber != "0" {
                            ProfileIdentityBadge(
                                isVerified: user.isVerified,
                                isBot: user.isBot,
                                membership: user.membership
                            )
                        }
                    }

                    if user.identityNumber != "0" {
                        Text("Mixin ID: \(user.identityNumber)")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                            .textSelection(.enabled)
                    }

                    if user.identityNumber != "0",
                       user.relationship == "STRANGER"
                    {
                        Button(user.isBot ? "Add Bot" : "Add Contact") {
                            Task {
                                await model.updateRelationship(
                                    account: session.handle
                                )
                            }
                        }
                        .buttonStyle(.borderedProminent)
                    }

                    if let biography = user.biography.nonEmpty {
                        Text(biography)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .textSelection(.enabled)
                            .padding(.horizontal, 18)
                    }

                    if user.identityNumber != "0" {
                        HStack(spacing: 28) {
                            profileAction(
                                "Share",
                                systemImage: "square.and.arrow.up"
                            ) {
                                sharePresented = true
                            }

                            if user.userId != session.profile.userId {
                                profileAction(
                                    "Chat",
                                    systemImage: "message"
                                ) {
                                    openConversation(showInfo: false)
                                }
                                profileAction(
                                    "Information",
                                    systemImage: "info.circle"
                                ) {
                                    openConversation(showInfo: true)
                                }
                            }
                        }
                    }

                    if model.updating {
                        ProgressView()
                            .controlSize(.small)
                    }
                }
                .padding(20)
            }
        }
    }

    private func profileAction(
        _ title: String,
        systemImage: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            VStack(spacing: 5) {
                Image(systemName: systemImage)
                    .font(.title3)
                Text(title)
                    .font(.caption)
            }
        }
        .buttonStyle(.borderless)
        .disabled(model.updating)
    }

    private func openConversation(showInfo: Bool) {
        Task {
            guard let user = model.user,
                  let conversationID = await model.openConversation(
                    account: session.handle
                  )
            else {
                return
            }
            if showInfo {
                navigation.showConversationInfo(
                    conversationID: conversationID,
                    name: user.fullName
                )
            } else {
                navigation.infoPresented = false
                navigation.inspectorPath = []
                navigation.selectConversation(
                    conversationID,
                    name: user.fullName
                )
            }
        }
    }
}

@MainActor
@Observable
final class MessageUserProfileModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    private(set) var state = State.loading
    private(set) var user: SwiftUserItem?
    private(set) var updating = false
    var operationError: String?

    private var requestVersion = 0

    func load(account: SwiftAccountHandle, userID: String) async {
        requestVersion += 1
        let version = requestVersion
        state = .loading
        do {
            var user = try await account.userProfile(userId: userID)
            if user == nil {
                user = try await account.refreshUserProfile(userId: userID)
            }
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            guard let user else {
                state = .failed("The requested Mixin user could not be found.")
                return
            }
            self.user = user
            state = .ready
        } catch {
            guard version == requestVersion, !Task.isCancelled else {
                return
            }
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func updateRelationship(account: SwiftAccountHandle) async {
        guard !updating, let user else {
            return
        }
        updating = true
        defer { updating = false }
        do {
            switch user.relationship {
            case "FRIEND":
                try await account.removeContact(userId: user.userId)
            case "BLOCKED":
                try await account.unblockUser(userId: user.userId)
            default:
                try await account.addContact(
                    userId: user.userId,
                    fullName: user.fullName
                )
            }
            self.user = try await account.userProfile(userId: user.userId)
                ?? user
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func openConversation(account: SwiftAccountHandle) async -> String? {
        guard !updating, let user else {
            return nil
        }
        updating = true
        defer { updating = false }
        do {
            return try await account.openUserConversation(userId: user.userId)
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return nil
        }
    }

    func share(
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
}
