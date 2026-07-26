import AppKit
import Observation
import SwiftUI

struct ProtocolSendSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
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
            .frame(minWidth: 480, minHeight: 560)
            .navigationTitle(
                model.selectingDestination
                    ? "Choose Destination"
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
        VStack(spacing: 28) {
            Spacer(minLength: 12)
            payloadPreview
                .frame(width: 340, height: 340)
                .background(.quaternary.opacity(0.35), in: RoundedRectangle(cornerRadius: 10))
                .clipShape(RoundedRectangle(cornerRadius: 10))
            if let error = model.error {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 340)
            }
            Button(request.conversationID == nil ? "Choose Destination" : "Send") {
                if let conversationID = request.conversationID {
                    send(to: .conversationID(conversationID))
                } else {
                    model.selectingDestination = true
                    Task {
                        await model.loadDestinations(account: session.handle)
                    }
                }
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(model.sending || model.previewLoading)
            Spacer(minLength: 18)
        }
        .padding()
    }

    @ViewBuilder
    private var payloadPreview: some View {
        if model.previewLoading {
            ProgressView()
        } else {
            switch request.payload.category {
            case .text, .post:
                ScrollView {
                    Text(request.payload.content)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(28)
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
                .padding(18)
            case .sticker:
                if let sticker = model.sticker {
                    MixinRemoteImage(url: URL(string: sticker.assetUrl)) { image in
                        image.resizable().scaledToFit()
                    } placeholder: {
                        ProgressView()
                    }
                    .padding(28)
                } else {
                    ContentUnavailableView("Sticker unavailable", systemImage: "face.smiling")
                }
            case .contact:
                if let user = model.contact {
                    VStack(spacing: 14) {
                        MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                            image.resizable().scaledToFill()
                        } placeholder: {
                            Image(systemName: "person.crop.circle.fill")
                                .resizable()
                                .foregroundStyle(.secondary)
                        }
                        .frame(width: 88, height: 88)
                        .clipShape(Circle())
                        Text(user.fullName)
                            .font(.title3.weight(.semibold))
                        Text("Mixin ID \(user.identityNumber)")
                            .foregroundStyle(.secondary)
                    }
                } else {
                    ContentUnavailableView("Contact unavailable", systemImage: "person.crop.circle")
                }
            case .appCard:
                ProtocolAppCardPreview(content: request.payload.content)
                    .padding(24)
            }
        }
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
                    List(model.filteredDestinations) { destination in
                        Button {
                            send(to: destination)
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: destination.iconURL)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: destination.isGroup
                                        ? "person.3.fill"
                                        : "person.crop.circle.fill")
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 42, height: 42)
                                .clipShape(Circle())
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(destination.name)
                                        .foregroundStyle(.primary)
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
                        }
                        .buttonStyle(.plain)
                        .disabled(model.sending)
                    }
                }
            }
        }
        .searchable(text: $model.query, prompt: "Search conversations and contacts")
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
            navigation.section = .chats
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
    private(set) var sticker: SwiftStickerItem?
    private(set) var contact: SwiftUserItem?
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
            var conversations: [SwiftConversationListItem] = []
            var offset: Int64 = 0
            while true {
                let page = try await account.conversations(
                    category: "all",
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
    let subtitle: String
    let isGroup: Bool

    var id: String {
        switch kind {
        case let .conversation(id):
            "conversation:\(id)"
        case let .user(id):
            "user:\(id)"
        }
    }

    static func conversation(_ item: SwiftConversationListItem) -> Self {
        Self(
            kind: .conversation(item.conversationId),
            name: item.name,
            iconURL: item.iconUrl,
            subtitle: item.lastMessage,
            isGroup: item.category == "GROUP"
        )
    }

    static func conversationID(_ id: String) -> Self {
        Self(
            kind: .conversation(id),
            name: "",
            iconURL: "",
            subtitle: "",
            isGroup: false
        )
    }

    static func user(_ item: SwiftUserItem) -> Self {
        Self(
            kind: .user(item.userId),
            name: item.fullName,
            iconURL: item.avatarUrl,
            subtitle: item.identityNumber,
            isGroup: false
        )
    }
}

private struct ProtocolAppCardPreview: View {
    let content: String

    private var card: [String: Any] {
        guard let data = content.data(using: .utf8),
              let value = try? JSONSerialization.jsonObject(with: data)
        else {
            return [:]
        }
        return value as? [String: Any] ?? [:]
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let icon = card["icon_url"] as? String {
                MixinRemoteImage(url: URL(string: icon)) { image in
                    image.resizable().scaledToFit()
                } placeholder: {
                    ProgressView()
                }
                .frame(width: 64, height: 64)
                .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            Text(nonEmpty(card["title"] as? String) ?? "App Card")
                .font(.title3.weight(.semibold))
            if let description = nonEmpty(card["description"] as? String) {
                Text(description)
                    .foregroundStyle(.secondary)
                    .lineLimit(6)
            }
            Spacer()
            if let action = nonEmpty(card["action"] as? String) {
                Text(action)
                    .font(.caption)
                    .foregroundStyle(.tint)
                    .lineLimit(1)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }

    private func nonEmpty(_ value: String?) -> String? {
        let value = value?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return value.isEmpty ? nil : value
    }
}
