import SwiftUI

struct HomeView: View {
  @Environment(AccountSession.self) private var session
  @State private var navigation = HomeNavigationModel()
  @Environment(SettingsPreferencesModel.self) private var preferences
  @State private var dockBadge = DockBadgeController()
  @State private var notifications = NotificationController()

  var body: some View {
    ResponsiveHomeShell()
      .environment(navigation)
      .focusedSceneValue(\.homeNavigation, navigation)
      .focusedSceneValue(\.accountSecurity, session.security)
      .task {
        dockBadge.start(account: session.handle)
        notifications.start(
          account: session.handle,
          navigation: navigation,
          preferences: preferences
        )
      }
      .onChange(of: navigation.selectedConversationID) {
        if let conversationID = navigation.selectedConversationID {
          notifications.dismiss(conversationID: conversationID)
        }
      }
    .onDisappear {
      dockBadge.stop()
      notifications.stop()
    }
    .onOpenURL { url in
      Task {
        await navigation.open(url, account: session.handle)
      }
    }
    .alert(
      navigation.protocolNotice?.title ?? "",
      isPresented: Binding(
        get: { navigation.protocolNotice != nil },
        set: { if !$0 { navigation.dismissProtocolNotice() } }
      ),
      actions: {
        Button("OK") {
          navigation.dismissProtocolNotice()
        }
      },
      message: {
        Text(navigation.protocolNotice?.message ?? "")
      }
    )
    .sheet(
      item: Binding(
        get: { navigation.protocolPresentation },
        set: { presentation in
          if presentation == nil {
            navigation.dismissProtocolPresentation()
          }
        }
      )
    ) { presentation in
      ProtocolPresentationSheet(presentation: presentation)
    }
    .sheet(
      item: Binding(
        get: { navigation.protocolSendRequest },
        set: { request in
          if request == nil {
            navigation.dismissProtocolSendRequest()
          }
        }
      )
    ) { request in
      ProtocolSendSheet(request: request)
    }
    .sheet(
      item: Binding(
        get: { navigation.creationRequest },
        set: { request in
          if request == nil {
            navigation.dismissCreation()
          }
        }
      )
    ) { request in
      ConversationCreationSheet(kind: request.kind) { conversationID, name in
        navigation.dismissCreation()
        if let conversationID {
          navigation.selectConversation(conversationID, name: name)
        }
      }
    }
    .sheet(isPresented: $navigation.commandPalettePresented) {
      CommandPaletteSheet { conversationID, name in
        navigation.commandPalettePresented = false
        navigation.selectConversation(conversationID, name: name)
      }
    }
  }
}

private struct ResponsiveHomeShell: View {
  @Environment(HomeNavigationModel.self) private var navigation
  @Environment(\.mixinTheme) private var theme
  @State private var userCollapsed = false
  @State private var drawerPresented = false

  var body: some View {
    GeometryReader { proxy in
      let layout = HomeShellLayout.resolve(
        maxWidth: proxy.size.width,
        userCollapsed: userCollapsed
      )
      HStack(spacing: 0) {
        if !layout.hasDrawer {
          HomeSidebarView(
            collapsed: layout.sidebarCollapsed,
            showCollapseControl: layout.showCollapseControl,
            onToggleCollapsed: {
              withAnimation(.easeInOut(duration: 0.2)) {
                userCollapsed.toggle()
              }
            }
          )
          .frame(width: layout.sidebarWidth)
        }

        mainContent(layout: layout)
      }
      .overlay(alignment: .leading) {
        if layout.hasDrawer, drawerPresented {
          ZStack(alignment: .leading) {
            Color.black.opacity(0.54)
              .contentShape(Rectangle())
              .onTapGesture {
                withAnimation(.easeInOut(duration: 0.2)) {
                  drawerPresented = false
                }
              }
            HomeSidebarView()
              .frame(width: HomeShellLayout.fullSidebarWidth)
              .background(theme.primary)
              .shadow(color: .black.opacity(0.32), radius: 16)
          }
          .transition(.opacity.combined(with: .move(edge: .leading)))
        }
      }
      .onChange(of: navigation.section) {
        drawerPresented = false
      }
      .onChange(of: navigation.page) {
        drawerPresented = false
      }
      .onChange(of: proxy.size.width) {
        if !layout.hasDrawer {
          drawerPresented = false
        }
      }
    }
  }

  @ViewBuilder
  private func mainContent(layout: HomeShellLayout) -> some View {
    if layout.routeMode {
      RoutedHomeContent(
        onShowSidebar: layout.hasDrawer ? { showSidebar() } : nil
      )
    } else {
      wideContent
    }
  }

  @ViewBuilder
  private var wideContent: some View {
    switch navigation.page {
    case .settings:
      SettingsView(routeMode: false)
    case .chats:
      HStack(spacing: 0) {
        ConversationListView()
          .frame(width: HomeShellLayout.conversationListWidth)
          .contentShape(.interaction, Rectangle())
          .clipped()
          .zIndex(1)
        Divider()
        chatDetailWithInfo
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .contentShape(.interaction, Rectangle())
          .clipped()
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
  }

  private func showSidebar() {
    withAnimation(.easeInOut(duration: 0.2)) {
      drawerPresented = true
    }
  }

  @ViewBuilder
  private var chatDetailWithInfo: some View {
    HStack(spacing: 0) {
      if let conversationID = navigation.selectedConversationID {
        ConversationChatDetail(conversationID: conversationID)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .clipped()
      } else {
        emptyChat
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      }

      if navigation.infoPresented,
        let conversation = navigation.selectedConversation
      {
        Divider()
        ConversationInfoView(conversation: conversation)
          .frame(width: HomeShellLayout.chatInfoWidth)
      }
    }
  }

  private var emptyChat: some View {
    Text("Pick a conversation")
      .foregroundStyle(theme.secondaryText)
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .background(theme.chatBackground)
  }
}

private struct RoutedHomeContent: View {
  @Environment(HomeNavigationModel.self) private var navigation

  let onShowSidebar: (() -> Void)?

  init(onShowSidebar: (() -> Void)?) {
    self.onShowSidebar = onShowSidebar
  }

  var body: some View {
    Group {
      if let route = navigation.routePath.last {
        destination(route)
          .id(route)
      } else {
        root
      }
    }
    .toolbar {
      if !navigation.routePath.isEmpty {
        ToolbarItem(placement: .navigation) {
          Button {
            navigation.routePath.removeLast()
          } label: {
            Image(systemName: "chevron.backward")
          }
          .help("Back")
          .accessibilityLabel("Back")
        }
      }
    }
  }

  @ViewBuilder
  private var root: some View {
    switch navigation.page {
    case .settings:
      SettingsView(
        routeMode: true,
        onShowSidebar: onShowSidebar
      )
    case .chats:
      ConversationListView(onShowSidebar: onShowSidebar)
    }
  }

  @ViewBuilder
  private func destination(_ route: HomeRoute) -> some View {
    switch route {
    case .chat(let conversationID):
      ConversationChatDetail(conversationID: conversationID)
    case .chatInfo:
      if let conversation = navigation.selectedConversation {
        ConversationInfoView(conversation: conversation)
      }
    case .settings(let destination):
      SettingsDetailView(destination: destination)
    }
  }
}

private struct ConversationChatDetail: View {
  @Environment(HomeNavigationModel.self) private var navigation

  let conversationID: String

  var body: some View {
    ChatTimelineView(
      conversationID: conversationID,
      conversationName: navigation.selectedConversationName,
      conversationCategory: navigation.selectedConversation?.category,
      conversationOwnerID: navigation.selectedConversation?.ownerId,
      conversationIsBot: navigation.selectedConversation?.isBot ?? false,
      conversationIsBotGroup:
        navigation.selectedConversation?.isBotGroup ?? false,
      conversationIsScam: navigation.selectedConversation?.isScam ?? false,
      participantCount: navigation.selectedConversation?.participantCount ?? 0,
      lastReadMessageID: navigation.selectedConversation?.lastReadMessageId,
      unseenCount: navigation.selectedConversation?.unseenCount ?? 0,
      initialDraft: navigation.selectedConversationDraft
    )
    .id(conversationID)
  }
}

private enum HomeSidebarMode {
  case drawer
  case compactRail
  case fullRail
}

private struct HomeShellLayout {
  static let compactSidebarWidth = 64.0
  static let fullSidebarWidth = 176.0
  static let responsiveNavigationMinWidth = 320.0
  static let conversationListWidth = 300.0
  static let chatInfoWidth = 300.0
  static let mainRouteSwitchWidth =
    responsiveNavigationMinWidth + conversationListWidth

  let sidebarMode: HomeSidebarMode
  let sidebarWidth: Double
  let autoCollapse: Bool
  let routeMode: Bool

  var hasDrawer: Bool {
    sidebarMode == .drawer
  }

  var sidebarCollapsed: Bool {
    sidebarMode != .fullRail
  }

  var showCollapseControl: Bool {
    !autoCollapse
  }

  static func resolve(
    maxWidth: Double,
    userCollapsed: Bool
  ) -> HomeShellLayout {
    let availableSidebarWidth = min(
      fullSidebarWidth,
      max(
        compactSidebarWidth,
        maxWidth - responsiveNavigationMinWidth
      )
    )
    let autoCollapse = availableSidebarWidth < fullSidebarWidth
    let sidebarMode: HomeSidebarMode
    if availableSidebarWidth <= compactSidebarWidth {
      sidebarMode = .drawer
    } else if userCollapsed || autoCollapse {
      sidebarMode = .compactRail
    } else {
      sidebarMode = .fullRail
    }
    let sidebarWidth: Double = switch sidebarMode {
    case .drawer:
      0
    case .compactRail:
      compactSidebarWidth
    case .fullRail:
      fullSidebarWidth
    }
    return HomeShellLayout(
      sidebarMode: sidebarMode,
      sidebarWidth: sidebarWidth,
      autoCollapse: autoCollapse,
      routeMode: maxWidth - sidebarWidth < mainRouteSwitchWidth
    )
  }
}
