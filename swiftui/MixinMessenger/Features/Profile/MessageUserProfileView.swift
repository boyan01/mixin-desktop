import AppKit
import Observation
import SwiftUI

struct MessageUserProfileView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.mixinTheme) private var theme
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
            AppScrollView {
                VStack(spacing: 0) {
                    UserAvatar(
                        userID: user.userId,
                        name: user.fullName,
                        url: user.avatarUrl,
                        size: 90
                    )
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

                    Spacer()
                        .frame(height: 8)

                    HStack(spacing: 5) {
                        Text(user.fullName)
                            .font(.system(size: 16))
                            .foregroundStyle(theme.text)
                            .textSelection(.enabled)
                        if user.identityNumber != "0" {
                            ProfileIdentityBadge(
                                isVerified: user.isVerified,
                                isBot: user.isBot,
                                membership: user.membership
                            )
                        }
                    }
                    .padding(.horizontal, 32)

                    if user.identityNumber != "0" {
                        Text("Mixin ID: \(user.identityNumber)")
                            .font(.system(size: 12))
                            .foregroundStyle(theme.secondaryText)
                            .textSelection(.enabled)
                            .padding(.top, 4)
                    }

                    if user.identityNumber != "0",
                       user.relationship == "STRANGER"
                    {
                        Button {
                            Task {
                                await model.updateRelationship(
                                    account: session.handle
                                )
                            }
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: "plus")
                                Text(user.isBot ? "Add Bot" : "Add Contact")
                            }
                        }
                        .buttonStyle(.plain)
                        .font(.system(size: 12))
                        .foregroundStyle(theme.accent)
                        .padding(.horizontal, 15)
                        .padding(.vertical, 7)
                        .background(theme.accent.opacity(0.1), in: Capsule())
                        .padding(.top, 8)
                    }

                    if let biography = user.biography.nonEmpty {
                        AppScrollView {
                            ExpandableInfoText(text: biography)
                                .lineSpacing(7)
                                .padding(.horizontal, 36)
                        }
                        .frame(minWidth: 160, maxHeight: 120)
                        .padding(.top, 20)
                    }

                    if user.identityNumber != "0" {
                        Group {
                            if user.userId == session.profile.userId {
                                profileAction(assetName: "InviteShare") {
                                    sharePresented = true
                                }
                                .contextMenu {
                                    copyLinkButton(user.codeUrl)
                                }
                                .frame(maxWidth: .infinity)
                            } else {
                                HStack {
                                    profileAction(assetName: "InviteShare") {
                                        sharePresented = true
                                    }
                                    .contextMenu {
                                        copyLinkButton(user.codeUrl)
                                    }
                                    Spacer()
                                    profileAction(assetName: "ChatSmall") {
                                        openConversation(showInfo: false)
                                    }
                                    Spacer()
                                    profileAction(assetName: "Information") {
                                        openConversation(showInfo: true)
                                    }
                                }
                            }
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.horizontal, 45)
                        .padding(.top, 24)
                    }

                    if model.updating {
                        ProgressView()
                            .controlSize(.small)
                    }

                    Spacer()
                        .frame(height: 56)
                }
                .frame(maxWidth: 340)
                .frame(maxWidth: .infinity)
            }
        }
    }

    private func profileAction(
        assetName: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(assetName)
                .resizable()
                .renderingMode(.template)
                .foregroundStyle(theme.icon)
                .frame(width: 30, height: 30)
        }
        .buttonStyle(.borderless)
        .disabled(model.updating)
    }

    private func copyLinkButton(_ link: String) -> some View {
        Button("Copy Link", systemImage: "doc.on.doc") {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(link, forType: .string)
        }
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
    private(set) var user: UserProfileItem?
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
            if let refreshed = try? await account.refreshUserProfile(
                userId: userID
            ), version == requestVersion, !Task.isCancelled {
                self.user = refreshed
            }
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
