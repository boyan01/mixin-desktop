import AppKit
import SwiftUI
import UniformTypeIdentifiers
import UserNotifications

enum SettingsDestination: String, CaseIterable, Hashable, Identifiable {
  case profile
  case notifications
  case storage
  case security
  case proxy
  case appearance
  case mcp
  case about

  var id: String { rawValue }

  var title: String {
    switch self {
    case .profile: "Edit Profile"
    case .notifications: "Notifications"
    case .storage: "Data and Storage"
    case .security: "Security"
    case .proxy: "Proxy"
    case .appearance: "Appearance"
    case .mcp: "Local MCP Server"
    case .about: "About"
    }
  }

  var assetName: String? {
    switch self {
    case .profile: "SettingsProfile"
    case .notifications: "SettingsNotification"
    case .storage: "SettingsStorage"
    case .security: "SettingsSecurity"
    case .proxy: "SettingsProxy"
    case .appearance: "SettingsAppearance"
    case .mcp: nil
    case .about: "SettingsAbout"
    }
  }
}

struct SettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @Environment(\.colorScheme) private var colorScheme
  @Environment(AppModel.self) private var appModel
  @Environment(AccountSession.self) private var session
  @Environment(HomeNavigationModel.self) private var navigation
  @State private var confirmSignOut = false
  @State private var notificationPermissionDenied = false

  let routeMode: Bool
  var onShowSidebar: (() -> Void)?

  var body: some View {
    Group {
      if routeMode {
        settingsList
      } else {
        HStack(spacing: 0) {
          settingsList
            .frame(width: 300)
          Divider()
          SettingsDetailView(destination: navigation.settingsDestination)
        }
      }
    }
    .confirmationDialog(
      "Sign out of Mixin?",
      isPresented: $confirmSignOut
    ) {
      Button("Sign Out", role: .destructive) {
        Task {
          await appModel.signOut()
        }
      }
      Button("Cancel", role: .cancel) {}
    }
    .task {
      let settings = await UNUserNotificationCenter.current().notificationSettings()
      notificationPermissionDenied = settings.authorizationStatus == .denied
    }
  }

  private var settingsList: some View {
    AppScrollView {
      VStack(spacing: 0) {
        settingsHeader
        profileHeader

        Spacer()
          .frame(height: 24)

        SettingsCellGroup {
          settingsRow(.profile)
        }

        SettingsCellGroup {
          VStack(spacing: 0) {
            settingsRow(.notifications)
            settingsRow(.storage)
            settingsRow(.security)
            settingsRow(.proxy)
            settingsRow(.appearance)
            settingsRow(.mcp)
            settingsRow(.about)
          }
        }

        SettingsCellGroup {
          Button {
            confirmSignOut = true
          } label: {
            SettingsCellContent(
              title: "Sign Out",
              assetName: "SettingsSignOut",
              color: theme.destructive,
              showsArrow: false
            )
          }
          .buttonStyle(.plain)
        }
      }
    }
    .scrollIndicators(.hidden)
    .background(theme.background)
  }

  private var settingsHeader: some View {
    HStack(spacing: 0) {
      if let onShowSidebar {
        Button(action: onShowSidebar) {
          Image(systemName: "line.3.horizontal")
            .frame(width: 32, height: 32)
        }
        .buttonStyle(.plain)
        .help("Show Sidebar")

        Text("Settings")
          .font(.system(size: 16, weight: .semibold))
          .foregroundStyle(theme.text)
          .padding(.leading, 8)
      }
      Spacer()
    }
    .padding(.horizontal, 16)
    .frame(height: 64)
  }

  private var profileHeader: some View {
    VStack(spacing: 0) {
      UserAvatar(
        userID: session.profile.userId,
        name: session.profile.fullName,
        url: session.profile.avatarUrl,
        size: 90
      )

      Spacer()
        .frame(height: 10)

      HStack(spacing: 4) {
        Text(session.profile.fullName)
          .font(.system(size: 18, weight: .semibold))
          .foregroundStyle(theme.text)
          .multilineTextAlignment(.center)
          .lineLimit(2)
        ProfileIdentityBadge(
          isVerified: session.profile.isVerified,
          isBot: false,
          membership: session.profile.membership
        )
      }
      .padding(.horizontal, 20)

      Spacer()
        .frame(height: 4)

      Text("Mixin ID: \(session.profile.identityNumber)")
        .font(.system(size: 14))
        .foregroundStyle(
          colorScheme == .dark
            ? Color.white.opacity(0.4)
            : Color(red: 188 / 255, green: 190 / 255, blue: 195 / 255)
        )
        .textSelection(.enabled)
    }
    .frame(maxWidth: .infinity)
  }

  private func settingsRow(_ item: SettingsDestination) -> some View {
    Button {
      navigation.showSettingsDestination(item)
    } label: {
      SettingsCellContent(
        title: item.title,
        assetName: item.assetName,
        systemImage: item == .mcp
          ? "point.3.connected.trianglepath.dotted"
          : nil,
        color: item == .notifications && notificationPermissionDenied
          ? theme.destructive
          : nil,
        selected: navigation.settingsDestination == item,
        showsWarning: item == .notifications && notificationPermissionDenied
      )
    }
    .buttonStyle(.plain)
  }
}

struct SettingsDetailView: View {
  @Environment(\.mixinTheme) private var theme
  @Environment(AppModel.self) private var appModel
  @Environment(AccountSession.self) private var session

  let destination: SettingsDestination

  var body: some View {
    ZStack {
      theme.chatBackground
        .ignoresSafeArea()
      settingsDetailContent
    }
  }

  @ViewBuilder
  private var settingsDetailContent: some View {
    switch destination {
    case .profile:
      EditProfileSettingsView()
    case .notifications:
      NotificationSettingsView()
    case .appearance:
      AppearanceSettingsView()
    case .about:
      if let desktop = appModel.desktopHandle {
        AboutSettingsView(desktop: desktop)
      }
    case .storage:
      if let desktop = appModel.desktopHandle {
        DataStorageSettingsView(desktop: desktop, account: session.handle)
      }
    case .security:
      SecuritySettingsView()
    case .proxy:
      if let desktop = appModel.desktopHandle {
        ProxySettingsView(desktop: desktop)
      }
    case .mcp:
      if let desktop = appModel.desktopHandle {
        McpSettingsView(desktop: desktop)
      }
    }
  }
}

private struct SettingsCellGroup<Content: View>: View {
  @Environment(\.mixinTheme) private var theme
  @ViewBuilder let content: () -> Content

  var body: some View {
    content()
      .background(theme.listSelected)
      .clipShape(RoundedRectangle(cornerRadius: 8))
      .padding(.horizontal, 10)
      .padding(.bottom, 10)
  }
}

private struct SettingsCellContent: View {
  @Environment(\.colorScheme) private var colorScheme
  @Environment(\.mixinTheme) private var theme

  let title: String
  var assetName: String?
  var systemImage: String?
  var color: Color?
  var selected = false
  var showsArrow = true
  var showsWarning = false

  var body: some View {
    HStack(spacing: 8) {
      if let assetName {
        Image(assetName)
          .resizable()
          .renderingMode(.template)
          .foregroundStyle(color ?? theme.text)
          .frame(width: 24, height: 24)
      } else if let systemImage {
        Image(systemName: systemImage)
          .font(.system(size: 19))
          .foregroundStyle(color ?? theme.text)
          .frame(width: 24, height: 24)
      }

      Text(title)
        .font(.system(size: 16))
        .foregroundStyle(color ?? theme.text)
        .lineLimit(1)

      Spacer(minLength: 4)

      if showsWarning {
        Image("Warning")
          .resizable()
          .renderingMode(.template)
          .foregroundStyle(theme.destructive)
          .frame(width: 22, height: 22)
          .padding(4)
      } else if showsArrow {
        Image("SettingsArrow")
          .resizable()
          .renderingMode(.template)
          .foregroundStyle(theme.secondaryText)
          .frame(width: 30, height: 30)
      }
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 17)
    .background(selected ? selectedBackground : Color.clear)
    .contentShape(Rectangle())
  }

  private var selectedBackground: Color {
    colorScheme == .dark ? Color.white.opacity(0.06) : Color.black.opacity(0.05)
  }
}

private struct SettingsFormLayoutModifier: ViewModifier {
  @Environment(\.mixinTheme) private var theme

  func body(content: Content) -> some View {
    content
      .scrollContentBackground(.hidden)
      .contentMargins(.top, 32, for: .scrollContent)
      .frame(maxWidth: 620)
      .frame(maxWidth: .infinity)
      .background(theme.chatBackground)
  }
}

extension View {
  func settingsFormLayout() -> some View {
    modifier(SettingsFormLayoutModifier())
  }
}

private struct BackupSettingsView: View {
  @Environment(AccountSession.self) private var session

  var body: some View {
    ContentUnavailableView {
      Label("Chat Backup", systemImage: "arrow.triangle.2.circlepath")
    } description: {
      Text("Transfer your chat history and attachments between Mixin devices.")
    } actions: {
      Button("Back Up or Restore") {
        session.deviceTransfer.openSetup()
      }
      .buttonStyle(.borderedProminent)
    }
    .navigationTitle("Chat Backup")
  }
}

private struct EditProfileSettingsView: View {
  @Environment(AccountSession.self) private var session
  @Environment(\.mixinTheme) private var theme
  @Environment(\.colorScheme) private var colorScheme
  @State private var fullName = ""
  @State private var biography = ""
  @State private var saving = false
  @State private var error: String?
  @State private var formWidth: CGFloat = 0

  var body: some View {
    VStack(spacing: 0) {
      profileHeader
      AppScrollView {
        VStack(spacing: 0) {
          profileField("Name", text: $fullName, limit: 40)
          Spacer().frame(height: 32)
          profileField("Biography", text: $biography, limit: 140)
          Spacer().frame(height: 32)
          profileField("Phone Number", text: .constant(session.profile.phone), readOnly: true)
          Spacer().frame(height: 70)
          Text(joinedText)
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)
          Spacer().frame(height: 48)
        }
        .background {
          GeometryReader { proxy in
            Color.clear
              .onAppear {
                formWidth = proxy.size.width
              }
              .onChange(of: proxy.size.width) {
                formWidth = $0
              }
          }
        }
      }
    }
    .background(theme.background)
    .navigationTitle("Edit Profile")
    .toolbar {
      ToolbarItem(placement: .primaryAction) {
        Button("Save") { save() }
          .disabled(saving || fullName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
      }
    }
    .task {
      fullName = session.profile.fullName
      biography = session.profile.biography
      do {
        try await session.refreshProfile()
        guard !Task.isCancelled else {
          return
        }
        fullName = session.profile.fullName
        biography = session.profile.biography
      } catch {
        // Keep the cached profile when the remote refresh fails.
      }
    }
  }

  private var profileHeader: some View {
    VStack(spacing: 0) {
      Spacer().frame(height: 40)
      UserAvatar(
        userID: session.profile.userId,
        name: session.profile.fullName,
        url: session.profile.avatarUrl,
        size: 100
      )
      Spacer().frame(height: 10)
      Text("Mixin ID: \(session.profile.identityNumber)")
        .font(.system(size: 14))
        .foregroundStyle(
          colorScheme == .dark
            ? Color.white.opacity(0.4)
            : Color(red: 188 / 255, green: 190 / 255, blue: 195 / 255)
        )
        .textSelection(.enabled)
      Spacer().frame(height: 32)
    }
  }

  private func profileField(
    _ title: String,
    text: Binding<String>,
    limit: Int? = nil,
    readOnly: Bool = false
  ) -> some View {
    VStack(alignment: .leading, spacing: 16) {
      Text(title)
        .font(.system(size: 14, weight: .medium))
        .foregroundStyle(theme.secondaryText)
      TextField("", text: text, axis: .vertical)
        .textFieldStyle(.plain)
        .font(.system(size: 16))
        .foregroundStyle(readOnly ? theme.secondaryText : theme.text)
        .lineLimit(1...10)
        .disabled(readOnly)
        .padding(.horizontal, 20)
        .padding(.vertical, 24)
        .background(
          readOnly ? theme.settingCellBackground : theme.listSelected,
          in: RoundedRectangle(cornerRadius: 8)
        )
        .onChange(of: text.wrappedValue) {
          if let limit, text.wrappedValue.count > limit {
            text.wrappedValue = String(text.wrappedValue.prefix(limit))
          }
        }
    }
    .padding(.horizontal, horizontalInset)
  }

  private var horizontalInset: CGFloat {
    min(90, max(20, (formWidth - 500) / 2))
  }

  private var joinedText: String {
    guard let date = try? Date.ISO8601FormatStyle().parseStrategy.parse(session.profile.createdAt) else {
      return ""
    }
    return "Joined in \(date.formatted(date: .abbreviated, time: .omitted))"
  }

  private func save() {
    saving = true
    error = nil
    Task {
      do {
        try await session.updateProfile(
          fullName: fullName.trimmingCharacters(in: .whitespacesAndNewlines),
          biography: biography
        )
      } catch {
        self.error = MixinErrorPresenter.message(for: error)
      }
      saving = false
    }
  }

}

private struct NotificationSettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @Environment(SettingsPreferencesModel.self) private var preferences
  @State private var notificationsAuthorized: Bool?

  var body: some View {
    AppScrollView {
      VStack(alignment: .leading, spacing: 0) {
        HStack {
          Text("Message Preview")
            .font(.system(size: 16))
            .foregroundStyle(theme.text)
          Spacer()
          Toggle(
            "",
            isOn: Binding(
              get: { preferences.messagePreview },
              set: { preferences.setMessagePreview($0) }
            )
          )
          .labelsHidden()
          .toggleStyle(.switch)
          .scaleEffect(0.7)
        }
        .padding(.leading, 16)
        .padding(.trailing, 10)
        .padding(.vertical, 17)
        .background(
          theme.settingCellBackground,
          in: RoundedRectangle(cornerRadius: 8)
        )

        Text("Preview message text inside new message notifications.")
          .font(.system(size: 14))
          .foregroundStyle(theme.secondaryText)
          .padding(.leading, 10)
          .padding(.top, 10)
          .padding(.bottom, 14)

        if notificationsAuthorized == false {
          Button {
            guard
              let url = URL(
                string: "x-apple.systempreferences:com.apple.preference.notifications"
              )
            else {
              return
            }
            NSWorkspace.shared.open(url)
          } label: {
            HStack(spacing: 4) {
              Text("Enable Push Notifications")
              Spacer(minLength: 0)
              Image("SettingsArrow")
                .resizable()
                .renderingMode(.template)
                .foregroundStyle(theme.secondaryText)
                .frame(width: 30, height: 30)
            }
          }
          .buttonStyle(.plain)
          .font(.system(size: 16))
          .foregroundStyle(theme.text)
          .padding(.leading, 16)
          .padding(.trailing, 10)
          .padding(.vertical, 17)
          .frame(maxWidth: .infinity, alignment: .leading)
          .background(
            theme.settingCellBackground,
            in: RoundedRectangle(cornerRadius: 8)
          )

          Text(
            "Enable push notifications to stay updated on price alerts and messages in real time."
          )
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)
            .padding(.leading, 10)
            .padding(.top, 10)
            .padding(.bottom, 14)
        }
      }
      .frame(maxWidth: 600)
      .padding(.horizontal, 10)
      .padding(.top, 40)
      .frame(maxWidth: .infinity)
    }
    .background(theme.chatBackground)
    .navigationTitle("Notifications")
    .task {
      let center = UNUserNotificationCenter.current()
      var settings = await center.notificationSettings()
      if settings.authorizationStatus == .notDetermined {
        _ = try? await center.requestAuthorization(
          options: [.alert, .badge, .sound]
        )
        settings = await center.notificationSettings()
      }
      notificationsAuthorized =
        settings.authorizationStatus == .authorized
        || settings.authorizationStatus == .provisional
    }
  }
}

private struct AppearanceSettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @Environment(SettingsPreferencesModel.self) private var preferences

  var body: some View {
    AppScrollView {
      VStack(alignment: .leading, spacing: 0) {
        sectionTitle("Theme", top: 20)
        appearanceGroup {
          VStack(spacing: 0) {
            radioRow("Follow System", value: "system")
            radioRow("Light", value: "light")
            radioRow("Dark", value: "dark")
          }
        }
        sectionTitle("Chats", top: 22)
        appearanceGroup {
          VStack(spacing: 0) {
            switchRow("Show Avatar", value: Binding(get: { preferences.showAvatar }, set: { preferences.setShowAvatar($0) }))
            switchRow("Show Identity Number", value: Binding(get: { preferences.showIdentityNumber }, set: { preferences.setShowIdentityNumber($0) }))
          }
        }
        sectionTitle("Chat Text Size", top: 22)
        AppearanceChatPreview(fontSizeDelta: preferences.chatFontSizeDelta)
          .frame(maxWidth: 600)
        Spacer().frame(height: 10)
        HStack(spacing: 10) {
          Text("A").font(.system(size: 12)).foregroundStyle(theme.text)
          Slider(value: Binding(get: { preferences.chatFontSizeDelta }, set: { preferences.setChatFontSizeDelta($0) }), in: -2...4, step: 1)
          Text("A").font(.system(size: 24)).foregroundStyle(theme.text)
        }
        .padding(.horizontal, 10)
        .frame(maxWidth: 600)
      }
      .padding(.horizontal, 10)
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .background(theme.background)
    .navigationTitle("Appearance")
  }

  private func sectionTitle(_ title: String, top: CGFloat) -> some View {
    Text(title)
      .font(.system(size: 14))
      .foregroundStyle(theme.secondaryText)
      .padding(.leading, 10)
      .padding(.top, top)
      .padding(.bottom, 14)
  }

  private func appearanceGroup<Content: View>(@ViewBuilder content: () -> Content) -> some View {
    content().background(theme.settingCellBackground, in: RoundedRectangle(cornerRadius: 8)).frame(maxWidth: 600)
  }

  private func radioRow(_ title: String, value: String) -> some View {
    Button { preferences.setTheme(value) } label: {
      HStack(spacing: 30) {
        ZStack {
          Circle().fill(preferences.theme == value ? theme.accent : theme.secondaryText)
          if preferences.theme == value {
            Text("✓").font(.system(size: 10)).foregroundStyle(.white)
          }
        }.frame(width: 16, height: 16)
        Text(title).font(.system(size: 16)).foregroundStyle(theme.text)
        Spacer()
      }
      .padding(.leading, 16)
      .padding(.trailing, 10)
      .padding(.vertical, 27)
      .contentShape(Rectangle())
    }.buttonStyle(.plain)
  }

  private func switchRow(_ title: String, value: Binding<Bool>) -> some View {
    HStack { Text(title).font(.system(size: 16)).foregroundStyle(theme.text); Spacer(); Toggle("", isOn: value).labelsHidden().scaleEffect(0.7) }
      .padding(.leading, 16).padding(.trailing, 10).frame(height: 58)
  }
}

private struct AppearanceChatPreview: View {
  @Environment(\.colorScheme) private var colorScheme
  let fontSizeDelta: Double

  var body: some View {
    ZStack {
      ChatBackgroundView()

      VStack(spacing: 0) {
        Text("Jan 1")
          .font(.system(size: 14))
          .foregroundStyle(.black)
          .frame(minWidth: 64)
          .padding(.horizontal, 10)
          .padding(.vertical, 2)
          .background(
            Color(red: 213 / 255, green: 211 / 255, blue: 243 / 255),
            in: RoundedRectangle(cornerRadius: 10)
          )
          .padding(.top, 16)
          .padding(.bottom, 10)

        previewBubble(
          "Say hi",
          outgoing: true
        )
        previewBubble(
          "I am good",
          outgoing: false
        )
      }
      .padding(.leading, 20)
      .padding(.trailing, 20)
      .padding(.top, 10)
      .padding(.bottom, 20)
    }
    .padding(.horizontal, 10)
    .clipShape(RoundedRectangle(cornerRadius: 8))
  }

  private func previewBubble(
    _ text: String,
    outgoing: Bool
  ) -> some View {
    HStack {
      if outgoing {
        Spacer(minLength: 60)
      }
      Text(text)
        .font(.system(size: 16 + fontSizeDelta))
        .padding(.horizontal, 8)
        .padding(.vertical, 8)
        .background(
          outgoing
            ? (colorScheme == .dark
              ? Color(red: 59 / 255, green: 79 / 255, blue: 103 / 255)
              : Color(red: 197 / 255, green: 237 / 255, blue: 253 / 255))
            : (colorScheme == .dark
              ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
              : Color.white)
        )
        .clipShape(RoundedRectangle(cornerRadius: 8))
      if !outgoing {
        Spacer(minLength: 60)
      }
    }
  }
}

private struct AboutSettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @State private var logsPresented = false
  @State private var debugMode = false
  let desktop: SwiftDesktopHandle

  private var version: String {
    let short =
      Bundle.main.object(
        forInfoDictionaryKey: "CFBundleShortVersionString"
      ) as? String ?? "1.0"
    let build =
      Bundle.main.object(
        forInfoDictionaryKey: "CFBundleVersion"
      ) as? String ?? ""
    return build.isEmpty ? short : "\(short) (\(build))"
  }

  var body: some View {
    AppScrollView {
      VStack(spacing: 0) {
        Spacer().frame(height: 40)
        Image("AboutLogo")
          .resizable()
          .scaledToFit()
          .frame(width: 60, height: 60)
          .onTapGesture(count: 5) {
            logsPresented = true
          }
        Spacer().frame(height: 24)
        Text("Mixin Messenger")
          .font(.system(size: 18))
          .foregroundStyle(theme.text)
          .onTapGesture(count: 7) {
            debugMode = true
          }
        Spacer().frame(height: 8)
        Text(version)
          .font(.system(size: 16))
          .foregroundStyle(theme.secondaryText)
          .textSelection(.enabled)
        Spacer().frame(height: 50)
        aboutGroup {
          VStack(spacing: 0) {
            aboutLink("Follow Us on X", "https://x.com/MixinMessenger")
            aboutLink("Follow Us on Facebook", "https://fb.com/MixinMessenger")
            aboutLink("Help Center", "https://support.mixin.one")
            aboutLink("Terms of Service", "https://mixin.one/pages/terms")
            aboutLink("Privacy Policy", "https://mixin.one/pages/privacy")
          }
        }
        if debugMode {
          aboutGroup { Button("Open Log Directory", action: openLogDirectory).buttonStyle(.plain).padding(.leading, 16).padding(.vertical, 17) }
        }
      }
    }
    .background(theme.background)
    .navigationTitle("About")
    .sheet(isPresented: $logsPresented) {
      LogViewerSheet(desktop: desktop)
    }
  }

  private func aboutGroup<Content: View>(@ViewBuilder content: () -> Content) -> some View {
    content().background(theme.settingCellBackground, in: RoundedRectangle(cornerRadius: 8)).padding(.horizontal, 10).padding(.bottom, 10).frame(maxWidth: 600)
  }

  private func aboutLink(_ title: String, _ destination: String) -> some View {
    Link(destination: URL(string: destination)!) {
      HStack(spacing: 4) { Text(title).font(.system(size: 16)).foregroundStyle(theme.text); Spacer(); Image("SettingsArrow").resizable().renderingMode(.template).foregroundStyle(theme.secondaryText).frame(width: 30, height: 30) }
        .padding(.leading, 16).padding(.trailing, 10).padding(.vertical, 17)
    }.buttonStyle(.plain)
  }

  private func openLogDirectory() {
    guard let path = try? desktop.logDirectory() else {
      return
    }
    NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: path)
  }
}
