import SwiftUI

struct ContactShareSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(MixinNoticeCenter.self) private var noticeCenter
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var users: [UserProfileItem] = []
    @State private var query = ""
    @State private var sendingUserID: String?
    @FocusState private var searchFocused: Bool
    let onSend: (String) async -> Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                }
                .buttonStyle(MixinActionButtonStyle())
                .frame(maxWidth: .infinity, alignment: .leading)

                Text("Select")
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                    .frame(maxWidth: .infinity)

                Color.clear
                    .frame(width: 36, height: 36)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .padding(.horizontal, 16)

            MixinSearchField(
                text: $query,
                focus: $searchFocused,
                placeholder: "Search"
            )
            .frame(height: 32)
            .padding(.top, 8)
            .padding(.horizontal, 24)

            Spacer().frame(height: 8)

            AppScrollView {
                LazyVStack(spacing: 0) {
                    contactSection(
                        title: "Contacts",
                        users: filteredUsers.filter { !$0.isBot }
                    )
                    contactSection(
                        title: "Bots",
                        users: filteredUsers.filter(\.isBot)
                    )
                }
                .padding(.horizontal, 16)
            }
        }
        .padding(.top, 16)
        .frame(width: 480, height: 600)
        .background(theme.popUp)
        .task {
            await load()
            searchFocused = true
        }
    }

    @ViewBuilder
    private func contactSection(
        title: String,
        users: [UserProfileItem]
    ) -> some View {
        if !users.isEmpty {
            Text(title)
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
                .frame(
                    maxWidth: .infinity,
                    minHeight: 42,
                    alignment: .leading
                )
                .padding(.leading, 14)
                .background(theme.popUp)

            ForEach(users, id: \.userId) { user in
                Button {
                    send(user.userId)
                } label: {
                    HStack(spacing: 16) {
                        UserAvatar(
                            userID: user.userId,
                            name: user.fullName,
                            url: user.avatarUrl,
                            size: 50
                        )
                        HStack(spacing: 0) {
                            highlightedName(user.fullName)
                            ProfileIdentityBadge(
                                isVerified: user.isVerified,
                                isBot: user.isBot,
                                membership: user.membership
                            )
                            .padding(.horizontal, 4)
                        }
                        Spacer()
                        if sendingUserID == user.userId {
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
                .disabled(sendingUserID != nil)
            }
        }
    }

    private func highlightedName(_ name: String) -> Text {
        let keyword = query.trimmingCharacters(
            in: .whitespacesAndNewlines
        )
        guard !keyword.isEmpty,
              let range = name.range(
                of: keyword,
                options: [.caseInsensitive]
              )
        else {
            return Text(name)
                .foregroundColor(theme.text)
        }
        return Text(String(name[..<range.lowerBound]))
            .foregroundColor(theme.text)
            + Text(String(name[range]))
            .foregroundColor(theme.accent)
            + Text(String(name[range.upperBound...]))
            .foregroundColor(theme.text)
    }

    private var filteredUsers: [UserProfileItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        guard !query.isEmpty else {
            return users
        }
        return users.filter {
            $0.fullName.lowercased().contains(query)
                || $0.identityNumber.contains(query)
        }
    }

    private func load() async {
        do {
            users = try await session.handle.selectableUsers()
        } catch {
            AppLogger.error("Load contacts failed", error: error)
        }
    }

    private func send(_ userID: String) {
        sendingUserID = userID
        Task {
            if await onSend(userID) {
                dismiss()
            } else {
                noticeCenter.show(
                    "The contact could not be shared.",
                    kind: .failure
                )
                sendingUserID = nil
            }
        }
    }
}
