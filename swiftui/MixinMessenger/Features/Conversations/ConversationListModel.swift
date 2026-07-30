import Foundation
import Observation

@MainActor
@Observable
final class ConversationListModel {
    private static let pageSize: Int64 = 50

    private(set) var state: ConversationListPhase = .loading
    private(set) var conversations: [ConversationListData] = []
    private(set) var circles: [CircleItem] = []
    private(set) var operationError: String?
    private(set) var canLoadMore = false
    private(set) var loadingMore = false
    private var requestVersion = 0
    private var account: SwiftAccountHandle?
    private var currentSection: HomeSection = .chats
    private var currentKeyword = ""
    private var currentUnseenOnly = false
    private var loadedLimit = Int(ConversationListModel.pageSize)
    private var nextOffset: Int64 = 0
    private var subscription: SwiftConversationSubscription?
    private var subscriptionTask: Task<Void, Never>?
    private var circleSubscription: SwiftCircleSubscription?
    private var circleSubscriptionTask: Task<Void, Never>?
    private var pendingConversationIDs = Set<String>()
    private var reloadAllPending = false
    private var changeFlushTask: Task<Void, Never>?
    private var changeFlushGeneration = 0

    func start(account: SwiftAccountHandle) async {
        guard self.account !== account else {
            return
        }
        stop()
        self.account = account
        do {
            circles = try await account.circles()
        } catch {
            AppLogger.error("ConversationList circle load failed", error: error)
            operationError = MixinErrorPresenter.message(for: error)
        }
        let circleSubscription = account.circleChanges()
        self.circleSubscription = circleSubscription
        circleSubscriptionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let circles = await circleSubscription.next()
            {
                self?.circles = circles
            }
        }
        let subscription = account.conversationChanges()
        self.subscription = subscription
        subscriptionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let event = await subscription.next()
            {
                guard let self else {
                    return
                }
                scheduleChanges(
                    reloadAll: event.reloadAll,
                    conversationIDs: event.conversationIds
                )
            }
        }
    }

    func load(
        account: SwiftAccountHandle,
        section: HomeSection,
        keyword: String,
        unseenOnly: Bool
    ) async {
        currentSection = section
        currentKeyword = keyword
        currentUnseenOnly = unseenOnly
        requestVersion += 1
        let version = requestVersion
        state = .loading
        loadingMore = false
        loadedLimit = Int(Self.pageSize)
        nextOffset = 0
        do {
            let filter = section.conversationFilter
            let page = try await account.conversations(
                category: filter.category,
                circleId: filter.circleID,
                keyword: keyword,
                unseenOnly: unseenOnly,
                limit: Self.pageSize,
                offset: 0
            )
            guard version == requestVersion else {
                return
            }
            conversations = page
            nextOffset = Int64(page.count)
            canLoadMore = page.count == Self.pageSize
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            AppLogger.error(
                "ConversationList load failed: section=\(String(describing: section)) unseen_only=\(unseenOnly)",
                error: error
            )
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func loadNextPage() async {
        guard let account else {
            AppLogger.error("ConversationList load next page failed: account unavailable")
            return
        }
        guard case .ready = state,
              canLoadMore,
              !loadingMore
        else {
            return
        }
        loadingMore = true
        let version = requestVersion
        let offset = nextOffset
        do {
            let filter = currentSection.conversationFilter
            let page = try await account.conversations(
                category: filter.category,
                circleId: filter.circleID,
                keyword: currentKeyword,
                unseenOnly: currentUnseenOnly,
                limit: Self.pageSize,
                offset: offset
            )
            guard version == requestVersion else {
                return
            }
            merge(page)
            nextOffset += Int64(page.count)
            loadedLimit = max(loadedLimit, Int(nextOffset))
            canLoadMore = page.count == Self.pageSize
        } catch {
            guard version == requestVersion else {
                return
            }
            AppLogger.error(
                "ConversationList load next page failed: offset=\(offset)",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
        }
        if version == requestVersion {
            loadingMore = false
        }
    }

    func stop() {
        requestVersion += 1
        subscription?.cancel()
        subscription = nil
        subscriptionTask?.cancel()
        subscriptionTask = nil
        circleSubscription?.cancel()
        circleSubscription = nil
        circleSubscriptionTask?.cancel()
        circleSubscriptionTask = nil
        changeFlushTask?.cancel()
        changeFlushTask = nil
        changeFlushGeneration += 1
        pendingConversationIDs = []
        reloadAllPending = false
        account = nil
    }

    func editCircleMembership(
        _ conversation: ConversationListData,
        circleID: String,
        add: Bool
    ) async {
        guard let account else {
            AppLogger.error(
                "ConversationList edit circle failed: account unavailable conversation_id=\(conversation.conversationId)"
            )
            return
        }
        do {
            try await account.editCircleConversation(
                circleId: circleID,
                conversationId: conversation.conversationId,
                ownerId: conversation.ownerId,
                category: conversation.category,
                add: add
            )
            await reloadVisibleWindow()
        } catch {
            AppLogger.error(
                "ConversationList edit circle failed: conversation_id=\(conversation.conversationId) circle_id=\(circleID) add=\(add)",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func setPinned(
        _ conversation: ConversationListData,
        pinned: Bool
    ) async {
        guard let account else {
            AppLogger.error(
                "ConversationList set pinned failed: account unavailable conversation_id=\(conversation.conversationId)"
            )
            return
        }
        do {
            try await account.setConversationPinned(
                conversationId: conversation.conversationId,
                pinned: pinned
            )
            await reloadVisibleWindow()
        } catch {
            AppLogger.error(
                "ConversationList set pinned failed: conversation_id=\(conversation.conversationId) pinned=\(pinned)",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func setMuted(
        _ conversation: ConversationListData,
        duration: Int64
    ) async {
        guard let account else {
            AppLogger.error(
                "ConversationList set muted failed: account unavailable conversation_id=\(conversation.conversationId)"
            )
            return
        }
        do {
            try await account.setConversationMuted(
                conversationId: conversation.conversationId,
                ownerId: conversation.ownerId,
                category: conversation.category,
                durationSeconds: duration
            )
            await reloadVisibleWindow()
        } catch {
            AppLogger.error(
                "ConversationList set muted failed: conversation_id=\(conversation.conversationId) duration=\(duration)",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func delete(_ conversation: ConversationListData) async -> Bool {
        guard let account else {
            AppLogger.error(
                "ConversationList delete failed: account unavailable conversation_id=\(conversation.conversationId)"
            )
            return false
        }
        do {
            try await account.deleteConversation(
                conversationId: conversation.conversationId
            )
            await reloadVisibleWindow()
            return true
        } catch {
            AppLogger.error(
                "ConversationList delete failed: conversation_id=\(conversation.conversationId)",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func dismissOperationError() {
        operationError = nil
    }

    func presentOperationError(_ error: Error) {
        operationError = MixinErrorPresenter.message(for: error)
    }

    private func reloadVisibleWindow() async {
        guard let account else {
            AppLogger.error(
                "ConversationList reload visible window failed: account unavailable"
            )
            return
        }
        let version = requestVersion
        let filter = currentSection.conversationFilter
        let requestedLimit = max(loadedLimit, Int(Self.pageSize))
        do {
            let items = try await account.conversations(
                category: filter.category,
                circleId: filter.circleID,
                keyword: currentKeyword,
                unseenOnly: currentUnseenOnly,
                limit: Int64(requestedLimit),
                offset: 0
            )
            guard version == requestVersion else {
                return
            }
            if conversations != items {
                conversations = items
            }
            loadedLimit = requestedLimit
            nextOffset = Int64(items.count)
            canLoadMore = items.count == requestedLimit
            state = .ready
        } catch {
            guard version == requestVersion else {
                return
            }
            AppLogger.error(
                "ConversationList reload visible window failed",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func applyChanges(conversationIDs: [String]) async {
        guard let account else {
            AppLogger.error(
                "ConversationList apply changes failed: account unavailable ids=\(conversationIDs)"
            )
            return
        }
        let version = requestVersion
        do {
            let changed = try await account.conversationItemsByIds(
                conversationIds: Array(Set(conversationIDs))
            )
            guard version == requestVersion else {
                return
            }
            let changedIDs = Set(conversationIDs)
            var updated = conversations.filter {
                !changedIDs.contains($0.conversationId)
            }
            updated.append(contentsOf: changed.filter(matchesCurrentQuery))
            updated.sort(by: Self.isOrderedBefore)
            if updated.count > loadedLimit {
                updated.removeLast(updated.count - loadedLimit)
            }
            if updated != conversations {
                conversations = updated
            }
        } catch {
            guard version == requestVersion else {
                return
            }
            AppLogger.error(
                "ConversationList apply changes failed: ids=\(conversationIDs)",
                error: error
            )
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    private func merge(_ items: [ConversationListData]) {
        var byID = Dictionary(
            uniqueKeysWithValues: conversations.map { ($0.conversationId, $0) }
        )
        for item in items {
            byID[item.conversationId] = item
        }
        let updated = byID.values.sorted(by: Self.isOrderedBefore)
        if updated != conversations {
            conversations = updated
        }
    }

    private func scheduleChanges(
        reloadAll: Bool,
        conversationIDs: [String]
    ) {
        reloadAllPending = reloadAllPending || reloadAll
        pendingConversationIDs.formUnion(conversationIDs)
        guard changeFlushTask == nil else {
            return
        }
        let generation = changeFlushGeneration
        changeFlushTask = Task { [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(16))
            } catch is CancellationError {
                return
            } catch {
                AppLogger.error(
                    "ConversationList change flush delay failed",
                    error: error
                )
                return
            }
            guard !Task.isCancelled, let self else {
                return
            }
            await flushChanges(generation: generation)
        }
    }

    private func flushChanges(generation: Int) async {
        guard generation == changeFlushGeneration else {
            return
        }
        let reloadAll = reloadAllPending
        let conversationIDs = Array(pendingConversationIDs)
        reloadAllPending = false
        pendingConversationIDs = []

        if reloadAll {
            await reloadVisibleWindow()
        } else if !conversationIDs.isEmpty {
            await applyChanges(conversationIDs: conversationIDs)
        }

        guard generation == changeFlushGeneration else {
            return
        }
        changeFlushTask = nil
        if reloadAllPending || !pendingConversationIDs.isEmpty {
            scheduleChanges(reloadAll: false, conversationIDs: [])
        }
    }

    private func matchesCurrentQuery(_ item: ConversationListData) -> Bool {
        let filter = currentSection.conversationFilter
        let sectionMatches = switch filter.category {
        case "contacts":
            item.category == "CONTACT" && item.relationship == "FRIEND" && !item.isBot
        case "groups":
            item.category == "GROUP"
        case "bots":
            item.category == "CONTACT" && item.isBot
        case "strangers":
            item.category == "CONTACT" && item.relationship == "STRANGER" && !item.isBot
        case "circle":
            filter.circleID.map(item.circleIds.contains) ?? false
        default:
            item.category == "CONTACT" || item.category == "GROUP"
        }
        guard sectionMatches, !currentUnseenOnly || item.unseenCount > 0 else {
            return false
        }
        let query = currentKeyword.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return true
        }
        return [item.name, item.identityNumber, item.lastMessage].contains {
            $0.localizedCaseInsensitiveContains(query)
        }
    }

    private static func isOrderedBefore(
        _ lhs: ConversationListData,
        _ rhs: ConversationListData
    ) -> Bool {
        if lhs.pinTimeMillis != rhs.pinTimeMillis {
            return lhs.pinTimeMillis > rhs.pinTimeMillis
        }
        if lhs.hasDraft != rhs.hasDraft {
            return lhs.hasDraft
        }
        if lhs.updatedAtMillis != rhs.updatedAtMillis {
            return lhs.updatedAtMillis > rhs.updatedAtMillis
        }
        return lhs.conversationId < rhs.conversationId
    }
}

struct ConversationListQuery: Hashable {
    let section: HomeSection
    let keyword: String
    let unseenOnly: Bool
}
