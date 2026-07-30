import AppKit
import Observation
import SwiftUI

struct McpSettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @State private var model: McpSettingsModel
  @State private var copiedValue: CopiedValue?

  init(desktop: SwiftDesktopHandle) {
    _model = State(initialValue: McpSettingsModel(desktop: desktop))
  }

  var body: some View {
    Group {
      if let settings = model.settings {
        settingsContent(settings)
      } else if let error = model.error {
        Text("Failed to load MCP settings: \(error)")
          .font(.system(size: 16))
          .foregroundStyle(theme.text)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .background(theme.background)
      } else {
        ProgressView()
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .background(theme.background)
      }
    }
    .navigationTitle("Local MCP Server")
    .task {
      await model.load()
    }
    .overlay(alignment: .bottom) {
      if let copiedValue {
        Label("\(copiedValue.rawValue) copied", systemImage: "checkmark.circle.fill")
          .font(.caption)
          .padding(.horizontal, 14)
          .padding(.vertical, 8)
          .background(.regularMaterial, in: Capsule())
          .padding()
          .transition(.move(edge: .bottom).combined(with: .opacity))
      }
    }
    .animation(.easeInOut(duration: 0.18), value: copiedValue)
  }

  private func settingsContent(_ settings: McpSettingsItem) -> some View {
    AppScrollView {
      VStack(spacing: 0) {
        Text(model.statusText)
          .font(.system(size: 14))
          .foregroundStyle(theme.secondaryText)
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(.horizontal, 20)
          .padding(.top, 20)

        Spacer().frame(height: 12)

        mcpGroup {
          VStack(spacing: 0) {
            switchCell("Server", value: Binding(
              get: { settings.enabled },
              set: { enabled in Task { await model.update(enabled: enabled) } }
            ))
            if settings.enabled {
              Divider()
              valueCell(
                "Endpoint",
                label: model.status?.endpoint ?? "Not running",
                value: model.status?.endpoint,
                copied: .endpoint
              )
              Divider()
              valueCell(
                "Access Token",
                label: masked(settings.token),
                value: settings.token,
                copied: .token
              )
              Divider()
              switchCell("Draft Editing", value: Binding(
                get: { settings.draftToolsEnabled },
                set: { enabled in Task { await model.update(draftToolsEnabled: enabled) } }
              ))
              Divider()
              switchCell("Circle Management", value: Binding(
                get: { settings.circleManagementEnabled },
                set: { enabled in Task { await model.update(circleManagementEnabled: enabled) } }
              ))
            }
          }
        }

        Text("Local clients must use the bearer token. MCP never sends messages or changes account data.")
          .font(.system(size: 14))
          .foregroundStyle(theme.secondaryText)
          .frame(maxWidth: 600, alignment: .leading)
          .padding(.horizontal, 20)
          .padding(.top, 12)

        if let error = model.error {
          Text(error)
            .font(.system(size: 14))
            .foregroundStyle(theme.destructive)
            .padding(.horizontal, 20)
            .padding(.top, 12)
        }
      }
    }
    .background(theme.background)
  }

  private func mcpGroup<Content: View>(@ViewBuilder content: () -> Content) -> some View {
    content()
      .background(theme.settingCellBackground)
      .clipShape(RoundedRectangle(cornerRadius: 8))
      .padding(.horizontal, 10)
      .frame(maxWidth: 600)
  }

  private func switchCell(_ title: String, value: Binding<Bool>) -> some View {
    HStack {
      Text(title)
        .font(.system(size: 16))
        .foregroundStyle(theme.text)
      Spacer(minLength: 4)
      Toggle(title, isOn: value)
        .labelsHidden()
        .scaleEffect(0.75)
        .disabled(model.updating)
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 17)
  }

  private func valueCell(
    _ title: String,
    label: String,
    value: String?,
    copied: CopiedValue
  ) -> some View {
    HStack(spacing: 8) {
      VStack(alignment: .leading, spacing: 3) {
        Text(title)
          .font(.system(size: 16))
          .foregroundStyle(theme.text)
        Text(label)
          .font(.system(size: 14))
          .foregroundStyle(theme.secondaryText)
      }
      Spacer(minLength: 4)
      copyButton(value: value, copied: copied)
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 10)
  }

  private func copyButton(value: String?, copied: CopiedValue) -> some View {
    Button {
        guard let value else {
          return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(value, forType: .string)
        copiedValue = copied
        Task {
          try? await Task.sleep(for: .seconds(1.5))
          if copiedValue == copied {
            copiedValue = nil
          }
        }
    } label: {
      Image(systemName: "doc.on.doc")
        .font(.system(size: 20))
    }
    .buttonStyle(.plain)
    .disabled(value == nil)
    .help("Copy \(copied.rawValue)")
  }

  private func masked(_ token: String) -> String {
    guard token.count >= 8 else {
      return "••••••••"
    }
    return "\(token.prefix(4))••••\(token.suffix(4))"
  }
}

private enum CopiedValue: String {
  case endpoint = "Endpoint"
  case token = "Access token"
}

@MainActor
@Observable
private final class McpSettingsModel {
  private let desktop: SwiftDesktopHandle

  private(set) var settings: McpSettingsItem?
  private(set) var status: McpServerStatusItem?
  private(set) var updating = false
  private(set) var error: String?

  init(desktop: SwiftDesktopHandle) {
    self.desktop = desktop
  }

  var statusText: String {
    if status?.running == true {
      return "Running on localhost"
    }
    return status?.lastError ?? "Stopped"
  }

  func load() async {
    error = nil
    do {
      async let loadedSettings = desktop.mcpSettings()
      async let loadedStatus = desktop.mcpServerStatus()
      settings = try await loadedSettings
      status = try await loadedStatus
    } catch {
      self.error = MixinErrorPresenter.message(for: error)
    }
  }

  func update(
    enabled: Bool? = nil,
    draftToolsEnabled: Bool? = nil,
    circleManagementEnabled: Bool? = nil
  ) async {
    guard !updating, let current = settings else {
      return
    }
    updating = true
    error = nil
    defer { updating = false }

    let next = McpSettingsItem(
      enabled: enabled ?? current.enabled,
      token: current.token,
      draftToolsEnabled: draftToolsEnabled ?? current.draftToolsEnabled,
      circleManagementEnabled: circleManagementEnabled
        ?? current.circleManagementEnabled
    )
    do {
      status = try await desktop.updateMcpSettings(settings: next)
      settings = next
    } catch {
      self.error = MixinErrorPresenter.message(for: error)
    }
  }
}
