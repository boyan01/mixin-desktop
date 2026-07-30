import Foundation
import Observation

struct ConversationSearchState {
    var users: [UserProfileItem] = []
    var maoUser: UserProfileItem?
    var mao: String?
    var messages: [MessageItem] = []
    var messageConversations: [String: ConversationListData] = [:]
    var loading = false

    var isEmpty: Bool {
        users.isEmpty && maoUser == nil && messages.isEmpty
    }
}

@MainActor
@Observable
final class ConversationSearchModel {
    private(set) var state = ConversationSearchState()
    private var revision = 0

    func search(
        account: SwiftAccountHandle,
        query: String,
        category: String,
        enabled: Bool
    ) async {
        revision += 1
        let currentRevision = revision
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard enabled, !normalized.isEmpty else {
            state = ConversationSearchState()
            return
        }

        state.loading = true
        state.maoUser = nil
        state.mao = nil

        async let maoResult = searchMao(
            account: account,
            query: normalized
        )

        do {
            async let messages = account.searchGlobalMessages(
                query: normalized,
                anchorMessageId: nil,
                limit: 32
            )
            async let users = account.searchLocalUsers(
                query: normalized,
                category: category,
                limit: 64
            )
            let (messageItems, userItems) = try await (messages, users)
            let conversationIDs = Array(Set(messageItems.map(\.conversationId)))
            let conversations = conversationIDs.isEmpty
                ? []
                : try await account.conversationItemsByIds(
                    conversationIds: conversationIDs
                )
            let mao = await maoResult
            guard currentRevision == revision, !Task.isCancelled else {
                return
            }
            state = ConversationSearchState(
                users: userItems,
                maoUser: mao?.user,
                mao: mao?.name,
                messages: messageItems,
                messageConversations: Dictionary(
                    uniqueKeysWithValues: conversations.map {
                        ($0.conversationId, $0)
                    }
                ),
                loading: false
            )
        } catch {
            let mao = await maoResult
            guard currentRevision == revision, !Task.isCancelled else {
                return
            }
            AppLogger.error(
                "Conversation search failed: query=\(normalized)",
                error: error
            )
            state = ConversationSearchState(
                maoUser: mao?.user,
                mao: mao?.name,
                loading: false
            )
        }
    }

    func reset() {
        revision += 1
        state = ConversationSearchState()
    }

    private func searchMao(
        account: SwiftAccountHandle,
        query: String
    ) async -> (user: UserProfileItem, name: String)? {
        guard let completed = Self.completeMao(query) else {
            return nil
        }
        do {
            guard let user = try await account.searchMaoUser(query: query) else {
                return nil
            }
            return (user, completed)
        } catch {
            AppLogger.error(
                "Conversation MAO search failed: query=\(query)",
                error: error
            )
            return nil
        }
    }

    private static func completeMao(_ value: String) -> String? {
        let text = value.trimmingCharacters(in: .whitespacesAndNewlines)
        let candidate = text.hasSuffix(".") ? String(text.dropLast()) : text
        guard !candidate.isEmpty,
              candidate.unicodeScalars.count <= 128,
              !candidate.unicodeScalars.allSatisfy({
                  CharacterSet.decimalDigits.contains($0)
              }),
              candidate.rangeOfCharacter(
                  from: .whitespacesAndNewlines
              ) == nil,
              candidate == candidate.lowercased()
        else {
            return nil
        }
        if text.hasSuffix(".mao") {
            return text
        }
        if text.hasSuffix(".ma") {
            return "\(text)o"
        }
        if text.hasSuffix(".m") {
            return "\(text)ao"
        }
        if text.hasSuffix(".") {
            return "\(text)mao"
        }
        return "\(text).mao"
    }
}
