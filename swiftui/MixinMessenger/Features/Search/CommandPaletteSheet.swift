import Observation
import SwiftUI

struct CommandPaletteSheet: View {
  @Environment(AccountSession.self) private var session
  @Environment(\.dismiss) private var dismiss
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
          placeholder: "Search conversations and people",
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
      .padding(16)

      Divider()

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
        let items = model.filtered(query: query)
        if items.isEmpty {
          ContentUnavailableView.search(text: query)
        } else {
          List(items, selection: $selection) { item in
            CommandPaletteRow(item: item)
              .tag(item.id)
              .contentShape(Rectangle())
              .onTapGesture(count: 2) {
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
    .frame(minWidth: 460, idealWidth: 520, minHeight: 420, idealHeight: 560)
    .task {
      await model.load(account: session.handle)
      selection = model.filtered(query: "").first?.id
      searchFocused = true
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
    let items = model.filtered(query: query)
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

  var body: some View {
    HStack(spacing: 12) {
      MixinRemoteImage(url: URL(string: item.avatarURL)) { image in
        image.resizable().scaledToFill()
      } placeholder: {
        Image(
          systemName: item.kind == .conversation
            ? "bubble.left.fill"
            : "person.crop.circle.fill"
        )
        .resizable()
        .foregroundStyle(.secondary)
      }
      .frame(width: 42, height: 42)
      .clipShape(Circle())

      VStack(alignment: .leading, spacing: 3) {
        Text(item.name)
          .font(.headline)
          .lineLimit(1)
        Text(item.subtitle)
          .font(.caption)
          .foregroundStyle(.secondary)
          .lineLimit(1)
      }
      Spacer()
      Image(systemName: item.kind == .conversation ? "bubble.left" : "person")
        .foregroundStyle(.tertiary)
    }
    .padding(.vertical, 5)
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
  private(set) var opening = false
  var openError: String?
  private var account: SwiftAccountHandle?
  private var conversations: [SwiftConversationListItem] = []
  private var users: [SwiftUserItem] = []

  func load(account: SwiftAccountHandle) async {
    self.account = account
    state = .loading
    do {
      conversations = try await account.conversations(
        category: "chats",
        circleId: nil,
        keyword: "",
        unseenOnly: false,
        limit: 100,
        offset: 0
      )
      users = try await account.selectableUsers()
      state = .ready
    } catch {
      state = .failed(MixinErrorPresenter.message(for: error))
    }
  }

  func filtered(query: String) -> [CommandPaletteItem] {
    let keyword = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    let conversationItems =
      conversations
      .filter { keyword.isEmpty || $0.name.lowercased().contains(keyword) }
      .prefix(50)
      .map {
        CommandPaletteItem(
          id: "conversation:\($0.conversationId)",
          kind: .conversation,
          name: $0.name,
          subtitle: $0.lastMessage.isEmpty ? "Conversation" : $0.lastMessage,
          avatarURL: $0.iconUrl
        )
      }
    let userItems =
      users
      .filter {
        keyword.isEmpty
          || $0.fullName.lowercased().contains(keyword)
          || $0.identityNumber.contains(keyword)
      }
      .prefix(50)
      .map {
        CommandPaletteItem(
          id: "user:\($0.userId)",
          kind: .user,
          name: $0.fullName,
          subtitle: "Mixin ID: \($0.identityNumber)",
          avatarURL: $0.avatarUrl
        )
      }
    return Array(conversationItems + userItems)
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
}
