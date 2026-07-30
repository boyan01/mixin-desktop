import AppKit
import Observation

struct ProtocolNotice: Identifiable {
    let id = UUID()
    let title: String
    let message: String
}

enum ConversationCreationKind: String {
    case conversation
    case group
    case circle
}

struct ConversationCreationRequest: Identifiable {
    let id = UUID()
    let kind: ConversationCreationKind
}

enum ConversationCommand {
    case mute
    case delete
    case pin
}

struct ConversationCommandRequest {
    let command: ConversationCommand
    let revision: Int
}

struct MessageJumpRequest {
    let conversationID: String
    let messageID: String
    let revision: Int
}

struct ChatViewportPosition: Equatable {
    let messageID: String
    let offset: CGFloat
}

enum ChatInspectorRoute: Hashable {
    case circles
    case participants
    case pinnedMessages
    case sharedContent
    case sharedApps
    case groupsInCommon(userID: String)
    case disappearingMessages
    case userProfile(userID: String)
}

@MainActor
@Observable
final class HomeNavigationModel {
    var page: HomePage = .chats
    var section: HomeSection = .chats {
        didSet {
            if oldValue != section {
                clearConversationSelection()
            }
        }
    }
    var routePath: [HomeRoute] = [] {
        didSet {
            if !routePath.contains(.chatInfo) {
                infoPresented = false
                inspectorPath = []
            }
        }
    }
    private(set) var settingsDestination: SettingsDestination = .profile
    var selectedConversationID: String?
    private(set) var selectedConversationName: String?
    private(set) var selectedConversationDraft = ""
    private(set) var selectedConversation: ConversationListData?
    private(set) var recentConversationIDs: [String] = []
    var infoPresented = false
    var inspectorPath: [ChatInspectorRoute] = []
    var protocolNotice: ProtocolNotice?
    var protocolPresentation: ProtocolPresentation?
    var protocolSendRequest: ProtocolSendRequest?
    var creationRequest: ConversationCreationRequest?
    var commandPalettePresented = false
    private(set) var searchRequest = 0
    private(set) var messageSearchRequest = 0
    private(set) var messageJumpRequest: MessageJumpRequest?
    private var messageJumpRevision = 0
    private var conversationIDs: [String] = []
    private var conversationNames: [String: String] = [:]
    private var conversationDrafts: [String: String] = [:]
    private var conversationItems: [String: ConversationListData] = [:]
    private var chatViewportPositions: [String: ChatViewportPosition] = [:]
    private(set) var conversationCommandRequest: ConversationCommandRequest?
    private var conversationCommandRevision = 0

    func selectConversation(
        _ conversationID: String,
        name: String? = nil,
        draft: String? = nil
    ) {
        page = .chats
        if selectedConversationID != conversationID {
            inspectorPath = []
            infoPresented = false
        }
        selectedConversationID = conversationID
        recordRecentConversation(conversationID)
        selectedConversationName = name ?? conversationNames[conversationID]
        selectedConversationDraft = draft ?? conversationDrafts[conversationID] ?? ""
        selectedConversation = conversationItems[conversationID]
        routePath = [.chat(conversationID)]
    }

    func selectConversation(_ conversation: ConversationListData) {
        page = .chats
        if selectedConversationID != conversation.conversationId {
            inspectorPath = []
            infoPresented = false
        }
        selectedConversationID = conversation.conversationId
        recordRecentConversation(conversation.conversationId)
        selectedConversationName = conversation.name
        selectedConversationDraft = conversation.draft
        selectedConversation = conversation
        routePath = [.chat(conversation.conversationId)]
    }

    func showSettings() {
        page = .settings
        routePath = []
    }

    func showChats(section: HomeSection = .chats) {
        page = .chats
        self.section = section
        routePath = []
    }

    func showSettingsDestination(_ destination: SettingsDestination) {
        page = .settings
        settingsDestination = destination
        routePath = [.settings(destination)]
    }

    func focusConversationSearch() {
        searchRequest += 1
    }

    func showCommandPalette() {
        commandPalettePresented = true
    }

    func focusMessageSearch() {
        guard selectedConversationID != nil else {
            return
        }
        messageSearchRequest += 1
    }

    func locateMessage(
        conversationID: String,
        messageID: String,
        conversationName: String? = nil
    ) {
        selectConversation(conversationID, name: conversationName)
        infoPresented = false
        inspectorPath = []
        messageJumpRevision += 1
        messageJumpRequest = MessageJumpRequest(
            conversationID: conversationID,
            messageID: messageID,
            revision: messageJumpRevision
        )
    }

    func consumeMessageJump(revision: Int) {
        guard messageJumpRequest?.revision == revision else {
            return
        }
        messageJumpRequest = nil
    }

    func chatViewportPosition(
        conversationID: String
    ) -> ChatViewportPosition? {
        chatViewportPositions[conversationID]
    }

    func saveChatViewportPosition(
        _ position: ChatViewportPosition?,
        conversationID: String
    ) {
        chatViewportPositions[conversationID] = position
    }

    func toggleConversationInfo() {
        guard selectedConversation != nil else {
            return
        }
        infoPresented.toggle()
        if infoPresented {
            presentChatInfoRoute()
        } else {
            inspectorPath = []
            routePath.removeAll { $0 == .chatInfo }
        }
    }

    func openInspector(_ route: ChatInspectorRoute? = nil) {
        guard selectedConversation != nil else {
            return
        }
        infoPresented = true
        inspectorPath = route.map { [$0] } ?? []
        presentChatInfoRoute()
    }

    func pushInspector(_ route: ChatInspectorRoute) {
        guard infoPresented else {
            openInspector(route)
            return
        }
        inspectorPath.append(route)
    }

    func showConversationInfo(
        conversationID: String,
        name: String? = nil
    ) {
        selectConversation(conversationID, name: name)
        openInspector()
    }

    func showCreation(_ kind: ConversationCreationKind) {
        creationRequest = ConversationCreationRequest(kind: kind)
    }

    func dismissCreation() {
        creationRequest = nil
    }

    func requestConversationCommand(_ command: ConversationCommand) {
        guard selectedConversation != nil else {
            return
        }
        conversationCommandRevision += 1
        conversationCommandRequest = ConversationCommandRequest(
            command: command,
            revision: conversationCommandRevision
        )
    }

    func conversationDeleted(_ conversationID: String) {
        chatViewportPositions[conversationID] = nil
        guard selectedConversationID == conversationID else {
            return
        }
        selectedConversationID = nil
        selectedConversationName = nil
        selectedConversationDraft = ""
        selectedConversation = nil
        infoPresented = false
        inspectorPath = []
        routePath = []
    }

    func clearConversationSelection() {
        selectedConversationID = nil
        selectedConversationName = nil
        selectedConversationDraft = ""
        selectedConversation = nil
        infoPresented = false
        inspectorPath = []
        routePath = []
    }

    func updateConversationOrder(_ conversations: [ConversationListData]) {
        let updatedIDs = conversations.map(\.conversationId)
        let updatedNames = Dictionary(
            uniqueKeysWithValues: conversations.map {
                ($0.conversationId, $0.name)
            }
        )
        let updatedDrafts = Dictionary(
            uniqueKeysWithValues: conversations.map {
                ($0.conversationId, $0.draft)
            }
        )
        let updatedItems = Dictionary(
            uniqueKeysWithValues: conversations.map {
                ($0.conversationId, $0)
            }
        )

        if conversationIDs != updatedIDs {
            conversationIDs = updatedIDs
        }
        if conversationNames != updatedNames {
            conversationNames = updatedNames
        }
        if conversationDrafts != updatedDrafts {
            conversationDrafts = updatedDrafts
        }
        if conversationItems != updatedItems {
            conversationItems = updatedItems
        }

        if let selectedConversationID {
            let updatedName = updatedNames[selectedConversationID]
            let updatedDraft = updatedDrafts[selectedConversationID] ?? ""
            let updatedConversation = updatedItems[selectedConversationID]
            if selectedConversationName != updatedName {
                selectedConversationName = updatedName
            }
            if selectedConversationDraft != updatedDraft {
                selectedConversationDraft = updatedDraft
            }
            if selectedConversation != updatedConversation {
                selectedConversation = updatedConversation
            }
        }
    }

    func selectAdjacentConversation(forward: Bool) {
        guard let selectedConversationID,
              let index = conversationIDs.firstIndex(of: selectedConversationID)
        else {
            return
        }
        let nextIndex = forward ? index + 1 : index - 1
        guard conversationIDs.indices.contains(nextIndex) else {
            return
        }
        self.selectedConversationID = conversationIDs[nextIndex]
        recordRecentConversation(conversationIDs[nextIndex])
        selectedConversationName = conversationNames[conversationIDs[nextIndex]]
        selectedConversationDraft = conversationDrafts[conversationIDs[nextIndex]] ?? ""
        selectedConversation = conversationItems[conversationIDs[nextIndex]]
        infoPresented = false
        inspectorPath = []
        routePath = [.chat(conversationIDs[nextIndex])]
    }

    private func recordRecentConversation(_ conversationID: String) {
        recentConversationIDs.removeAll { $0 == conversationID }
        recentConversationIDs.insert(conversationID, at: 0)
        if recentConversationIDs.count > 5 {
            recentConversationIDs.removeLast(
                recentConversationIDs.count - 5
            )
        }
    }

    private func presentChatInfoRoute() {
        guard let selectedConversationID else {
            return
        }
        routePath = [.chat(selectedConversationID), .chatInfo]
    }

    func open(_ url: URL, account: SwiftAccountHandle) async {
        switch MixinDeepLink(url: url) {
        case let .conversation(id, start):
            selectConversation(id)
            if let start {
                do {
                    _ = try await account.sendText(
                        conversationId: id,
                        content: start,
                        quoteMessageId: nil,
                        silent: false
                    )
                } catch {
                    AppLogger.error(
                        "Open conversation link start message failed: conversation_id=\(id)",
                        error: error
                    )
                    protocolNotice = ProtocolNotice(
                        title: "Unable to Send Message",
                        message: MixinErrorPresenter.message(for: error)
                    )
                }
            }
        case let .user(identityNumber):
            do {
                let identityMatch = try await account.usersByIdentityNumbers(
                    identityNumbers: [identityNumber]
                ).first
                let user = if let identityMatch {
                    identityMatch
                } else {
                    try await account.userProfile(userId: identityNumber)
                }
                guard let user else {
                    protocolNotice = ProtocolNotice(
                        title: "User Not Found",
                        message: "No Mixin user has identity number \(identityNumber)."
                    )
                    return
                }
                let conversationID = try await account.openUserConversation(
                    userId: user.userId
                )
                selectConversation(conversationID, name: user.fullName)
                openInspector()
            } catch {
                AppLogger.error(
                    "Open user link failed: identity_number=\(identityNumber)",
                    error: error
                )
                protocolNotice = ProtocolNotice(
                    title: "Unable to Open User",
                    message: MixinErrorPresenter.message(for: error)
                )
            }
        case let .app(id, open, source):
            do {
                guard let app = try await account.userProfile(userId: id) else {
                    protocolNotice = ProtocolNotice(
                        title: "Bot Not Found",
                        message: "The requested Mixin app is unavailable."
                    )
                    return
                }
                if !open {
                    let conversationID = try await account.openUserConversation(
                        userId: app.userId
                    )
                    selectConversation(conversationID, name: app.fullName)
                    openInspector()
                    return
                }
                guard let home = try await account.botHomeUri(appId: id),
                      var components = URLComponents(string: home)
                else {
                    protocolNotice = ProtocolNotice(
                        title: "Bot Not Found",
                        message: "This bot does not publish a homepage."
                    )
                    return
                }
                let passthrough = URLComponents(
                    url: source,
                    resolvingAgainstBaseURL: false
                )?.queryItems?.filter { $0.name != "action" } ?? []
                var query = components.queryItems ?? []
                let replaced = Set(passthrough.map(\.name))
                query.removeAll { replaced.contains($0.name) }
                query.append(contentsOf: passthrough)
                components.queryItems = query
                guard let url = components.url else {
                    throw URLError(.badURL)
                }
                BotWebViewWindow.open(
                    url: url,
                    title: app.fullName,
                    conversationID: "",
                    currency: account.profile().fiatCurrency
                )
            } catch {
                AppLogger.error(
                    "Open bot link failed: app_id=\(id)",
                    error: error
                )
                protocolNotice = ProtocolNotice(
                    title: "Unable to Open Bot",
                    message: MixinErrorPresenter.message(for: error)
                )
            }
        case let .code(value, source):
            do {
                let result = try await account.resolveCode(code: value)
                if result.kind == "user", let userID = result.userId,
                   let user = try await account.userProfile(userId: userID)
                {
                    let conversationID = try await account.openUserConversation(
                        userId: user.userId
                    )
                    selectConversation(conversationID, name: user.fullName)
                    openInspector()
                } else if result.kind == "conversation",
                          result.conversationId != nil
                {
                    protocolPresentation = .group(result, value)
                } else if result.kind == "payment" || result.kind == "multisig_request" {
                    protocolPresentation = .payment(result, source)
                } else {
                    protocolNotice = ProtocolNotice(
                        title: "Link Not Supported",
                        message: "This code resolves to \(result.kind), which is handled by another workflow."
                    )
                }
            } catch {
                AppLogger.error("Resolve code link failed", error: error)
                protocolNotice = ProtocolNotice(
                    title: "Unable to Resolve Code",
                    message: MixinErrorPresenter.message(for: error)
                )
            }
        case let .snapshot(traceID):
            do {
                let snapshot = try await account.snapshotByTrace(traceId: traceID)
                protocolPresentation = .snapshot(snapshot)
            } catch {
                AppLogger.error(
                    "Open snapshot link failed: trace_id=\(traceID)",
                    error: error
                )
                protocolNotice = ProtocolNotice(
                    title: "Unable to Load Transaction",
                    message: MixinErrorPresenter.message(for: error)
                )
            }
        case let .send(request):
            if let userID = request.userID {
                do {
                    let user = try await account.userProfile(userId: userID)
                    let conversationID = try await account.openUserConversation(
                        userId: userID
                    )
                    try await ProtocolSendService.send(
                        request.payload,
                        to: conversationID,
                        account: account
                    )
                    selectConversation(conversationID, name: user?.fullName)
                } catch {
                    AppLogger.error(
                        "Send deep link message failed: user_id=\(userID)",
                        error: error
                    )
                    protocolNotice = ProtocolNotice(
                        title: "Unable to Send Message",
                        message: MixinErrorPresenter.message(for: error)
                    )
                }
            } else if request.conversationID == nil,
                      request.payload.category == .text,
                      let selectedConversationID
            {
                do {
                    try await ProtocolSendService.send(
                        request.payload,
                        to: selectedConversationID,
                        account: account
                    )
                } catch {
                    AppLogger.error(
                        "Send deep link message failed: conversation_id=\(selectedConversationID)",
                        error: error
                    )
                    protocolNotice = ProtocolNotice(
                        title: "Unable to Send Message",
                        message: MixinErrorPresenter.message(for: error)
                    )
                }
            } else {
                protocolSendRequest = request
            }
        case .external:
            NSWorkspace.shared.open(url)
        case let .unsupported(kind):
            protocolNotice = ProtocolNotice(
                title: "Link Not Supported Yet",
                message: "\(kind.displayName) links are recognized, but this action is not available in the SwiftUI app yet."
            )
        case .invalid:
            protocolNotice = ProtocolNotice(
                title: "Unable to Open Link",
                message: "This Mixin link is invalid or incomplete."
            )
        }
    }

    func dismissProtocolNotice() {
        protocolNotice = nil
    }

    func dismissProtocolPresentation() {
        protocolPresentation = nil
    }

    func dismissProtocolSendRequest() {
        protocolSendRequest = nil
    }
}
