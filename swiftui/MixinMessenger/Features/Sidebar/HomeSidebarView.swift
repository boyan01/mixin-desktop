import SwiftUI

struct HomeSidebarView: View {
  var collapsed = false
  var showCollapseControl = false
  var onToggleCollapsed: () -> Void = {}

  @Environment(\.colorScheme) private var colorScheme
  @Environment(\.mixinTheme) private var theme
  @Environment(AccountSession.self) private var session
  @Environment(HomeNavigationModel.self) private var navigation
  @State private var model = SidebarModel()
  @State private var presentedSheet: CircleSheet?
  @State private var pendingDelete: SwiftCircleItem?

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      Spacer()
        .frame(height: 64)

      profile

      Spacer()
        .frame(height: 24)

      category(
        "All Chats",
        assetName: "SidebarChat",
        section: .chats
      )

      Spacer()
        .frame(height: 12)
      sidebarDivider
      Spacer()
        .frame(height: 12)

      VStack(spacing: 8) {
        category(
          "Contacts",
          assetName: "SidebarContacts",
          section: .contacts
        )
        category(
          "Groups",
          assetName: "SidebarGroups",
          section: .groups
        )
        category(
          "Bots",
          assetName: "SidebarBots",
          section: .bots
        )
        category(
          "Strangers",
          assetName: "SidebarStrangers",
          section: .strangers
        )
      }

      if !model.circles.isEmpty {
        Spacer()
          .frame(height: 16)
        sidebarDivider
        Spacer()
          .frame(height: 8)
      }

      ScrollView {
        LazyVStack(spacing: 0) {
          ForEach(model.circles, id: \.circleId) { circle in
            let section = HomeSection.circle(circle.circleId)
            sidebarItem(
              title: circle.name,
              assetName: "SidebarCircle",
              iconColor: circleColor(circle.circleId),
              section: section,
              count: model.unseenCount(for: section)
            )
            .padding(.vertical, 4)
            .draggable(circle.circleId)
            .dropDestination(for: String.self) { items, _ in
              guard
                let sourceID = items.first,
                let sourceIndex = model.circles.firstIndex(where: {
                  $0.circleId == sourceID
                }),
                let destinationIndex = model.circles.firstIndex(where: {
                  $0.circleId == circle.circleId
                }),
                sourceIndex != destinationIndex
              else {
                return false
              }
              let destination =
                destinationIndex > sourceIndex
                ? destinationIndex + 1
                : destinationIndex
              Task {
                await model.reorderCircles(
                  fromOffsets: IndexSet(integer: sourceIndex),
                  toOffset: destination,
                  account: session.handle
                )
              }
              return true
            }
            .contextMenu {
              Button("Rename") {
                presentedSheet = .rename(circle)
              }
              Button("Edit Conversations") {
                presentedSheet = .conversations(circle)
              }
              Divider()
              Button("Delete", role: .destructive) {
                pendingDelete = circle
              }
            }
          }
        }
      }
      .scrollIndicators(.never)

      if showCollapseControl {
        SidebarItem(
          title: "Collapse",
          assetName: collapsed ? "SidebarExpanded" : "SidebarCollapse",
          selected: false,
          collapsed: collapsed,
          count: 0,
          mutedCount: 0,
          action: onToggleCollapsed
        )
      }

      Spacer()
        .frame(height: 4)
    }
    .padding(.horizontal, 12)
    .background {
      theme.primary
        .overlay(sidebarBackgroundOverlay)
    }
    .task {
      await model.start(account: session.handle)
    }
    .onDisappear {
      model.stop()
    }
    .sheet(item: $presentedSheet) { sheet in
      switch sheet {
      case .rename(let circle):
        CircleNameSheet(
          circle: circle,
          onCancel: { presentedSheet = nil },
          onSave: { name in
            await model.updateCircle(
              circle,
              name: name,
              account: session.handle
            )
          }
        )
      case .conversations(let circle):
        CircleConversationsSheet(
          circle: circle,
          onDismiss: { presentedSheet = nil }
        )
      }
    }
    .alert(
      "Delete Circle?",
      isPresented: deletePresented,
      presenting: pendingDelete
    ) { circle in
      Button("Delete", role: .destructive) {
        Task {
          if await model.deleteCircle(circle, account: session.handle),
            navigation.section == .circle(circle.circleId)
          {
            navigation.section = .chats
          }
          pendingDelete = nil
        }
      }
      Button("Cancel", role: .cancel) {
        pendingDelete = nil
      }
    } message: { circle in
      Text("“\(circle.name)” and its conversation assignments will be removed.")
    }
    .alert(
      "Circle Action Failed",
      isPresented: errorPresented
    ) {
      Button("OK") {
        model.clearError()
      }
    } message: {
      Text(model.errorMessage ?? "")
    }
  }

  private var sidebarBackgroundOverlay: Color {
    colorScheme == .dark
      ? Color.white.opacity(0.01)
      : Color.black.opacity(0.03)
  }

  private var sidebarDivider: some View {
    Capsule()
      .fill(theme.listSelected)
      .frame(height: 1.5)
      .padding(.horizontal, 8)
  }

  private var profile: some View {
    SidebarItem(
      title: session.profile.fullName,
      subtitle: session.profile.identityNumber,
      selected: navigation.section == .settings,
      collapsed: collapsed,
      count: 0,
      mutedCount: 0,
      action: {
        navigation.section = .settings
      }
    ) {
      MixinRemoteImage(url: URL(string: session.profile.avatarUrl)) { image in
        image
          .resizable()
          .scaledToFill()
      } placeholder: {
        Image(systemName: "person.crop.circle.fill")
          .resizable()
          .foregroundStyle(theme.secondaryText)
      }
      .frame(width: 24, height: 24)
      .clipShape(Circle())
      .padding(.vertical, 8)
    }
  }

  private func category(
    _ title: String,
    assetName: String,
    section: HomeSection
  ) -> some View {
    sidebarItem(
      title: title,
      assetName: assetName,
      section: section,
      count: model.unseenCount(for: section)
    )
  }

  private func sidebarItem(
    title: String,
    assetName: String,
    iconColor: Color? = nil,
    section: HomeSection,
    count: (count: Int64, muted: Int64)
  ) -> some View {
    SidebarItem(
      title: title,
      assetName: assetName,
      iconColor: iconColor,
      selected: navigation.section == section,
      collapsed: collapsed,
      count: count.count,
      mutedCount: count.muted,
      action: {
        navigation.section = section
      }
    )
  }

  private var deletePresented: Binding<Bool> {
    Binding(
      get: { pendingDelete != nil },
      set: { if !$0 { pendingDelete = nil } }
    )
  }

  private var errorPresented: Binding<Bool> {
    Binding(
      get: { model.errorMessage != nil },
      set: { if !$0 { model.clearError() } }
    )
  }
}

private struct SidebarItem<Icon: View>: View {
  @Environment(\.colorScheme) private var colorScheme
  @Environment(\.mixinTheme) private var theme
  @State private var hovering = false

  let title: String
  var subtitle: String?
  var assetName: String?
  var iconColor: Color?
  let selected: Bool
  let collapsed: Bool
  let count: Int64
  let mutedCount: Int64
  let action: () -> Void
  @ViewBuilder let customIcon: () -> Icon

  init(
    title: String,
    subtitle: String? = nil,
    assetName: String? = nil,
    iconColor: Color? = nil,
    selected: Bool,
    collapsed: Bool,
    count: Int64,
    mutedCount: Int64,
    action: @escaping () -> Void,
    @ViewBuilder icon: @escaping () -> Icon
  ) {
    self.title = title
    self.subtitle = subtitle
    self.assetName = assetName
    self.iconColor = iconColor
    self.selected = selected
    self.collapsed = collapsed
    self.count = count
    self.mutedCount = mutedCount
    self.action = action
    self.customIcon = icon
  }

  var body: some View {
    Button(action: action) {
      ZStack(alignment: .topLeading) {
        HStack(spacing: 8) {
          sidebarIcon

          if !collapsed {
            VStack(alignment: .leading, spacing: 2) {
              Text(title)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)
                .lineLimit(1)

              if let subtitle {
                Text(subtitle)
                  .font(.system(size: 12))
                  .foregroundStyle(theme.secondaryText)
                  .lineLimit(1)
              }
            }

            Spacer(minLength: 4)

            if count > 0 {
              SidebarBadge(count: count)
            }
          }
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)

        if collapsed, count > 0 {
          Circle()
            .fill(collapsedBadgeColor)
            .frame(width: 6, height: 6)
            .offset(x: 28, y: 6)
        }
      }
      .frame(minHeight: 40)
      .background(itemBackground, in: RoundedRectangle(cornerRadius: 8))
      .contentShape(RoundedRectangle(cornerRadius: 8))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .help(collapsed ? title : "")
    .accessibilityLabel(title)
    .accessibilityValue(count > 0 ? "\(count) unread" : "")
  }

  @ViewBuilder
  private var sidebarIcon: some View {
    if let assetName {
      Image(assetName)
        .resizable()
        .renderingMode(.template)
        .foregroundStyle(iconColor ?? theme.text)
        .frame(width: 24, height: 24)
    } else {
      customIcon()
    }
  }

  private var itemBackground: Color {
    if selected {
      return theme.sidebarSelected
    }
    if hovering {
      return theme.sidebarSelected.opacity(0.5)
    }
    return .clear
  }

  private var collapsedBadgeColor: Color {
    if count != mutedCount {
      return Color(red: 1, green: 90 / 255, blue: 95 / 255)
    }
    return colorScheme == .dark
      ? Color.white.opacity(0.4)
      : Color(red: 51 / 255, green: 51 / 255, blue: 51 / 255).opacity(0.16)
  }
}

extension SidebarItem where Icon == EmptyView {
  init(
    title: String,
    subtitle: String? = nil,
    assetName: String,
    iconColor: Color? = nil,
    selected: Bool,
    collapsed: Bool,
    count: Int64,
    mutedCount: Int64,
    action: @escaping () -> Void
  ) {
    self.init(
      title: title,
      subtitle: subtitle,
      assetName: assetName,
      iconColor: iconColor,
      selected: selected,
      collapsed: collapsed,
      count: count,
      mutedCount: mutedCount,
      action: action,
      icon: { EmptyView() }
    )
  }
}

private struct SidebarBadge: View {
  @Environment(\.colorScheme) private var colorScheme
  @Environment(\.mixinTheme) private var theme

  let count: Int64

  var body: some View {
    Text("\(count)")
      .font(.system(size: 12))
      .foregroundStyle(theme.text)
      .lineLimit(1)
      .padding(.horizontal, 5)
      .frame(minWidth: 26, minHeight: 20)
      .background(background, in: Capsule())
  }

  private var background: Color {
    colorScheme == .dark
      ? Color.white.opacity(0.4)
      : Color(red: 51 / 255, green: 51 / 255, blue: 51 / 255).opacity(0.16)
  }
}

private let sidebarCircleColors: [Color] = [
  Color(red: 142 / 255, green: 123 / 255, blue: 1),
  Color(red: 101 / 255, green: 124 / 255, blue: 251 / 255),
  Color(red: 167 / 255, green: 57 / 255, blue: 194 / 255),
  Color(red: 189 / 255, green: 109 / 255, blue: 218 / 255),
  Color(red: 253 / 255, green: 137 / 255, blue: 241 / 255),
  Color(red: 250 / 255, green: 123 / 255, blue: 149 / 255),
  Color(red: 233 / 255, green: 65 / 255, blue: 86 / 255),
  Color(red: 250 / 255, green: 150 / 255, blue: 82 / 255),
  Color(red: 241 / 255, green: 210 / 255, blue: 43 / 255),
  Color(red: 186 / 255, green: 227 / 255, blue: 97 / 255),
  Color(red: 94 / 255, green: 221 / 255, blue: 94 / 255),
  Color(red: 75 / 255, green: 230 / 255, blue: 1),
  Color(red: 69 / 255, green: 183 / 255, blue: 254 / 255),
  Color(red: 0, green: 236 / 255, blue: 208 / 255),
  Color(red: 1, green: 204 / 255, blue: 192 / 255),
  Color(red: 206 / 255, green: 160 / 255, blue: 107 / 255),
]

private func circleColor(_ circleID: String) -> Color {
  guard let uuid = UUID(uuidString: circleID) else {
    return sidebarCircleColors[0]
  }
  let bytes = withUnsafeBytes(of: uuid.uuid) { Array($0) }
  var high: UInt64 = 0
  var low: UInt64 = 0
  for byte in bytes.prefix(8) {
    high = (high << 8) | UInt64(byte)
  }
  for byte in bytes.suffix(8) {
    low = (low << 8) | UInt64(byte)
  }
  let mixed = high ^ low
  let hash = UInt32(truncatingIfNeeded: mixed >> 32) ^ UInt32(truncatingIfNeeded: mixed)
  return sidebarCircleColors[Int(hash % UInt32(sidebarCircleColors.count))]
}

private enum CircleSheet: Identifiable {
  case rename(SwiftCircleItem)
  case conversations(SwiftCircleItem)

  var id: String {
    switch self {
    case .rename(let circle):
      "rename:\(circle.circleId)"
    case .conversations(let circle):
      "conversations:\(circle.circleId)"
    }
  }
}
