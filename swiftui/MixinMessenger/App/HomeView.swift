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
    .inspector(isPresented: $navigation.infoPresented) {
      if navigation.infoPresented,
        let conversation = navigation.selectedConversation
      {
        ConversationInfoView(conversation: conversation)
          .environment(navigation)
      }
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
        navigation.section = .chats
        navigation.selectConversation(conversationID, name: name)
      }
    }
  }
}

private struct ResponsiveHomeShell: View {
  @Environment(HomeNavigationModel.self) private var navigation
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
            collapsed: layout.collapsed,
            showCollapseControl: !layout.autoCollapse,
            onToggleCollapsed: {
              withAnimation(.easeInOut(duration: 0.2)) {
                userCollapsed.toggle()
              }
            }
          )
          .frame(width: layout.sidebarWidth)
          Divider()
        }

        mainContent(routeMode: layout.routeMode)
      }
      .overlay(alignment: .leading) {
        if layout.hasDrawer, drawerPresented {
          ZStack(alignment: .leading) {
            Color.black.opacity(0.2)
              .contentShape(Rectangle())
              .onTapGesture {
                withAnimation(.easeInOut(duration: 0.2)) {
                  drawerPresented = false
                }
              }
            HomeSidebarView()
              .frame(width: HomeShellLayout.fullSidebarWidth)
              .background(.background)
              .shadow(radius: 12)
          }
          .transition(.opacity.combined(with: .move(edge: .leading)))
        }
      }
      .toolbar {
        ToolbarItemGroup(placement: .navigation) {
          if layout.hasDrawer {
            Button {
              withAnimation(.easeInOut(duration: 0.2)) {
                drawerPresented.toggle()
              }
            } label: {
              Label("Show Sidebar", systemImage: "sidebar.left")
            }
            .help("Show Sidebar")
          }
          if navigation.section != .settings,
            layout.routeMode,
            navigation.selectedConversationID != nil
          {
            Button {
              navigation.clearConversationSelection()
            } label: {
              Label("Back to Conversations", systemImage: "chevron.left")
            }
            .help("Back to Conversations")
          }
        }
      }
      .onChange(of: navigation.section) {
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
  private func mainContent(routeMode: Bool) -> some View {
    if navigation.section == .settings {
      SettingsView()
    } else if routeMode {
      if navigation.selectedConversationID == nil {
        conversationList
      } else {
        chatDetail
      }
    } else {
      HStack(spacing: 0) {
        conversationList
          .frame(width: HomeShellLayout.conversationListWidth)
        Divider()
        chatDetail
      }
    }
  }

  private var conversationList: some View {
    ConversationListView()
  }

  @ViewBuilder
  private var chatDetail: some View {
    if let conversationID = navigation.selectedConversationID {
      ChatTimelineView(
        conversationID: conversationID,
        conversationName: navigation.selectedConversationName,
        conversationCategory: navigation.selectedConversation?.category,
        conversationOwnerID: navigation.selectedConversation?.ownerId,
        conversationIsBot: navigation.selectedConversation?.isBot ?? false,
        conversationIsScam: navigation.selectedConversation?.isScam ?? false,
        participantCount: navigation.selectedConversation?.participantCount ?? 0,
        lastReadMessageID: navigation.selectedConversation?.lastReadMessageId,
        unseenCount: navigation.selectedConversation?.unseenCount ?? 0,
        initialDraft: navigation.selectedConversationDraft
      )
      .id(conversationID)
    } else {
      ContentUnavailableView(
        "Pick a conversation",
        systemImage: "bubble.left.and.bubble.right"
      )
    }
  }
}

private struct HomeShellLayout {
  static let compactSidebarWidth = 64.0
  static let fullSidebarWidth = 176.0
  static let responsiveNavigationMinWidth = 320.0
  static let conversationListWidth = 300.0
  static let mainRouteSwitchWidth =
    responsiveNavigationMinWidth + conversationListWidth

  let sidebarWidth: Double
  let collapsed: Bool
  let autoCollapse: Bool
  let hasDrawer: Bool
  let routeMode: Bool

  static func resolve(maxWidth: Double, userCollapsed: Bool) -> HomeShellLayout {
    let availableSidebarWidth = min(
      fullSidebarWidth,
      max(compactSidebarWidth, maxWidth - responsiveNavigationMinWidth)
    )
    let autoCollapse = availableSidebarWidth < fullSidebarWidth
    let collapsed = userCollapsed || autoCollapse
    let hasDrawer =
      availableSidebarWidth <= compactSidebarWidth
    let sidebarWidth =
      hasDrawer
      ? 0
      : collapsed ? compactSidebarWidth : fullSidebarWidth
    return HomeShellLayout(
      sidebarWidth: sidebarWidth,
      collapsed: collapsed,
      autoCollapse: autoCollapse,
      hasDrawer: hasDrawer,
      routeMode: maxWidth - sidebarWidth < mainRouteSwitchWidth
    )
  }
}
