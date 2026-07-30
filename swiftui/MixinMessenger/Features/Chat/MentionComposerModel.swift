import Foundation
import Observation

@MainActor
@Observable
final class MentionComposerModel {
  private(set) var candidates: [ConversationParticipantItem] = []
  private(set) var keyword = ""
  private(set) var selectedIndex = 0
  private(set) var knownMentionIdentityNumbers = Set<String>()

  private enum Mode {
    case disabled
    case group
    case bot
  }

  private static let mentionQueryExpression: NSRegularExpression? = {
    do {
      return try NSRegularExpression(pattern: #"@(\S*)$"#)
    } catch {
      AppLogger.error("Compile mention query expression failed", error: error)
      return nil
    }
  }()

  private var account: SwiftAccountHandle?
  private var conversationID: String?
  private var currentUserID: String?
  private var mode = Mode.disabled
  private var loadTask: Task<Void, Never>?
  private var namesTask: Task<Void, Never>?
  private var inputText = ""
  private var selectionUTF16 = 0
  private var hasMarkedText = false
  private var requestRevision = 0

  func configure(
    account: SwiftAccountHandle,
    conversationID: String,
    conversationCategory: String?,
    conversationOwnerID: String?,
    currentUserID: String,
    initialText: String
  ) async {
    stop()
    self.account = account
    self.conversationID = conversationID
    self.currentUserID = currentUserID
    inputText = initialText
    selectionUTF16 = initialText.utf16.count

    let resolvedMode: Mode
    if conversationCategory == "GROUP" {
      resolvedMode = .group
    } else if let conversationOwnerID {
      do {
        if let owner = try await account.userProfile(
          userId: conversationOwnerID
        ) {
          resolvedMode = owner.isBot ? .bot : .disabled
        } else {
          AppLogger.error(
            "Composer conversation owner not found"
              + " conversation_id=\(conversationID)"
              + " owner_id=\(conversationOwnerID)"
          )
          resolvedMode = .disabled
        }
      } catch {
        AppLogger.error(
          "Resolve composer conversation owner failed"
            + " conversation_id=\(conversationID)"
            + " owner_id=\(conversationOwnerID)",
          error: error
        )
        resolvedMode = .disabled
      }
    } else {
      resolvedMode = .disabled
    }

    guard !Task.isCancelled, self.conversationID == conversationID else {
      return
    }
    mode = resolvedMode
    updateInput(
      ComposerEditingState(
        text: inputText,
        selectionUTF16: selectionUTF16,
        hasMarkedText: false
      )
    )
  }

  func updateInput(_ state: ComposerEditingState) {
    inputText = state.text
    selectionUTF16 = state.selectionUTF16
    hasMarkedText = state.hasMarkedText

    guard !state.hasMarkedText else {
      dismiss()
      return
    }

    resolveMentionNames(in: state.text)

    guard mode != .disabled,
      let match = Self.mentionMatch(
        in: state.text,
        selectionUTF16: state.selectionUTF16
      )
    else {
      dismiss()
      return
    }

    if keyword != match.keyword {
      selectedIndex = 0
    }
    keyword = match.keyword
    loadCandidates(keyword: match.keyword)
  }

  func moveSelection(_ delta: Int) {
    guard !candidates.isEmpty else {
      return
    }
    selectedIndex = min(max(selectedIndex + delta, 0), candidates.count - 1)
  }

  func selectCandidate(at index: Int?, in text: String) -> MentionInsertion? {
    guard !hasMarkedText,
      let match = Self.mentionMatch(
        in: text,
        selectionUTF16: selectionUTF16
      ),
      candidates.indices.contains(index ?? selectedIndex)
    else {
      return nil
    }

    let user = candidates[index ?? selectedIndex]
    let replacement = "@\(user.identityNumber) "
    let updated = (text as NSString).replacingCharacters(
      in: match.range,
      with: replacement
    )
    let cursor = match.range.location + (replacement as NSString).length

    knownMentionIdentityNumbers.insert(user.identityNumber)
    dismiss()
    inputText = updated
    selectionUTF16 = cursor
    resolveMentionNames(in: updated)
    return MentionInsertion(text: updated, selectionUTF16: cursor)
  }

  func dismiss() {
    requestRevision += 1
    loadTask?.cancel()
    loadTask = nil
    candidates = []
    keyword = ""
    selectedIndex = 0
  }

  func stop() {
    requestRevision += 1
    loadTask?.cancel()
    namesTask?.cancel()
    loadTask = nil
    namesTask = nil
    account = nil
    conversationID = nil
    currentUserID = nil
    mode = .disabled
    candidates = []
    keyword = ""
    selectedIndex = 0
    knownMentionIdentityNumbers = []
  }

  private func loadCandidates(keyword: String) {
    guard let account, let conversationID else {
      AppLogger.error("Load mention candidates failed: composer is not configured")
      dismiss()
      return
    }

    requestRevision += 1
    let revision = requestRevision
    let mode = mode
    let currentUserID = currentUserID
    loadTask?.cancel()
    loadTask = Task {
      do {
        let users: [ConversationParticipantItem]
        switch mode {
        case .group where keyword.isEmpty:
          users = try await account.conversationParticipants(
            conversationId: conversationID
          ).filter { $0.userId != currentUserID }
        case .group:
          users = try await account.searchGroupUsers(
            conversationId: conversationID,
            keyword: keyword
          )
        case .bot:
          users = try await account.searchBotGroupUsers(
            conversationId: conversationID,
            keyword: keyword
          )
        case .disabled:
          users = []
        }

        guard !Task.isCancelled,
          revision == requestRevision,
          self.conversationID == conversationID
        else {
          return
        }
        candidates = users
        selectedIndex = 0
      } catch {
        guard !Task.isCancelled, revision == requestRevision else {
          return
        }
        AppLogger.error(
          "Load mention candidates failed"
            + " conversation_id=\(conversationID)"
            + " keyword=\(keyword)",
          error: error
        )
        candidates = []
        selectedIndex = 0
      }
    }
  }

  private func resolveMentionNames(in text: String) {
    guard let account else {
      return
    }

    namesTask?.cancel()
    namesTask = Task {
      do {
        let names = try await account.mentionNames(contents: [text])
        guard !Task.isCancelled else {
          return
        }
        knownMentionIdentityNumbers.formUnion(names.keys)
      } catch {
        guard !Task.isCancelled else {
          return
        }
        AppLogger.error("Resolve composer mention names failed", error: error)
      }
    }
  }

  private static func mentionMatch(
    in text: String,
    selectionUTF16: Int
  ) -> (range: NSRange, keyword: String)? {
    let nsText = text as NSString
    guard selectionUTF16 >= 0, selectionUTF16 <= nsText.length else {
      AppLogger.error(
        "Resolve mention query failed: invalid selection"
          + " selection=\(selectionUTF16)"
          + " text_length=\(nsText.length)"
      )
      return nil
    }
    guard let expression = mentionQueryExpression else {
      return nil
    }

    let prefixRange = NSRange(location: 0, length: selectionUTF16)
    guard let match = expression.firstMatch(in: text, range: prefixRange),
      match.range.location != NSNotFound
    else {
      return nil
    }
    return (
      match.range,
      nsText.substring(with: match.range(at: 1))
    )
  }
}

struct MentionInsertion {
  let text: String
  let selectionUTF16: Int
}
