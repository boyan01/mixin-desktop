import Foundation
import Observation
import SwiftUI

struct ProxySettingsView: View {
  @State private var model: ProxySettingsModel
  @State private var editorRequest: ProxyEditorRequest?
  @State private var pendingDeletion: SwiftProxyItem?

  init(desktop: SwiftDesktopHandle) {
    _model = State(initialValue: ProxySettingsModel(desktop: desktop))
  }

  var body: some View {
    NavigationStack {
      Group {
        switch model.state {
        case .loading:
          ProgressView("Loading proxy settings…")
        case .failed(let message):
          ContentUnavailableView {
            Label("Proxy Settings Unavailable", systemImage: "network.slash")
          } description: {
            Text(message)
          } actions: {
            Button("Retry") {
              Task {
                await model.load()
              }
            }
          }
        case .loaded:
          proxyForm
        }
      }
      .navigationTitle("Proxy")
      .toolbar {
        ToolbarItem {
          Button {
            editorRequest = ProxyEditorRequest(proxy: nil)
          } label: {
            Label("Add Proxy", systemImage: "plus")
          }
          .disabled(model.isSaving)
          .accessibilityIdentifier("proxy-add")
        }
      }
      .task {
        await model.load()
      }
      .sheet(item: $editorRequest) { request in
        ProxyEditorView(proxy: request.proxy) { proxy in
          await model.save(proxy)
        }
      }
      .confirmationDialog(
        "Delete this proxy?",
        isPresented: deletionPresented,
        presenting: pendingDeletion
      ) { proxy in
        Button("Delete \(proxy.host):\(proxy.port)", role: .destructive) {
          Task {
            await model.delete(proxy.id)
          }
        }
        Button("Cancel", role: .cancel) {}
      } message: { proxy in
        Text(
          model.isSelected(proxy.id)
            ? "The active proxy will be disabled."
            : "This proxy configuration will be removed.")
      }
    }
  }

  private var proxyForm: some View {
    Form {
      Section {
        Toggle(
          "Use Proxy",
          isOn: Binding(
            get: { model.enabled },
            set: { value in
              Task {
                await model.setEnabled(value)
              }
            }
          )
        )
        .disabled(model.proxies.isEmpty || model.isSaving)
        .accessibilityIdentifier("proxy-enabled")
      } footer: {
        if model.proxies.isEmpty {
          Text("Add a proxy before enabling network proxying.")
        } else {
          Text("Mixin network requests use the selected proxy while this is enabled.")
        }
      }

      Section("Proxy Servers") {
        if model.proxies.isEmpty {
          Text("No proxy servers configured.")
            .foregroundStyle(.secondary)
        } else {
          ForEach(model.proxies, id: \.id) { proxy in
            ProxyRow(
              proxy: proxy,
              selected: model.isSelected(proxy.id),
              disabled: model.isSaving,
              onSelect: {
                Task {
                  await model.select(proxy.id)
                }
              },
              onEdit: {
                editorRequest = ProxyEditorRequest(proxy: proxy)
              },
              onDelete: {
                pendingDeletion = proxy
              }
            )
          }
        }
      }

      if let message = model.operationError {
        Section {
          Label(message, systemImage: "exclamationmark.triangle")
            .foregroundStyle(.red)
          HStack {
            Button("Dismiss") {
              model.dismissError()
            }
            Button("Reload") {
              Task {
                await model.load()
              }
            }
          }
        }
      }
    }
    .formStyle(.grouped)
    .settingsFormLayout()
    .overlay {
      if model.isSaving {
        ProgressView()
          .controlSize(.small)
          .padding(10)
          .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
      }
    }
  }

  private var deletionPresented: Binding<Bool> {
    Binding(
      get: { pendingDeletion != nil },
      set: { presented in
        if !presented {
          pendingDeletion = nil
        }
      }
    )
  }
}

private struct ProxyRow: View {
  let proxy: SwiftProxyItem
  let selected: Bool
  let disabled: Bool
  let onSelect: () -> Void
  let onEdit: () -> Void
  let onDelete: () -> Void

  var body: some View {
    HStack(spacing: 12) {
      Button(action: onSelect) {
        Image(systemName: selected ? "checkmark.circle.fill" : "circle")
          .foregroundStyle(selected ? Color.accentColor : Color.secondary)
          .frame(width: 20)
      }
      .buttonStyle(.plain)
      .disabled(disabled || selected)
      .accessibilityLabel(selected ? "Selected proxy" : "Select proxy")

      Button(action: onSelect) {
        VStack(alignment: .leading, spacing: 3) {
          Text("\(proxy.host):\(proxy.port)")
            .foregroundStyle(.primary)
          Text(proxy.kind.uppercased())
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
      }
      .buttonStyle(.plain)
      .disabled(disabled || selected)

      Button(action: onEdit) {
        Image(systemName: "pencil")
      }
      .buttonStyle(.borderless)
      .disabled(disabled)
      .help("Edit Proxy")

      Button(role: .destructive, action: onDelete) {
        Image(systemName: "trash")
      }
      .buttonStyle(.borderless)
      .disabled(disabled)
      .help("Delete Proxy")
    }
    .padding(.vertical, 4)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("proxy-\(proxy.id)")
  }
}

private struct ProxyEditorRequest: Identifiable {
  let id = UUID()
  let proxy: SwiftProxyItem?
}

private struct ProxyEditorView: View {
  @Environment(\.dismiss) private var dismiss
  private let proxyId: String
  private let editing: Bool
  private let onSave: (SwiftProxyItem) async -> Bool

  @State private var kind: String
  @State private var host: String
  @State private var port: String
  @State private var username: String
  @State private var password: String
  @State private var validationError: String?
  @State private var saving = false

  init(
    proxy: SwiftProxyItem?,
    onSave: @escaping (SwiftProxyItem) async -> Bool
  ) {
    proxyId = proxy?.id ?? UUID().uuidString.lowercased()
    editing = proxy != nil
    self.onSave = onSave
    _kind = State(initialValue: proxy?.kind.lowercased() ?? "http")
    _host = State(initialValue: proxy?.host ?? "")
    _port = State(initialValue: proxy.map { String($0.port) } ?? "")
    _username = State(initialValue: proxy?.username ?? "")
    _password = State(initialValue: proxy?.password ?? "")
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      Text(editing ? "Edit Proxy" : "Add Proxy")
        .font(.title2.bold())

      Form {
        Section("Proxy Type") {
          Picker("Type", selection: $kind) {
            Text("HTTP").tag("http")
            Text("SOCKS5").tag("socks5")
          }
          .pickerStyle(.segmented)
        }

        Section("Connection") {
          TextField("Host", text: $host)
            .textContentType(.URL)
          TextField("Port", text: $port)
            .onChange(of: port) {
              port = String(port.filter(\.isNumber).prefix(5))
            }
        }

        Section("Authentication (Optional)") {
          TextField("Username", text: $username)
          SecureField("Password", text: $password)
        }

        if let validationError {
          Text(validationError)
            .foregroundStyle(.red)
        }
      }
      .formStyle(.grouped)
      .onChange(of: host) {
        host = String(host.prefix(200))
      }
      .onChange(of: username) {
        username = String(username.prefix(200))
      }
      .onChange(of: password) {
        password = String(password.prefix(200))
      }

      HStack {
        Spacer()
        Button("Cancel", role: .cancel) {
          dismiss()
        }
        .keyboardShortcut(.cancelAction)
        Button(editing ? "Save" : "Add") {
          save()
        }
        .keyboardShortcut(.defaultAction)
        .disabled(saving)
      }
    }
    .padding(24)
    .frame(width: 440, height: 500)
    .interactiveDismissDisabled(saving)
  }

  private func save() {
    let trimmedHost = host.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmedHost.isEmpty else {
      validationError = "Host is required."
      return
    }
    guard let parsedPort = UInt16(port), parsedPort > 0 else {
      validationError = "Port must be between 1 and 65535."
      return
    }
    guard kind == "http" || kind == "socks5" else {
      validationError = "Proxy type is invalid."
      return
    }

    validationError = nil
    saving = true
    let item = SwiftProxyItem(
      id: proxyId,
      kind: kind,
      host: trimmedHost,
      port: parsedPort,
      username: nilIfEmpty(username),
      password: nilIfEmpty(password)
    )
    Task {
      if await onSave(item) {
        dismiss()
      } else {
        validationError = "The proxy could not be saved. Check the values and try again."
        saving = false
      }
    }
  }

  private func nilIfEmpty(_ value: String) -> String? {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmed.isEmpty ? nil : trimmed
  }
}

@MainActor
@Observable
private final class ProxySettingsModel {
  enum State {
    case loading
    case loaded
    case failed(String)
  }

  private let desktop: SwiftDesktopHandle
  private var settings: SwiftProxySettings?
  private(set) var state: State = .loading
  private(set) var isSaving = false
  private(set) var operationError: String?

  var enabled: Bool {
    settings?.enabled ?? false
  }

  var proxies: [SwiftProxyItem] {
    settings?.proxies ?? []
  }

  init(desktop: SwiftDesktopHandle) {
    self.desktop = desktop
  }

  func load() async {
    guard !isSaving else {
      return
    }
    state = .loading
    do {
      settings = try await desktop.proxySettings()
      operationError = nil
      state = .loaded
    } catch {
      state = .failed(MixinErrorPresenter.message(for: error))
    }
  }

  func isSelected(_ id: String) -> Bool {
    guard let settings else {
      return false
    }
    return (settings.selectedProxyId ?? settings.proxies.first?.id) == id
  }

  func setEnabled(_ enabled: Bool) async {
    guard let settings, !isSaving else {
      return
    }
    let selected = settings.selectedProxyId ?? settings.proxies.first?.id
    await persist(
      SwiftProxySettings(
        enabled: enabled && selected != nil,
        selectedProxyId: selected,
        proxies: settings.proxies
      ))
  }

  func select(_ id: String) async {
    guard let settings,
      settings.proxies.contains(where: { $0.id == id }),
      !isSaving
    else {
      return
    }
    await persist(
      SwiftProxySettings(
        enabled: settings.enabled,
        selectedProxyId: id,
        proxies: settings.proxies
      ))
  }

  func save(_ proxy: SwiftProxyItem) async -> Bool {
    guard let settings, !isSaving else {
      return false
    }
    var proxies = settings.proxies
    if let index = proxies.firstIndex(where: { $0.id == proxy.id }) {
      proxies[index] = proxy
    } else {
      proxies.append(proxy)
    }
    return await persist(
      SwiftProxySettings(
        enabled: settings.enabled,
        selectedProxyId: settings.selectedProxyId,
        proxies: proxies
      ))
  }

  func delete(_ id: String) async {
    guard let settings, !isSaving else {
      return
    }
    let proxies = settings.proxies.filter { $0.id != id }
    let selected = settings.selectedProxyId ?? settings.proxies.first?.id
    let deletedSelected = selected == id
    await persist(
      SwiftProxySettings(
        enabled: !deletedSelected && !proxies.isEmpty && settings.enabled,
        selectedProxyId: deletedSelected ? nil : settings.selectedProxyId,
        proxies: proxies
      ))
  }

  func dismissError() {
    operationError = nil
  }

  @discardableResult
  private func persist(_ next: SwiftProxySettings) async -> Bool {
    isSaving = true
    operationError = nil
    defer {
      isSaving = false
    }
    do {
      try await desktop.setProxySettings(settings: next)
      settings = next
      state = .loaded
      return true
    } catch {
      operationError = MixinErrorPresenter.message(for: error)
      return false
    }
  }
}
