import AppKit
import Observation
import SwiftUI

struct McpSettingsView: View {
  @State private var model: McpSettingsModel
  @State private var copiedValue: CopiedValue?

  init(desktop: SwiftDesktopHandle) {
    _model = State(initialValue: McpSettingsModel(desktop: desktop))
  }

  var body: some View {
    Group {
      if let settings = model.settings {
        Form {
          statusSection
          serverSection(settings)
          if settings.enabled {
            connectionSection(settings)
            permissionsSection(settings)
          }
          Section {
            Text(
              "Local clients must use the bearer token. MCP can read account data, "
                + "while write operations remain limited to the permissions enabled here."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
          }
          if let error = model.error {
            Section {
              Label(error, systemImage: "exclamationmark.triangle")
                .foregroundStyle(.red)
              Button("Reload") {
                Task {
                  await model.load()
                }
              }
            }
          }
        }
        .formStyle(.grouped)
        .settingsFormLayout()
      } else if let error = model.error {
        ContentUnavailableView {
          Label("MCP Settings Unavailable", systemImage: "exclamationmark.triangle")
        } description: {
          Text(error)
        } actions: {
          Button("Retry") {
            Task {
              await model.load()
            }
          }
        }
      } else {
        ProgressView("Loading MCP settings…")
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

  private var statusSection: some View {
    Section("Status") {
      HStack {
        Circle()
          .fill(model.status?.running == true ? Color.green : Color.secondary)
          .frame(width: 9, height: 9)
        Text(model.statusText)
        Spacer()
        if model.updating {
          ProgressView()
            .controlSize(.small)
        }
      }
    }
  }

  private func serverSection(_ settings: SwiftMcpSettings) -> some View {
    Section {
      Toggle(
        "Server",
        isOn: Binding(
          get: { settings.enabled },
          set: { enabled in
            Task {
              await model.update(enabled: enabled)
            }
          }
        )
      )
      .disabled(model.updating)
    } footer: {
      Text("The server only listens on localhost.")
    }
  }

  private func connectionSection(_ settings: SwiftMcpSettings) -> some View {
    Section("Connection") {
      LabeledContent("Endpoint") {
        copyableValue(
          model.status?.endpoint ?? "Not running",
          value: model.status?.endpoint,
          copied: .endpoint
        )
      }
      LabeledContent("Access Token") {
        copyableValue(
          masked(settings.token),
          value: settings.token,
          copied: .token
        )
      }
    }
  }

  private func permissionsSection(_ settings: SwiftMcpSettings) -> some View {
    Section("Write Permissions") {
      Toggle(
        "Draft Editing",
        isOn: Binding(
          get: { settings.draftToolsEnabled },
          set: { enabled in
            Task {
              await model.update(draftToolsEnabled: enabled)
            }
          }
        ))
      Toggle(
        "Circle Management",
        isOn: Binding(
          get: { settings.circleManagementEnabled },
          set: { enabled in
            Task {
              await model.update(circleManagementEnabled: enabled)
            }
          }
        ))
    }
    .disabled(model.updating)
  }

  private func copyableValue(
    _ label: String,
    value: String?,
    copied: CopiedValue
  ) -> some View {
    HStack(spacing: 8) {
      Text(label)
        .foregroundStyle(.secondary)
        .textSelection(.enabled)
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
      }
      .buttonStyle(.borderless)
      .disabled(value == nil)
      .help("Copy \(copied.rawValue)")
    }
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

  private(set) var settings: SwiftMcpSettings?
  private(set) var status: SwiftMcpServerStatus?
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

    let next = SwiftMcpSettings(
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
