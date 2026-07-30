import Observation
import SwiftUI

struct CommandPaletteSheet: View {
  @Environment(AccountSession.self) private var session
  @Environment(HomeNavigationModel.self) private var navigation
  @Environment(\.dismiss) private var dismiss
  @Environment(\.mixinTheme) private var theme
  @State private var model = CommandPaletteModel()
  @State private var query = ""
  @State private var selection: CommandPaletteItem.ID?
  @FocusState private var searchFocused: Bool
  let onSelect: (String, String) -> Void

  var body: some View {
    VStack(spacing: 0) {
      HStack(spacing: 10) {
        MixinSearchField(
          text: $query,
          focus: $searchFocused,
          placeholder: "Search",
          onSubmit: selectCurrent,
          onExit: { dismiss() }
        )
        Button {
          dismiss()
        } label: {
          Image(systemName: "xmark")
        }
        .buttonStyle(MixinActionButtonStyle())
        .help("Close")
      }
      .padding(.top, 20)
      .padding(.bottom, 10)
      .padding(.horizontal, 20)

      switch model.state {
      case .loading:
        ProgressView()
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      case .failed(let message):
        ContentUnavailableView(
          "Unable to load quick search",
          systemImage: "exclamationmark.triangle",
          description: Text(message)
        )
      case .ready:
        let items = model.items
        if items.isEmpty {
          VStack(spacing: 0) {
            Image("EmptyFile")
              .resizable()
              .renderingMode(.template)
              .foregroundStyle(theme.secondaryText)
              .frame(width: 80, height: 80)
            Spacer().frame(height: 20)
            Text("No Results")
              .foregroundStyle(theme.secondaryText)
          }
          .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
          AppListView(items, selection: $selection) { item in
            CommandPaletteRow(item: item, query: query)
              .tag(item.id)
              .contentShape(Rectangle())
              .onTapGesture {
                select(item)
              }
          }
          .onChange(of: items.map(\.id)) {
            if !items.contains(where: { $0.id == selection }) {
              selection = items.first?.id
            }
          }
        }
      }

      if model.opening {
        Divider()
        ProgressView("Opening conversation…")
          .controlSize(.small)
          .padding(10)
      }
    }
    .frame(minWidth: 400, idealWidth: 480, maxWidth: 480, minHeight: 400, maxHeight: 600)
    .background(theme.popUp)
    .clipShape(RoundedRectangle(cornerRadius: 11))
    .task {
      searchFocused = true
    }
    .task(id: query) {
      try? await Task.sleep(for: .milliseconds(100))
      guard !Task.isCancelled else {
        return
      }
      await model.search(
        account: session.handle,
        query: query,
        recentConversationIDs: navigation.recentConversationIDs
      )
      selection = model.items.first?.id
    }
    .alert(
      "Unable to Open Conversation",
      isPresented: Binding(
        get: { model.openError != nil },
        set: { if !$0 { model.openError = nil } }
      )
    ) {
      Button("OK") {
        model.openError = nil
      }
    } message: {
      Text(model.openError ?? "")
    }
  }

  private func selectCurrent() {
    let items = model.items
    guard let item = items.first(where: { $0.id == selection }) ?? items.first else {
      return
    }
    select(item)
  }

  private func select(_ item: CommandPaletteItem) {
    Task {
      guard let result = await model.open(item) else {
        return
      }
      dismiss()
      onSelect(result.id, result.name)
    }
  }
}

private struct CommandPaletteRow: View {
  let item: CommandPaletteItem
  let query: String

  var body: some View {
    HStack(spacing: 12) {
      Group {
        if item.isGroup {
          GroupAvatarPuzzle(avatars: Array(item.groupAvatars.prefix(4)))
        } else {
          UserAvatar(
            userID: item.ownerID,
            name: item.name,
            url: item.avatarURL,
            size: 40
          )
        }
      }
      .frame(width: 40, height: 40)
      .clipShape(Circle())

      CommandPaletteHighlightedText(text: item.name, query: query)
      ProfileIdentityBadge(
        isVerified: item.isVerified,
        isBot: item.isBot,
        membership: item.membership
      )
      Spacer()
    }
    .padding(.horizontal, 14)
    .frame(height: 72)
  }
}

private struct CommandPaletteItem: Identifiable {
  enum Kind {
    case conversation
    case user
  }

  let id: String
  let kind: Kind
  let name: String
  let subtitle: String
  let avatarURL: String
  let ownerID: String
  let identityNumber: String
  let isGroup: Bool
  let groupAvatars: [GroupAvatar]
  let isVerified: Bool
  let isBot: Bool
  let membership: String?
  let matchScore: Int
}

@MainActor
@Observable
private final class CommandPaletteModel {
  struct OpenResult {
    let id: String
    let name: String
  }

  enum State {
    case loading
    case ready
    case failed(String)
  }

  private(set) var state: State = .loading
  private(set) var items: [CommandPaletteItem] = []
  private(set) var opening = false
  var openError: String?
  private var account: SwiftAccountHandle?
  private var requestVersion = 0

  func search(
    account: SwiftAccountHandle,
    query: String,
    recentConversationIDs: [String]
  ) async {
    self.account = account
    requestVersion += 1
    let version = requestVersion
    if items.isEmpty {
      state = .loading
    }
    let keyword = query.trimmingCharacters(in: .whitespacesAndNewlines)
    do {
      let conversations = try await account.conversations(
        category: "chats",
        circleId: nil,
        keyword: keyword,
        unseenOnly: false,
        limit: keyword.isEmpty ? 200 : 100,
        offset: 0
      )
      let users = keyword.isEmpty
        ? []
        : try await account.searchLocalUsers(
          query: keyword,
          category: "chats",
          limit: 100
        )
      guard version == requestVersion, !Task.isCancelled else {
        return
      }
      let visibleConversations: [ConversationListData]
      if keyword.isEmpty {
        let byID = Dictionary(
          uniqueKeysWithValues: conversations.map {
            ($0.conversationId, $0)
          }
        )
        visibleConversations = recentConversationIDs.compactMap {
          byID[$0]
        }
      } else {
        visibleConversations = conversations
      }
      let conversationItems = visibleConversations.map {
        makeConversationItem($0, query: keyword)
      }
      let userItems = users.map {
        makeUserItem($0, query: keyword)
      }
      items = (conversationItems + userItems).sorted {
        $0.matchScore > $1.matchScore
      }
      state = .ready
    } catch {
      guard version == requestVersion, !Task.isCancelled else {
        return
      }
      state = .failed(MixinErrorPresenter.message(for: error))
    }
  }

  func open(_ item: CommandPaletteItem) async -> OpenResult? {
    guard !opening, let account else {
      return nil
    }
    opening = true
    openError = nil
    defer { opening = false }
    switch item.kind {
    case .conversation:
      let id = String(item.id.dropFirst("conversation:".count))
      return OpenResult(id: id, name: item.name)
    case .user:
      do {
        let userID = String(item.id.dropFirst("user:".count))
        let id = try await account.openUserConversation(userId: userID)
        return OpenResult(id: id, name: item.name)
      } catch {
        openError = MixinErrorPresenter.message(for: error)
        return nil
      }
    }
  }

  private func makeConversationItem(
    _ conversation: ConversationListData,
    query: String
  ) -> CommandPaletteItem {
    CommandPaletteItem(
      id: "conversation:\(conversation.conversationId)",
      kind: .conversation,
      name: conversation.name,
      subtitle: conversation.lastMessage,
      avatarURL: conversation.avatarUrl,
      ownerID: conversation.ownerId,
      identityNumber: conversation.identityNumber,
      isGroup: conversation.category == "GROUP",
      groupAvatars: conversation.groupAvatars,
      isVerified: conversation.isVerified,
      isBot: conversation.isBot,
      membership: conversation.membership,
      matchScore: Self.matchScore(
        query: query,
        name: conversation.name,
        identityNumber: conversation.identityNumber
      )
    )
  }

  private func makeUserItem(
    _ user: UserProfileItem,
    query: String
  ) -> CommandPaletteItem {
    CommandPaletteItem(
      id: "user:\(user.userId)",
      kind: .user,
      name: user.fullName,
      subtitle: "",
      avatarURL: user.avatarUrl,
      ownerID: user.userId,
      identityNumber: user.identityNumber,
      isGroup: false,
      groupAvatars: [],
      isVerified: user.isVerified,
      isBot: user.isBot,
      membership: user.membership,
      matchScore: Self.matchScore(
        query: query,
        name: user.fullName,
        identityNumber: user.identityNumber
      )
    )
  }

  private static func matchScore(
    query: String,
    name: String,
    identityNumber: String
  ) -> Int {
    let query = query.lowercased()
    guard !query.isEmpty else {
      return 0
    }
    let name = name.lowercased()
    let identityNumber = identityNumber.lowercased()
    if name == query || identityNumber == query {
      return 100
    }
    if name.hasPrefix(query) || identityNumber.hasPrefix(query) {
      return 80
    }
    return name.contains(query) || identityNumber.contains(query)
      ? 60
      : 0
  }
}

private struct CommandPaletteHighlightedText: View {
  @Environment(\.mixinTheme) private var theme
  let text: String
  let query: String

  var body: some View {
    let range = text.range(
      of: query.trimmingCharacters(in: .whitespacesAndNewlines),
      options: [.caseInsensitive]
    )
    if let range, !query.isEmpty {
      (
        Text(String(text[..<range.lowerBound]))
          .foregroundColor(theme.text)
          + Text(String(text[range]))
          .foregroundColor(theme.accent)
          + Text(String(text[range.upperBound...]))
          .foregroundColor(theme.text)
      )
      .font(.system(size: 16))
      .lineLimit(1)
    } else {
      Text(text)
        .font(.system(size: 16))
        .foregroundStyle(theme.text)
        .lineLimit(1)
    }
  }
}
