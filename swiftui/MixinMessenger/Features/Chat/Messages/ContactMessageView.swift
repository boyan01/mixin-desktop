import SwiftUI

struct ContactMessageView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(SettingsPreferencesModel.self) private var preferences
    @Environment(\.mixinTheme) private var theme

    let message: MessageItem

    var body: some View {
        Button {
            guard let userID = message.sharedUserId,
                  let url = URL(string: "mixin://users/\(userID)")
            else {
                return
            }
            Task {
                await navigation.open(url, account: session.handle)
            }
        } label: {
            HStack(spacing: 8) {
                MixinRemoteImage(
                    url: message.sharedUserAvatarUrl.flatMap(URL.init(string:))
                ) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    AvatarPlaceholder(
                        userID: message.sharedUserId ?? "",
                        name: message.sharedUserFullName ?? ""
                    )
                }
                .frame(width: 40, height: 40)
                .clipShape(Circle())

                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: 4) {
                        Text(message.sharedUserFullName ?? "Contact")
                            .font(
                                .system(
                                    size: 16
                                        + preferences.chatFontSizeDelta
                                )
                            )
                            .foregroundStyle(theme.text)
                            .lineLimit(1)
                        ProfileIdentityBadge(
                            isVerified: message.sharedUserIsVerified,
                            isBot: message.sharedUserAppId != nil,
                            membership: message.sharedUserMembership
                        )
                    }
                    Text(message.sharedUserIdentityNumber ?? "")
                        .font(
                            .system(
                                size: 14
                                    + preferences.chatFontSizeDelta
                            )
                        )
                        .foregroundStyle(theme.secondaryText)
                }
            }
        }
        .buttonStyle(.plain)
        .disabled(message.sharedUserId == nil)
        .help(message.sharedUserId == nil ? "Contact unavailable" : "Open profile")
    }
}
