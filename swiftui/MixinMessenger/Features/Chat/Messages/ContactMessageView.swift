import SwiftUI

struct ContactMessageView: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation

    let message: SwiftMessageItem

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
                    Image(systemName: "person.crop.circle.fill")
                        .resizable()
                        .foregroundStyle(.secondary)
                }
                .frame(width: 40, height: 40)
                .clipShape(Circle())

                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 4) {
                        Text(message.sharedUserFullName ?? "Contact")
                            .lineLimit(1)
                        if message.sharedUserIsVerified {
                            Image(systemName: "checkmark.seal.fill")
                                .foregroundStyle(.blue)
                        }
                        if message.sharedUserAppId != nil {
                            Image(systemName: "bolt.circle.fill")
                                .foregroundStyle(.orange)
                        }
                        if message.sharedUserMembership != nil {
                            Image(systemName: "star.circle.fill")
                                .foregroundStyle(.yellow)
                        }
                    }
                    Text(message.sharedUserIdentityNumber ?? "")
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)
                }
            }
        }
        .buttonStyle(.plain)
        .disabled(message.sharedUserId == nil)
        .help(message.sharedUserId == nil ? "Contact unavailable" : "Open profile")
    }
}
