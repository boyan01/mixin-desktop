import AppKit
import SwiftUI
import UniformTypeIdentifiers
import UserNotifications

enum SettingsDestination: String, CaseIterable, Identifiable {
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
  @Environment(AppModel.self) private var appModel
  @Environment(AccountSession.self) private var session
  @State private var destination: SettingsDestination? = .profile
  @State private var confirmSignOut = false

  var body: some View {
    NavigationSplitView {
      settingsList
        .navigationSplitViewColumnWidth(min: 300, ideal: 300, max: 300)
    } detail: {
      settingsDetail
    }
    .navigationSplitViewStyle(.balanced)
    .toolbar(removing: .sidebarToggle)
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
  }

  private var settingsList: some View {
    ScrollView {
      VStack(spacing: 0) {
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
      .padding(.top, 64)
    }
    .scrollIndicators(.hidden)
    .background(theme.background)
  }

  private var profileHeader: some View {
    VStack(spacing: 0) {
      MixinRemoteImage(url: URL(string: session.profile.avatarUrl)) { image in
        image.resizable().scaledToFill()
      } placeholder: {
        Image(systemName: "person.crop.circle.fill")
          .resizable()
          .foregroundStyle(.secondary)
      }
      .frame(width: 90, height: 90)
      .clipShape(Circle())

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
        .foregroundStyle(theme.secondaryText)
        .textSelection(.enabled)
    }
    .frame(maxWidth: .infinity)
  }

  private func settingsRow(_ item: SettingsDestination) -> some View {
    Button {
      destination = item
    } label: {
      SettingsCellContent(
        title: item.title,
        assetName: item.assetName,
        systemImage: item == .mcp
          ? "point.3.connected.trianglepath.dotted"
          : nil,
        selected: destination == item
      )
    }
    .buttonStyle(.plain)
  }

  @ViewBuilder
  private var settingsDetail: some View {
    ZStack {
      theme.chatBackground
        .ignoresSafeArea()
      settingsDetailContent
    }
  }

  @ViewBuilder
  private var settingsDetailContent: some View {
    switch destination ?? .profile {
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
  @Environment(\.mixinTheme) private var theme

  let title: String
  var assetName: String?
  var systemImage: String?
  var color: Color?
  var selected = false
  var showsArrow = true

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

      if showsArrow {
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
    .background(selected ? theme.sidebarSelected : Color.clear)
    .contentShape(Rectangle())
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
  @State private var fullName = ""
  @State private var biography = ""
  @State private var saving = false
  @State private var uploadingAvatar = false
  @State private var selectingAvatar = false
  @State private var error: String?

  var body: some View {
    Form {
      Section("Profile") {
        HStack {
          Spacer()
          Button {
            selectingAvatar = true
          } label: {
            ZStack(alignment: .bottomTrailing) {
              MixinRemoteImage(url: URL(string: session.profile.avatarUrl)) { image in
                image.resizable().scaledToFill()
              } placeholder: {
                Image(systemName: "person.crop.circle.fill")
                  .resizable()
                  .foregroundStyle(.secondary)
              }
              .frame(width: 96, height: 96)
              .clipShape(Circle())

              Image(systemName: "camera.fill")
                .padding(7)
                .foregroundStyle(.white)
                .background(.tint, in: Circle())
            }
            .overlay {
              if uploadingAvatar {
                Circle()
                  .fill(.black.opacity(0.42))
                ProgressView()
                  .controlSize(.small)
                  .tint(.white)
              }
            }
          }
          .buttonStyle(.plain)
          .disabled(uploadingAvatar)
          .accessibilityLabel("Change Profile Photo")
          Spacer()
        }
        TextField("Full Name", text: $fullName)
          .textFieldStyle(.roundedBorder)
          .onChange(of: fullName) { _, value in
            if value.count > 40 {
              fullName = String(value.prefix(40))
            }
          }
        Text("\(fullName.count)/40")
          .font(.caption)
          .foregroundStyle(.secondary)
        TextField("Biography", text: $biography, axis: .vertical)
          .textFieldStyle(.roundedBorder)
          .lineLimit(3...8)
          .onChange(of: biography) { _, value in
            if value.count > 140 {
              biography = String(value.prefix(140))
            }
          }
        Text("\(biography.count)/140")
          .font(.caption)
          .foregroundStyle(.secondary)
        LabeledContent("Mixin ID", value: session.profile.identityNumber)
        LabeledContent("Phone", value: session.profile.phone)
        LabeledContent(
          "Joined",
          value: (try? Date.ISO8601FormatStyle()
            .parseStrategy
            .parse(session.profile.createdAt))?
            .formatted(date: .abbreviated, time: .omitted)
            ?? session.profile.createdAt
        )
      }

      if let error {
        Text(error)
          .foregroundStyle(.red)
      }

      Button {
        save()
      } label: {
        if saving {
          ProgressView()
            .controlSize(.small)
        } else {
          Text("Save Changes")
        }
      }
      .disabled(
        saving
          || fullName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          || (fullName == session.profile.fullName
            && biography == session.profile.biography)
      )
    }
    .formStyle(.grouped)
    .settingsFormLayout()
    .navigationTitle("Edit Profile")
    .fileImporter(
      isPresented: $selectingAvatar,
      allowedContentTypes: [.image],
      allowsMultipleSelection: false
    ) { result in
      guard case .success(let urls) = result, let url = urls.first else {
        if case .failure(let importError) = result {
          error = MixinErrorPresenter.message(for: importError)
        }
        return
      }
      updateAvatar(from: url)
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

  private func updateAvatar(from url: URL) {
    uploadingAvatar = true
    error = nil
    Task {
      let accessing = url.startAccessingSecurityScopedResource()
      defer {
        if accessing {
          url.stopAccessingSecurityScopedResource()
        }
        uploadingAvatar = false
      }
      do {
        let avatar = try await ProfileImageProcessor.avatarBase64(from: url)
        try await session.updateAvatar(avatar)
      } catch {
        self.error = MixinErrorPresenter.message(for: error)
      }
    }
  }
}

private struct NotificationSettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @Environment(SettingsPreferencesModel.self) private var preferences
  @State private var notificationsAuthorized: Bool?

  var body: some View {
    ScrollView {
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
          .controlSize(.small)
        }
        .padding(.horizontal, 16)
        .frame(height: 58)
        .background(
          theme.settingCellBackground,
          in: RoundedRectangle(cornerRadius: 8)
        )

        Text("Show the sender and message content in notifications.")
          .font(.system(size: 14))
          .foregroundStyle(theme.secondaryText)
          .padding(.leading, 10)
          .padding(.top, 10)

        if notificationsAuthorized == false {
          Button("Open Notification Settings") {
            guard
              let url = URL(
                string: "x-apple.systempreferences:com.apple.Notifications-Settings.extension"
              )
            else {
              return
            }
            NSWorkspace.shared.open(url)
          }
          .buttonStyle(.plain)
          .font(.system(size: 16))
          .foregroundStyle(theme.text)
          .padding(.horizontal, 16)
          .frame(maxWidth: .infinity, minHeight: 58, alignment: .leading)
          .background(
            theme.settingCellBackground,
            in: RoundedRectangle(cornerRadius: 8)
          )
          .padding(.top, 24)

          Text("Notifications are disabled for Mixin in System Settings.")
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)
            .padding(.leading, 10)
            .padding(.top, 10)
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
  @Environment(SettingsPreferencesModel.self) private var preferences

  var body: some View {
    Form {
      Section("Theme") {
        Picker(
          "Theme",
          selection: Binding(
            get: { preferences.theme },
            set: { preferences.setTheme($0) }
          )
        ) {
          Text("Follow System").tag("system")
          Text("Light").tag("light")
          Text("Dark").tag("dark")
        }
        .pickerStyle(.radioGroup)
      }

      Section("Chats") {
        Toggle(
          "Show Avatar",
          isOn: Binding(
            get: { preferences.showAvatar },
            set: { preferences.setShowAvatar($0) }
          ))
        Toggle(
          "Show Identity Number",
          isOn: Binding(
            get: { preferences.showIdentityNumber },
            set: { preferences.setShowIdentityNumber($0) }
          ))
      }

      Section("Chat Text Size") {
        HStack {
          Text("A").font(.caption)
          Slider(
            value: Binding(
              get: { preferences.chatFontSizeDelta },
              set: { preferences.setChatFontSizeDelta($0) }
            ),
            in: -2...4,
            step: 1
          )
          Text("A").font(.title2)
        }
        AppearanceChatPreview(
          fontSizeDelta: preferences.chatFontSizeDelta
        )
      }
    }
    .formStyle(.grouped)
    .settingsFormLayout()
    .navigationTitle("Appearance")
  }
}

private struct AppearanceChatPreview: View {
  let fontSizeDelta: Double

  var body: some View {
    ZStack {
      ChatBackgroundView()

      VStack(spacing: 10) {
        Text("Jan 1")
          .font(.caption2)
          .foregroundStyle(.secondary)
          .padding(.horizontal, 8)
          .padding(.vertical, 3)
          .background(.regularMaterial, in: Capsule())

        previewBubble(
          "Say hi",
          outgoing: true
        )
        previewBubble(
          "I am good",
          outgoing: false
        )
      }
      .padding(16)
    }
    .frame(maxWidth: .infinity, minHeight: 160)
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
        .font(.system(size: 15 + fontSizeDelta))
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(
          outgoing
            ? Color.accentColor.opacity(0.18)
            : Color(nsColor: .controlBackgroundColor)
        )
        .clipShape(RoundedRectangle(cornerRadius: 10))
      if !outgoing {
        Spacer(minLength: 60)
      }
    }
  }
}

private struct AboutSettingsView: View {
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
    Form {
      Section {
        Image("AboutLogo")
          .resizable()
          .scaledToFit()
          .frame(width: 60, height: 60)
          .frame(maxWidth: .infinity)
          .padding(.vertical, 12)
          .onTapGesture(count: 5) {
            logsPresented = true
          }
        Text("Mixin Messenger")
          .font(.headline)
          .frame(maxWidth: .infinity)
          .onTapGesture(count: 7) {
            debugMode = true
          }
        LabeledContent("Version", value: version)
      }
      Section {
        Link(
          "Follow Us on X",
          destination: URL(string: "https://x.com/MixinMessenger")!
        )
        Link(
          "Follow Us on Facebook",
          destination: URL(string: "https://fb.com/MixinMessenger")!
        )
        Link("Mixin Website", destination: URL(string: "https://mixin.one/")!)
        Link("Help Center", destination: URL(string: "https://support.mixin.one/")!)
        Link(
          "Terms of Service",
          destination: URL(string: "https://mixin.one/pages/terms")!
        )
        Link(
          "Privacy Policy",
          destination: URL(string: "https://mixin.one/pages/privacy")!
        )
      }
      if debugMode {
        Section("Diagnostics") {
          Button("Open Log Directory") {
            openLogDirectory()
          }
          Button("View Logs") {
            logsPresented = true
          }
        }
      }
    }
    .formStyle(.grouped)
    .settingsFormLayout()
    .navigationTitle("About")
    .sheet(isPresented: $logsPresented) {
      LogViewerSheet(desktop: desktop)
    }
  }

  private func openLogDirectory() {
    guard let path = try? desktop.logDirectory() else {
      return
    }
    NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: path)
  }
}
