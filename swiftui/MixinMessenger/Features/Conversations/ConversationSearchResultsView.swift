import Foundation
import SwiftUI

private let conversationSearchPreviewLimit = 3

private enum ConversationSearchExpandedSection {
    case contacts
    case conversations
    case messages
}

struct ConversationSearchActions {
    let selectConversation: (ConversationListData) -> Void
    let selectMessage: (MessageItem, ConversationListData?) -> Void
    let selectUser: (UserProfileItem) -> Void
    let openMao: (UserProfileItem) -> Void
    let searchUser: (String) -> Void
    let openLink: (URL) -> Void
    let clear: () -> Void
}

struct ConversationSearchResultsView: View {
    @Environment(\.mixinTheme) private var theme
    let keyword: String
    let conversations: [ConversationListData]
    let state: ConversationSearchState
    let actions: ConversationSearchActions
    @State private var expandedSection: ConversationSearchExpandedSection?

    var body: some View {
        Group {
            if expandedSection == .messages {
                messageOnlyContent
            } else if isEmpty, !state.loading {
                emptyContent
            } else {
                searchContent
            }
        }
        .onChange(of: keyword) {
            expandedSection = nil
        }
    }

    private var searchContent: some View {
        AppScrollView {
            LazyVStack(spacing: 0) {
                if let maoUser = state.maoUser {
                    searchItem(
                        avatar: .user(maoUser),
                        name: maoUser.fullName,
                        description: state.mao ?? "",
                        descriptionIconURL: URL(
                            string: "https://kernel.mixin.dev/objects/fe75a8e48aeffb486df622c91bebfe4056ada7009f3151fb49e2a18340bbd615/icon"
                        ),
                        user: maoUser,
                        contentButtonTitle: maoUser.isBot ? "Open" : nil,
                        action: { actions.selectUser(maoUser) },
                        contentAction: { actions.openMao(maoUser) }
                    )
                }
                if let url = webURL {
                    searchItem(
                        name: "Open Link \(normalizedKeyword)",
                        description: nil,
                        allowsMultipleNameLines: true,
                        action: { actions.openLink(url) }
                    )
                }
                if isMixinNumber {
                    searchItem(
                        name: "Search Mixin ID: \(normalizedKeyword)",
                        description: nil,
                        allowsMultipleNameLines: true,
                        action: { actions.searchUser(normalizedKeyword) }
                    )
                }
                searchSection(
                    title: "Contact",
                    section: .contacts,
                    items: state.users
                ) { user in
                    searchItem(
                        avatar: .user(user),
                        name: user.fullName,
                        description: "Mixin ID: \(user.identityNumber)",
                        user: user,
                        action: { actions.selectUser(user) }
                    )
                }
                searchSection(
                    title: "Conversation",
                    section: .conversations,
                    items: conversations
                ) { conversation in
                    searchItem(
                        avatar: .conversation(conversation),
                        name: conversation.name,
                        description: conversationSearchDescription(conversation),
                        conversation: conversation,
                        action: { actions.selectConversation(conversation) }
                    )
                }
                if !state.messages.isEmpty {
                    searchHeader(
                        title: "Messages",
                        itemCount: state.messages.count,
                        section: .messages
                    )
                    ForEach(Array(state.messages.prefix(conversationSearchPreviewLimit)), id: \.messageId) {
                        messageItem($0)
                    }
                    Spacer().frame(height: 10)
                }
                if state.loading {
                    ProgressView()
                        .controlSize(.small)
                        .tint(theme.accent)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 24)
                }
            }
        }
    }

    private var messageOnlyContent: some View {
        VStack(spacing: 0) {
            searchHeader(
                title: "Messages",
                itemCount: state.messages.count,
                section: .messages
            )
            AppScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(state.messages, id: \.messageId) {
                        messageItem($0)
                    }
                }
            }
        }
    }

    private var emptyContent: some View {
        VStack(spacing: 4) {
            Text("No chats, contacts or messages found.")
                .font(.system(size: 14))
                .foregroundStyle(theme.secondaryText)
            Button("Clear Filter", action: actions.clear)
                .buttonStyle(.plain)
                .font(.system(size: 14))
                .foregroundStyle(theme.accent)
        }
        .padding(.horizontal, 43)
        .padding(.vertical, 86)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    @ViewBuilder
    private func searchSection<Item>(
        title: String,
        section: ConversationSearchExpandedSection,
        items: [Item],
        @ViewBuilder row: @escaping (Item) -> some View
    ) -> some View {
        if !items.isEmpty {
            searchHeader(
                title: title,
                itemCount: items.count,
                section: section
            )
            let visibleItems = expandedSection == section
                ? items
                : Array(items.prefix(conversationSearchPreviewLimit))
            ForEach(Array(visibleItems.enumerated()), id: \.offset) { _, item in
                row(item)
            }
            Spacer().frame(height: 10)
        }
    }

    private func searchHeader(
        title: String,
        itemCount: Int,
        section: ConversationSearchExpandedSection
    ) -> some View {
        HStack {
            Text(title)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)
            Spacer()
            if itemCount > conversationSearchPreviewLimit {
                Button(expandedSection == section ? "Less" : "More") {
                    expandedSection = expandedSection == section ? nil : section
                }
                .buttonStyle(.plain)
                .font(.system(size: 14))
                .foregroundStyle(theme.accent)
            }
        }
        .padding(.top, 16)
        .padding(.bottom, 10)
        .padding(.leading, 12)
        .padding(.trailing, 20)
    }

    private func messageItem(_ message: MessageItem) -> some View {
        let conversation = state.messageConversations[message.conversationId]
        return searchItem(
            avatar: conversation.map(SearchResultAvatar.conversation)
                ?? .messageSender(message),
            name: conversation?.name ?? message.senderName,
            description: messageSearchDescription(message),
            descriptionSymbol: messageSearchSymbol(message.category),
            dateMicros: message.createdAtMicros,
            highlightsName: false,
            action: { actions.selectMessage(message, conversation) }
        )
    }

    private func searchItem(
        avatar: SearchResultAvatar? = nil,
        name: String,
        description: String?,
        descriptionSymbol: String? = nil,
        descriptionIconURL: URL? = nil,
        user: UserProfileItem? = nil,
        conversation: ConversationListData? = nil,
        dateMicros: Int64? = nil,
        highlightsName: Bool = true,
        allowsMultipleNameLines: Bool = false,
        contentButtonTitle: String? = nil,
        action: @escaping () -> Void,
        contentAction: @escaping () -> Void = {}
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 0) {
                if let avatar {
                    searchAvatar(avatar)
                        .frame(width: 50, height: 50)
                    Spacer().frame(width: 12)
                }
                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: 0) {
                        HStack(spacing: 3) {
                            MessageRichText(
                                content: name,
                                baseFontSize: 16,
                                color: theme.text,
                                lineLimit: allowsMultipleNameLines ? nil : 1,
                                highlight: highlightsName ? normalizedKeyword : "",
                                selectable: false
                            )
                            .allowsHitTesting(false)
                            if let user {
                                ProfileIdentityBadge(
                                    isVerified: user.isVerified,
                                    isBot: user.isBot,
                                    membership: user.membership
                                )
                            } else if let conversation {
                                ConversationIdentityBadge(conversation: conversation)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        if let dateMicros {
                            Text(searchTime(dateMicros))
                                .font(.system(size: 12))
                                .foregroundStyle(theme.secondaryText)
                        }
                    }
                    if let description {
                        HStack(spacing: 2) {
                            if let descriptionIconURL {
                                MixinRemoteImage(url: descriptionIconURL) { image in
                                    image
                                        .resizable()
                                        .scaledToFill()
                                } placeholder: {
                                    Image(systemName: "seal.fill")
                                        .resizable()
                                        .foregroundStyle(theme.secondaryText)
                                }
                                .frame(width: 14, height: 14)
                                .clipShape(Circle())
                            } else if let descriptionSymbol {
                                Image(systemName: descriptionSymbol)
                                    .font(.system(size: 12))
                                    .foregroundStyle(theme.secondaryText)
                                    .frame(width: 14, height: 14)
                            }
                            MessageRichText(
                                content: description,
                                baseFontSize: 14,
                                color: theme.secondaryText,
                                lineLimit: 1,
                                highlight: normalizedKeyword,
                                selectable: false
                            )
                            .allowsHitTesting(false)
                        }
                    }
                }
                if let contentButtonTitle {
                    Button(contentButtonTitle, action: contentAction)
                        .buttonStyle(.borderedProminent)
                        .controlSize(.small)
                        .font(.system(size: 12))
                        .padding(.leading, 8)
                }
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 12)
            .frame(minHeight: 72)
        }
        .buttonStyle(MixinRowButtonStyle(selected: false))
        .padding(.horizontal, 6)
    }

    @ViewBuilder
    private func searchAvatar(_ avatar: SearchResultAvatar) -> some View {
        switch avatar {
        case let .user(user):
            UserAvatar(
                userID: user.userId,
                name: user.fullName,
                url: user.avatarUrl,
                size: 50
            )
        case let .conversation(conversation):
            ConversationAvatar(conversation: conversation, size: 50)
        case let .messageSender(message):
            UserAvatar(
                userID: message.senderId,
                name: message.senderName,
                url: message.senderAvatarUrl,
                size: 50
            )
        }
    }

    private var normalizedKeyword: String {
        keyword.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var webURL: URL? {
        guard let url = URL(string: normalizedKeyword),
              ["http", "https"].contains(url.scheme?.lowercased() ?? "")
        else {
            return nil
        }
        return url
    }

    private var isMixinNumber: Bool {
        normalizedKeyword.range(
            of: #"^\+?\d+$"#,
            options: .regularExpression
        ) != nil
    }

    private var isEmpty: Bool {
        state.isEmpty
            && conversations.isEmpty
            && webURL == nil
            && !isMixinNumber
    }
}

private enum SearchResultAvatar {
    case user(UserProfileItem)
    case conversation(ConversationListData)
    case messageSender(MessageItem)
}

private func conversationSearchDescription(
    _ conversation: ConversationListData
) -> String {
    guard let category = conversation.lastMessageCategory else {
        return ""
    }
    if category.contains("TEXT") {
        return conversation.lastMessage
    }
    if category.contains("IMAGE") {
        return "[Image]"
    }
    if category.contains("VIDEO") {
        return "[Video]"
    }
    if category.contains("AUDIO") {
        return "[Audio]"
    }
    if category.contains("STICKER") {
        return "[Sticker]"
    }
    if category.contains("DATA") {
        return "[File]"
    }
    return conversation.lastMessage
}

private func messageSearchDescription(_ message: MessageItem) -> String {
    if message.category.contains("TEXT") || message.category.contains("POST") {
        return message.content
    }
    if let caption = message.caption?
        .trimmingCharacters(in: .whitespacesAndNewlines),
       !caption.isEmpty
    {
        return caption
    }
    return message.mediaName ?? message.content
}

private func messageSearchSymbol(_ category: String) -> String? {
    if category.contains("IMAGE") {
        return "photo.fill"
    }
    if category.contains("VIDEO") || category.contains("LIVE") {
        return "video.fill"
    }
    if category.contains("AUDIO") {
        return "waveform"
    }
    if category.contains("STICKER") {
        return "face.smiling.fill"
    }
    if category.contains("DATA")
        || category.contains("POST")
        || category.contains("TRANSCRIPT")
    {
        return "doc.fill"
    }
    if category.contains("CONTACT") {
        return "person.crop.circle.fill"
    }
    if category.contains("SNAPSHOT") || category.contains("TRANSFER") {
        return "arrow.left.arrow.right"
    }
    if category.contains("LOCATION") {
        return "location.fill"
    }
    if category == "APP_CARD" || category == "APP_BUTTON_GROUP" {
        return "square.grid.2x2.fill"
    }
    if category.contains("RECALL") {
        return "arrow.uturn.backward"
    }
    if category.contains("CALL") {
        return "video.fill"
    }
    return nil
}

private func searchTime(_ micros: Int64) -> String {
    let date = Date(timeIntervalSince1970: Double(micros) / 1_000_000)
    return date.formatted(
        Date.FormatStyle()
            .hour(.twoDigits(amPM: .omitted))
            .minute(.twoDigits)
    )
}
