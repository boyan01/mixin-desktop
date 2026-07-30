import SwiftUI

struct ConversationAvatar: View {
    @Environment(\.mixinTheme) private var theme

    let conversation: ConversationListData?
    let size: CGFloat

    var body: some View {
        Group {
            if let conversation,
               conversation.category == "GROUP"
            {
                GroupAvatarPuzzle(
                    avatars: Array(conversation.groupAvatars.prefix(4))
                )
            } else if let conversation {
                ConversationAvatarTile(
                    userID: conversation.ownerId,
                    name: conversation.name,
                    url: conversation.avatarUrl
                )
            } else {
                theme.listSelected
                    .overlay {
                        Image(systemName: "bubble.left.fill")
                            .font(.system(size: size * 0.4))
                            .foregroundStyle(theme.secondaryText)
                    }
            }
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
    }
}

struct UserAvatar: View {
    let userID: String
    let name: String
    let url: String
    let size: CGFloat

    var body: some View {
        ConversationAvatarTile(
            userID: userID,
            name: name,
            url: url
        )
        .frame(width: size, height: size)
        .clipShape(Circle())
    }
}

struct GroupAvatarPuzzle: View {
    let avatars: [GroupAvatar]

    var body: some View {
        GeometryReader { proxy in
            let columnWidth = proxy.size.width / 2
            let rowHeight = proxy.size.height / 2
            switch avatars.count {
            case 0:
                Color.clear
            case 1:
                avatarTile(avatars[0])
            case 2:
                HStack(spacing: 0) {
                    ForEach(Array(avatars.enumerated()), id: \.offset) { _, avatar in
                        avatarTile(avatar)
                            .frame(width: columnWidth, height: proxy.size.height)
                    }
                }
            case 3:
                HStack(spacing: 0) {
                    avatarTile(avatars[0])
                        .frame(width: columnWidth, height: proxy.size.height)
                    VStack(spacing: 0) {
                        avatarTile(avatars[1])
                            .frame(width: columnWidth, height: rowHeight)
                        avatarTile(avatars[2])
                            .frame(width: columnWidth, height: rowHeight)
                    }
                }
            default:
                HStack(spacing: 0) {
                    VStack(spacing: 0) {
                        avatarTile(avatars[0])
                            .frame(width: columnWidth, height: rowHeight)
                        avatarTile(avatars[1])
                            .frame(width: columnWidth, height: rowHeight)
                    }
                    VStack(spacing: 0) {
                        avatarTile(avatars[2])
                            .frame(width: columnWidth, height: rowHeight)
                        avatarTile(avatars[3])
                            .frame(width: columnWidth, height: rowHeight)
                    }
                }
            }
        }
    }

    private func avatarTile(_ avatar: GroupAvatar) -> some View {
        ConversationAvatarTile(
            userID: avatar.userId,
            name: avatar.name,
            url: avatar.avatarUrl
        )
    }
}

struct ConversationAvatarTile: View {
    let userID: String
    let name: String
    let url: String

    var body: some View {
        MixinRemoteImage(url: URL(string: url)) { image in
            image
                .resizable()
                .scaledToFill()
        } placeholder: {
            AvatarPlaceholder(userID: userID, name: name)
        }
        .clipped()
    }
}

struct AvatarPlaceholder: View {
    let userID: String
    let name: String

    var body: some View {
        AvatarPalette.color(for: userID)
            .overlay {
                Text(name.first.map { String($0).uppercased() } ?? "")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(.white)
            }
    }
}

enum AvatarPalette {
    private static let colors: [Color] = [
        color(0xFFD659), color(0xFFC168), color(0xF58268), color(0xF4979C),
        color(0xEC7F87), color(0xFF78CB), color(0xC377E0), color(0x8BAAFF),
        color(0x78DCFA), color(0x88E5B9), color(0xBFF199), color(0xC5E1A5),
        color(0xCD907D), color(0xBE938E), color(0xB68F91), color(0xBC987B),
        color(0xA69E8E), color(0xD4C99E), color(0x93C2E6), color(0x92C3D9),
        color(0x8FBFC5), color(0x80CBC4), color(0xA4DBDB), color(0xB2C8BD),
        color(0xF7C8C9), color(0xDCC6E4), color(0xBABAE8), color(0xBABCD5),
        color(0xAD98DA), color(0xC097D9),
    ]

    static func color(for userID: String) -> Color {
        colors[index(for: userID)]
    }

    private static func index(for userID: String) -> Int {
        Int(hash(for: userID).magnitude % UInt64(colors.count))
    }

    private static func hash(for userID: String) -> Int64 {
        let components = userID.trimmingCharacters(in: .whitespacesAndNewlines)
            .split(separator: "-")
        guard components.count == 5,
              let first = UInt64(components[0], radix: 16),
              let second = UInt64(components[1], radix: 16),
              let third = UInt64(components[2], radix: 16),
              let fourth = UInt64(components[3], radix: 16),
              let fifth = UInt64(components[4], radix: 16)
        else {
            return userID.utf8.reduce(0) { hash, byte in
                hash &* 31 &+ Int64(byte)
            }
        }

        let mostSignificantBits = first << 32 | second << 16 | third
        let leastSignificantBits = fourth << 48 | fifth
        let hilo = mostSignificantBits ^ leastSignificantBits
        return (Int64(bitPattern: hilo) >> 32)
            ^ Int64(Int32(truncatingIfNeeded: hilo))
    }

    private static func color(_ hex: UInt32) -> Color {
        Color(
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
    }
}
