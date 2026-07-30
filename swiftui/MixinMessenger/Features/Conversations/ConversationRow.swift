import SwiftUI

struct ConversationRow: View {
    @Environment(\.mixinTheme) private var theme
    let conversation: ConversationListData
    let keyword: String
    let currentUserID: String
    let selected: Bool
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            rowContent
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
        .background(
            rowBackground,
            in: RoundedRectangle(cornerRadius: 8, style: .continuous)
        )
        .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .padding(.horizontal, 8)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(conversation.name)
    }

    private var rowContent: some View {
        HStack(spacing: 12) {
            ConversationAvatar(conversation: conversation, size: 50)
            VStack(spacing: 7) {
                HStack(spacing: 8) {
                    HStack(spacing: 4) {
                        MessageRichText(
                            content: conversation.name,
                            baseFontSize: 16,
                            color: theme.text,
                            lineLimit: 1,
                            highlight: keyword,
                            selectable: false
                        )
                        .allowsHitTesting(false)
                        ConversationIdentityBadge(conversation: conversation)
                    }
                    Spacer(minLength: 8)
                    Text(conversation.formattedTime)
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                }
                HStack(spacing: 8) {
                    ConversationPreview(
                        conversation: conversation,
                        currentUserID: currentUserID,
                        keyword: keyword
                    )
                    Spacer(minLength: 4)
                    if conversation.mentionCount > 0 {
                        UnreadBadge(text: "@", muted: false)
                    }
                    if conversation.unseenCount > 0 {
                        UnreadBadge(
                            text: String(conversation.unseenCount),
                            muted: conversation.isMuted
                        )
                    } else {
                        if conversation.isMuted {
                            Image("ConversationMute")
                                .resizable()
                                .renderingMode(.template)
                                .scaledToFit()
                                .frame(width: 13, height: 13)
                                .foregroundStyle(theme.secondaryText)
                        }
                        if conversation.isPinned {
                            Image("ConversationPin")
                                .resizable()
                                .renderingMode(.template)
                                .scaledToFit()
                                .frame(width: 13, height: 13)
                                .foregroundStyle(theme.secondaryText)
                        }
                    }
                }
            }
        }
        .padding(.horizontal, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .frame(height: 78)
    }

    private var rowBackground: Color {
        if selected {
            return theme.listSelected
        }
        return .clear
    }
}

struct ConversationIdentityBadge: View {
    let conversation: ConversationListData

    var body: some View {
        HStack(spacing: 3) {
            if let plan = conversation.activeMembershipPlan {
                Image(systemName: plan == "prosperity" ? "crown.fill" : "star.circle.fill")
                    .foregroundStyle(plan == "advance" ? .blue : plan == "elite" ? .purple : .orange)
                    .help("Mixin \(plan.capitalized)")
            } else if conversation.isVerified {
                Image(systemName: "checkmark.seal.fill")
                    .foregroundStyle(.blue)
                    .help("Verified")
            } else if conversation.isBot {
                Image(systemName: "bolt.circle.fill")
                    .foregroundStyle(.secondary)
                    .help("Bot")
            }
            if conversation.isScam {
                Image(systemName: "exclamationmark.shield.fill")
                    .foregroundStyle(.red)
                    .help("Scam warning")
            }
        }
        .font(.system(size: 12))
        .accessibilityElement(children: .combine)
    }
}

private struct ConversationPreview: View {
    @Environment(\.mixinTheme) private var theme
    let conversation: ConversationListData
    let currentUserID: String
    let keyword: String

    var body: some View {
        HStack(spacing: 4) {
            if !conversation.hasDraft,
               conversation.showsOutgoingStatus(for: currentUserID),
               let statusSymbol = conversation.statusSymbol
            {
                if let assetName = conversation.statusAssetName {
                    Image(assetName)
                        .resizable()
                        .renderingMode(.template)
                        .scaledToFit()
                        .frame(width: 14, height: 8)
                        .foregroundStyle(
                            conversation.lastMessageStatus == "READ"
                                ? theme.accent
                                : theme.secondaryText
                        )
                } else {
                    Image(systemName: statusSymbol)
                        .font(.system(size: 11))
                        .foregroundStyle(
                            conversation.lastMessageStatus == "FAILED"
                                ? theme.destructive
                                : theme.secondaryText
                        )
                }
            }
            if let icon = conversation.previewSymbol, !conversation.hasDraft {
                Image(systemName: icon)
                    .font(.system(size: 11))
                    .foregroundStyle(theme.secondaryText)
            }
            if conversation.hasDraft {
                Text("Draft:")
                    .font(.system(size: 14))
                    .foregroundStyle(theme.destructive)
            }
            MessageRichText(
                content: conversation.rowPreview(currentUserID: currentUserID),
                baseFontSize: 14,
                color: theme.secondaryText,
                lineLimit: 1,
                highlight: keyword,
                selectable: false
            )
            .allowsHitTesting(false)
        }
        .lineLimit(1)
    }
}

private struct UnreadBadge: View {
    @Environment(\.mixinTheme) private var theme
    let text: String
    let muted: Bool

    var body: some View {
        Text(text)
            .font(.system(size: 12))
            .foregroundStyle(.white)
            .padding(.horizontal, 6)
            .frame(minWidth: 26, minHeight: 20)
            .background(muted ? theme.secondaryText : theme.accent)
            .clipShape(Capsule())
    }
}
