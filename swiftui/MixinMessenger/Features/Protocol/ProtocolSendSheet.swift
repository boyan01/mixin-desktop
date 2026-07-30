import AppKit
import Observation
import SwiftUI

struct ProtocolSendSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @State private var model = ProtocolSendModel()

    let request: ProtocolSendRequest

    var body: some View {
        NavigationStack {
            Group {
                if model.selectingDestination {
                    destinationList
                } else {
                    payloadConfirmation
                }
            }
            .frame(width: 480)
            .navigationTitle(
                model.selectingDestination
                    ? "Forward"
                    : "Share \(request.payload.category.displayName)"
            )
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(model.selectingDestination ? "Back" : "Cancel") {
                        if model.selectingDestination {
                            model.selectingDestination = false
                        } else {
                            dismiss()
                        }
                    }
                }
            }
        }
        .task {
            await model.loadPreview(for: request.payload, account: session.handle)
        }
    }

    private var payloadConfirmation: some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 12)
            payloadPreview
                .padding(
                    request.payload.category == .appCard
                        ? EdgeInsets()
                        : EdgeInsets(
                            top: 34,
                            leading: 34,
                            bottom: 34,
                            trailing: 34
                        )
                )
                .frame(width: 340, height: 340)
                .background(previewBackground, in: RoundedRectangle(cornerRadius: 8))
                .clipShape(RoundedRectangle(cornerRadius: 8))
            if let error = model.error {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 340)
                    .padding(.top, 12)
            }
            Spacer().frame(height: 54)
            Button(request.conversationID == nil ? "Forward" : "Send") {
                if let conversationID = request.conversationID {
                    send(to: .conversationID(conversationID))
                } else {
                    model.selectingDestination = true
                    Task {
                        await model.loadDestinations(account: session.handle)
                    }
                }
            }
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(.white)
            .padding(.vertical, 8)
            .padding(.horizontal, 16)
            .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
            .disabled(model.sending || model.previewLoading)
            Spacer().frame(height: 56)
        }
        .frame(maxWidth: .infinity)
    }

    private var previewBackground: Color {
        colorScheme == .dark
            ? Color.white.opacity(0.08)
            : Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255)
    }

    @ViewBuilder
    private var payloadPreview: some View {
        if model.previewLoading {
            ProgressView()
        } else {
            switch request.payload.category {
            case .text:
                AppScrollView {
                    Text(request.payload.content)
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                        .textSelection(.enabled)
                        .padding(8)
                        .background(
                            incomingBubbleColor,
                            in: RoundedRectangle(cornerRadius: 8)
                        )
                }
            case .image:
                MixinAsyncImage(url: URL(string: request.payload.content)) { phase in
                    switch phase {
                    case let .success(image):
                        image.resizable().scaledToFit()
                    case .failure:
                        ContentUnavailableView(
                            "Image unavailable",
                            systemImage: "photo.badge.exclamationmark"
                        )
                    default:
                        ProgressView()
                    }
                }
            case .sticker:
                if let sticker = model.sticker {
                    MixinRemoteImage(url: URL(string: sticker.assetUrl)) { image in
                        image.resizable().scaledToFit()
                    } placeholder: {
                        ProgressView()
                    }
                    .padding(45)
                } else {
                    ContentUnavailableView("Sticker unavailable", systemImage: "face.smiling")
                }
            case .contact:
                if let user = model.contact {
                    HStack(spacing: 8) {
                        UserAvatar(
                            userID: user.userId,
                            name: user.fullName,
                            url: user.avatarUrl,
                            size: 40
                        )
                        VStack(alignment: .leading, spacing: 0) {
                            HStack(spacing: 3) {
                                Text(user.fullName)
                                    .font(.system(size: 16))
                                    .foregroundStyle(theme.text)
                                    .lineLimit(1)
                                ProfileIdentityBadge(
                                    isVerified: user.isVerified,
                                    isBot: user.isBot,
                                    membership: nil
                                )
                            }
                            Text(user.identityNumber)
                                .font(.system(size: 14))
                                .foregroundStyle(theme.secondaryText)
                                .lineLimit(1)
                        }
                    }
                    .padding(8)
                    .background(
                        incomingBubbleColor,
                        in: RoundedRectangle(cornerRadius: 8)
                    )
                } else {
                    ContentUnavailableView("Contact unavailable", systemImage: "person.crop.circle")
                }
            case .post:
                ProtocolPostPreview(content: request.payload.content)
            case .appCard:
                ProtocolAppCardPreview(content: request.payload.content)
            }
        }
    }

    private var incomingBubbleColor: Color {
        colorScheme == .dark
            ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
            : .white
    }

    private var destinationList: some View {
        VStack(spacing: 0) {
            if let error = model.error {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .padding(.horizontal)
                    .padding(.vertical, 8)
                    .frame(maxWidth: .infinity)
                    .background(.red.opacity(0.08))
            }
            Group {
                switch model.destinationState {
                case .loading:
                    ProgressView()
                case let .failed(message):
                    ContentUnavailableView(
                        "Unable to load destinations",
                        systemImage: "exclamationmark.triangle",
                        description: Text(message)
                    )
                case .ready where model.filteredDestinations.isEmpty:
                    ContentUnavailableView.search(text: model.query)
                case .ready:
                    AppListView(model.filteredDestinations) { destination in
                        Button {
                            send(to: destination)
                        } label: {
                            HStack(spacing: 0) {
                                Group {
                                    if destination.isGroup {
                                        GroupAvatarPuzzle(avatars: Array(destination.groupAvatars.prefix(4)))
                                    } else {
                                        MixinRemoteImage(url: URL(string: destination.iconURL)) { image in
                                            image.resizable().scaledToFill()
                                        } placeholder: {
                                            AvatarPlaceholder(userID: destination.ownerID, name: destination.name)
                                        }
                                    }
                                }
                                .frame(width: 50, height: 50)
                                .clipShape(Circle())
                                Spacer().frame(width: 16)
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(destination.name)
                                        .font(.system(size: 16))
                                        .foregroundStyle(theme.text)
                                    ProfileIdentityBadge(isVerified: destination.isVerified, isBot: destination.isBot, membership: destination.membership)
                                        .padding(.horizontal, 4)
                                    if !destination.subtitle.isEmpty {
                                        Text(destination.subtitle)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                            .lineLimit(1)
                                    }
                                }
                                Spacer()
                                if model.sendingDestinationID == destination.id {
                                    ProgressView()
                                        .controlSize(.small)
                                }
                            }
                            .frame(height: 70)
                            .padding(.leading, 14)
                            .padding(.trailing, 10)
                        }
                        .buttonStyle(MixinRowButtonStyle(selected: false))
                        .disabled(model.sending)
                    }
                }
            }
        }
        .searchable(text: $model.query, prompt: "Search conversations and contacts")
        .onChange(of: model.query) {
            if model.query.count > 200 {
                model.query = String(model.query.prefix(200))
            }
        }
    }

    private func send(to destination: ProtocolSendDestination) {
        Task {
            guard let outcome = await model.send(
                request.payload,
                destination: destination,
                account: session.handle
            ) else {
                return
            }
            navigation.selectConversation(outcome.conversationID, name: outcome.name)
            dismiss()
        }
    }
}

@MainActor
enum ProtocolSendService {
    static func send(
        _ payload: ProtocolSendPayload,
        to conversationID: String,
        account: SwiftAccountHandle
    ) async throws {
        switch payload.category {
        case .text:
            _ = try await account.sendText(
                conversationId: conversationID,
                content: payload.content,
                quoteMessageId: nil,
                silent: false
            )
        case .post:
            _ = try await account.sendPost(
                conversationId: conversationID,
                content: payload.content
            )
        case .contact:
            _ = try await account.sendContact(
                conversationId: conversationID,
                sharedUserId: payload.content,
                quoteMessageId: nil,
                silent: false
            )
        case .sticker:
            _ = try await account.stickerDetail(stickerId: payload.content)
            _ = try await account.sendSticker(
                conversationId: conversationID,
                stickerId: payload.content
            )
        case .appCard:
            _ = try await account.sendAppCard(
                conversationId: conversationID,
                content: payload.content
            )
        case .image:
            try await sendImage(
                at: payload.content,
                to: conversationID,
                account: account
            )
        }
    }

    private static func sendImage(
        at source: String,
        to conversationID: String,
        account: SwiftAccountHandle
    ) async throws {
        guard let url = URL(string: source),
              let scheme = url.scheme?.lowercased(),
              scheme == "https" || scheme == "http"
        else {
            throw URLError(.badURL)
        }
        let (data, response) = try await URLSession.shared.data(from: url)
        if let response = response as? HTTPURLResponse,
           !(200 ... 299).contains(response.statusCode)
        {
            throw URLError(.badServerResponse)
        }
        guard let image = NSImage(data: data),
              let representation = image.representations.first
        else {
            throw CocoaError(.fileReadCorruptFile)
        }

        let temporaryURL = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
        try data.write(to: temporaryURL, options: .atomic)
        defer {
            try? FileManager.default.removeItem(at: temporaryURL)
        }
        _ = try await account.sendAttachment(
            conversationId: conversationID,
            path: temporaryURL.path,
            kind: "IMAGE",
            mimeType: response.mimeType ?? "image/jpeg",
            name: nil,
            width: Int32(exactly: representation.pixelsWide),
            height: Int32(exactly: representation.pixelsHigh),
            durationMillis: nil,
            thumbnail: nil,
            caption: nil,
            quoteMessageId: nil,
            silent: false
        )
    }
}

@MainActor
@Observable
final class ProtocolSendModel {
    enum DestinationState {
        case loading
        case ready
        case failed(String)
    }

    private(set) var previewLoading = false
    private(set) var sticker: StickerItem?
    private(set) var contact: UserProfileItem?
    private(set) var destinationState = DestinationState.loading
    private(set) var destinations: [ProtocolSendDestination] = []
    private(set) var sending = false
    private(set) var sendingDestinationID: String?
    var selectingDestination = false
    var query = ""
    var error: String?

    var filteredDestinations: [ProtocolSendDestination] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return destinations
        }
        return destinations.filter {
            $0.name.localizedCaseInsensitiveContains(query)
                || $0.subtitle.localizedCaseInsensitiveContains(query)
        }
    }

    func loadPreview(
        for payload: ProtocolSendPayload,
        account: SwiftAccountHandle
    ) async {
        previewLoading = true
        defer { previewLoading = false }
        do {
            switch payload.category {
            case .sticker:
                sticker = try await account.stickerDetail(stickerId: payload.content).sticker
            case .contact:
                contact = try await account.userProfile(userId: payload.content)
                if contact == nil {
                    throw CocoaError(.fileNoSuchFile)
                }
            default:
                break
            }
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    func loadDestinations(account: SwiftAccountHandle) async {
        guard destinations.isEmpty else {
            destinationState = .ready
            return
        }
        destinationState = .loading
        do {
            var conversations: [ConversationListData] = []
            var offset: Int64 = 0
            while true {
                let page = try await account.conversations(
                    category: "chats",
                    circleId: nil,
                    keyword: "",
                    unseenOnly: false,
                    limit: 200,
                    offset: offset
                )
                conversations.append(contentsOf: page)
                if page.count < 200 {
                    break
                }
                offset += Int64(page.count)
            }
            let existingOwners = Set(conversations.map(\.ownerId))
            let users = try await account.selectableUsers()
                .filter { !existingOwners.contains($0.userId) }
            destinations = conversations.map(ProtocolSendDestination.conversation)
                + users.map(ProtocolSendDestination.user)
            destinationState = .ready
        } catch {
            destinationState = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func send(
        _ payload: ProtocolSendPayload,
        destination: ProtocolSendDestination,
        account: SwiftAccountHandle
    ) async -> ProtocolSendOutcome? {
        guard !sending else {
            return nil
        }
        sending = true
        sendingDestinationID = destination.id
        error = nil
        defer {
            sending = false
            sendingDestinationID = nil
        }
        do {
            let conversationID: String
            let name: String?
            switch destination.kind {
            case let .conversation(id):
                let item = try await account.conversationItemsByIds(
                    conversationIds: [id]
                ).first
                guard let item else {
                    throw CocoaError(.fileNoSuchFile)
                }
                conversationID = item.conversationId
                name = item.name
            case let .user(id):
                conversationID = try await account.openUserConversation(userId: id)
                name = destination.name
            }
            try await ProtocolSendService.send(payload, to: conversationID, account: account)
            return ProtocolSendOutcome(conversationID: conversationID, name: name)
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
            return nil
        }
    }
}

struct ProtocolSendOutcome {
    let conversationID: String
    let name: String?
}

struct ProtocolSendDestination: Identifiable {
    enum Kind {
        case conversation(String)
        case user(String)
    }

    let kind: Kind
    let name: String
    let iconURL: String
    let ownerID: String
    let subtitle: String
    let isGroup: Bool
    let isVerified: Bool
    let isBot: Bool
    let membership: String?
    let groupAvatars: [GroupAvatar]

    var id: String {
        switch kind {
        case let .conversation(id):
            "conversation:\(id)"
        case let .user(id):
            "user:\(id)"
        }
    }

    static func conversation(_ item: ConversationListData) -> Self {
        Self(
            kind: .conversation(item.conversationId),
            name: item.name,
            iconURL: item.avatarUrl,
            ownerID: item.ownerId,
            subtitle: item.lastMessage,
            isGroup: item.category == "GROUP"
            , isVerified: item.isVerified, isBot: item.isBot, membership: item.membership, groupAvatars: item.groupAvatars
        )
    }

    static func conversationID(_ id: String) -> Self {
        Self(
            kind: .conversation(id),
            name: "",
            iconURL: "",
            ownerID: "",
            subtitle: "",
            isGroup: false
            , isVerified: false, isBot: false, membership: nil, groupAvatars: []
        )
    }

    static func user(_ item: UserProfileItem) -> Self {
        Self(
            kind: .user(item.userId),
            name: item.fullName,
            iconURL: item.avatarUrl,
            ownerID: item.userId,
            subtitle: item.identityNumber,
            isGroup: false
            , isVerified: item.isVerified, isBot: item.isBot, membership: item.membership, groupAvatars: []
        )
    }
}

private struct ProtocolPostPreview: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    let content: String

    var body: some View {
        ZStack(alignment: .topTrailing) {
            AppScrollView {
                Text(attributedContent)
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            Image("PostDetail")
                .resizable()
                .frame(width: 20, height: 20)
                .background(.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 8))
        }
        .padding(8)
        .frame(minWidth: 128, maxHeight: 400)
        .background(incomingBubbleColor, in: RoundedRectangle(cornerRadius: 8))
        .padding(36)
    }

    private var attributedContent: AttributedString {
        (try? AttributedString(
            markdown: content,
            options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        )) ?? AttributedString(content)
    }

    private var incomingBubbleColor: Color {
        colorScheme == .dark
            ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
            : .white
    }
}

private struct ProtocolAppCardPreview: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    let content: String

    private var card: AppCardContent? {
        try? JSONDecoder().decode(
            AppCardContent.self,
            from: Data(content.utf8)
        )
    }

    @ViewBuilder
    var body: some View {
        if let card {
            if card.action.isEmpty {
                actionsCard(card)
            } else {
                compactCard(card)
            }
        } else {
            UnsupportedMessageView(category: "APP_CARD")
        }
    }

    private func compactCard(_ card: AppCardContent) -> some View {
        HStack(spacing: 8) {
            MixinRemoteImage(url: URL(string: card.iconURL)) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Rectangle().fill(theme.background)
            }
            .frame(width: 40, height: 40)
            .clipShape(RoundedRectangle(cornerRadius: 4))

            VStack(alignment: .leading, spacing: 0) {
                Text(card.title)
                    .font(.system(size: 14))
                    .foregroundStyle(theme.text)
                    .lineLimit(1)
                Text(
                    card.description.split(
                        separator: "\n",
                        omittingEmptySubsequences: false
                    ).first.map(String.init) ?? ""
                )
                .font(.system(size: 12))
                .foregroundStyle(theme.secondaryText)
                .lineLimit(1)
            }
        }
        .padding(34)
        .background(incomingBubbleColor, in: RoundedRectangle(cornerRadius: 8))
    }

    private func actionsCard(_ card: AppCardContent) -> some View {
        VStack(spacing: 8) {
            VStack(alignment: .leading, spacing: 0) {
                if let coverURL = URL(string: card.resolvedCoverURL),
                   !card.resolvedCoverURL.isEmpty
                {
                    MixinRemoteImage(url: coverURL) { image in
                        image.resizable().scaledToFill()
                    } placeholder: {
                        Rectangle().fill(theme.background)
                    }
                    .frame(width: 320, height: coverHeight(card))
                    .clipped()
                } else {
                    Spacer().frame(height: 10)
                }
                Text(card.title)
                    .font(.system(size: 16, weight: .bold))
                    .foregroundStyle(theme.text)
                    .padding(.horizontal, 12)
                    .padding(.top, card.resolvedCoverURL.isEmpty ? 0 : 10)
                Text(card.description)
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                    .padding(.horizontal, 12)
                    .padding(.top, 8)
                Spacer().frame(height: 10)
            }
            .frame(width: 320, alignment: .leading)
            .background(incomingBubbleColor, in: RoundedRectangle(cornerRadius: 8))
            .clipShape(RoundedRectangle(cornerRadius: 8))

            AppActionButtonLayout {
                ForEach(Array(card.actions.enumerated()), id: \.offset) { _, action in
                    Text(action.label)
                        .font(.system(size: 14))
                        .foregroundStyle(Color(hex: action.color) ?? theme.accent)
                        .padding(.horizontal, 14)
                        .frame(height: 30)
                        .background(theme.primary)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                }
            }
            .frame(width: 320)
        }
    }

    private func coverHeight(_ card: AppCardContent) -> CGFloat {
        guard let cover = card.cover, cover.width > 0, cover.height > 0 else {
            return 320
        }
        return 320 / max(CGFloat(cover.width) / CGFloat(cover.height), 1.5)
    }

    private var incomingBubbleColor: Color {
        colorScheme == .dark
            ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
            : .white
    }
}
