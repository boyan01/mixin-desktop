import Foundation
import Observation
import SwiftUI

struct ProxySettingsView: View {
  @Environment(\.mixinTheme) private var theme
  @State private var model: ProxySettingsModel
  @State private var editorRequest: ProxyEditorRequest?

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
      .task {
        await model.load()
      }
      .sheet(item: $editorRequest) { request in
        ProxyEditorView(proxy: request.proxy) { proxy in
          await model.save(proxy)
        }
      }
    }
  }

  private var proxyForm: some View {
    AppScrollView {
      VStack(spacing: 0) {
        Spacer().frame(height: 40)
        proxyGroup {
          HStack {
            Text("Proxy")
              .font(.system(size: 16))
              .foregroundStyle(theme.text)
            Spacer(minLength: 4)
            Toggle("Use Proxy", isOn: Binding(
              get: { model.enabled },
              set: { value in Task { await model.setEnabled(value) } }
            ))
            .labelsHidden()
            .scaleEffect(0.7)
            .disabled(model.proxies.isEmpty || model.isSaving)
            .accessibilityIdentifier("proxy-enabled")
          }
          .padding(.leading, 16)
          .padding(.trailing, 10)
          .padding(.vertical, 17)
        }
        proxyGroup {
          VStack(spacing: 0) {
            Button {
              editorRequest = ProxyEditorRequest(proxy: nil)
            } label: {
              HStack(spacing: 8) {
                Image(systemName: "plus")
                  .font(.system(size: 24))
                  .foregroundStyle(theme.icon)
                  .frame(width: 24, height: 24)
                Text("Add Proxy")
                  .font(.system(size: 16))
                  .foregroundStyle(theme.text)
                Spacer(minLength: 0)
              }
              .padding(.leading, 16)
              .padding(.trailing, 10)
              .padding(.vertical, 17)
              .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(model.isSaving)
            .accessibilityIdentifier("proxy-add")

            Divider().padding(.leading, 56)
            ForEach(model.proxies, id: \.id) { proxy in
              ProxyRow(
                proxy: proxy,
                selected: model.isSelected(proxy.id),
                disabled: model.isSaving,
                onSelect: { Task { await model.select(proxy.id) } },
                onDelete: { Task { await model.delete(proxy.id) } }
              )
            }
          }
        }

        if let message = model.operationError {
          proxyGroup {
            VStack(alignment: .leading, spacing: 10) {
              Label(message, systemImage: "exclamationmark.triangle")
                .font(.system(size: 14))
                .foregroundStyle(theme.destructive)
              HStack(spacing: 16) {
                Button("Dismiss") { model.dismissError() }
                Button("Reload") { Task { await model.load() } }
              }
              .font(.system(size: 14))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 17)
          }
        }
      }
    }
    .background(theme.background)
    .overlay {
      if model.isSaving {
        ProgressView()
          .controlSize(.small)
          .padding(10)
          .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
      }
    }
  }

  private func proxyGroup<Content: View>(
    @ViewBuilder content: () -> Content
  ) -> some View {
    content()
      .background(theme.settingCellBackground)
      .clipShape(RoundedRectangle(cornerRadius: 8))
      .padding(.horizontal, 10)
      .padding(.bottom, 10)
      .frame(maxWidth: 600)
  }

}

private struct ProxyRow: View {
  @Environment(\.mixinTheme) private var theme
  let proxy: ProxyItem
  let selected: Bool
  let disabled: Bool
  let onSelect: () -> Void
  let onDelete: () -> Void

  var body: some View {
    HStack(spacing: 8) {
      Button(action: onSelect) {
        Image(systemName: "checkmark")
          .font(.system(size: 20))
          .foregroundStyle(theme.icon)
          .opacity(selected ? 1 : 0)
          .frame(width: 20, height: 20)
      }
      .buttonStyle(.plain)
      .disabled(disabled || selected)
      .accessibilityLabel(selected ? "Selected proxy" : "Select proxy")

      Button(action: onSelect) {
        VStack(alignment: .leading, spacing: 3) {
          Text("\(proxy.host):\(proxy.port)")
            .font(.system(size: 16))
            .foregroundStyle(theme.text)
          Text(proxy.kind)
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
      }
      .buttonStyle(.plain)
      .disabled(disabled || selected)

      Button(role: .destructive, action: onDelete) {
        Image("Delete")
          .resizable()
          .renderingMode(.template)
          .foregroundStyle(theme.icon)
          .frame(width: 24, height: 24)
          .frame(width: 30, height: 30)
      }
      .buttonStyle(.borderless)
      .disabled(disabled)
      .help("Delete Proxy")
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 17)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("proxy-\(proxy.id)")
  }
}

private struct ProxyEditorRequest: Identifiable {
  let id = UUID()
  let proxy: ProxyItem?
}

private struct ProxyEditorView: View {
  @Environment(\.dismiss) private var dismiss
  private let onSave: (ProxyItem) async -> Bool

  @State private var host: String
  @State private var port: String
  @State private var username: String
  @State private var password: String
  @State private var validationError: String?
  @State private var saving = false

  init(
    proxy: ProxyItem?,
    onSave: @escaping (ProxyItem) async -> Bool
  ) {
    self.onSave = onSave
    _host = State(initialValue: proxy?.host ?? "")
    _port = State(initialValue: proxy.map { String($0.port) } ?? "")
    _username = State(initialValue: proxy?.username ?? "")
    _password = State(initialValue: proxy?.password ?? "")
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      Text("Add Proxy")
        .font(.system(size: 18, weight: .semibold))

      proxyLabel("Proxy Type")
      HStack {
        Text("HTTP")
          .font(.system(size: 14))
        Spacer()
        Image(systemName: "checkmark")
          .font(.system(size: 24))
      }
      .foregroundStyle(.primary)
      .padding(.horizontal, 16)
      .frame(height: 52)
      .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))

      proxyLabel("Proxy Connection")
      proxyInputGroup(
        first: TextField("Host", text: $host)
          .textContentType(.URL),
        second: TextField("Port", text: $port)
          .onChange(of: port) {
            port = String(port.filter(\.isNumber).prefix(5))
          }
      )

      proxyLabel("Proxy Authentication")
      proxyInputGroup(
        first: TextField("Username", text: $username),
        second: SecureField("Password", text: $password)
      )

      if let validationError {
        Text(validationError)
          .font(.system(size: 14))
          .foregroundStyle(.red)
      }

      HStack {
        Spacer()
        Button("Cancel", role: .cancel) {
          dismiss()
        }
        .keyboardShortcut(.cancelAction)
        Button("Add") {
          save()
        }
        .keyboardShortcut(.defaultAction)
        .disabled(saving)
      }
    }
    .padding(24)
    .frame(width: 420)
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
    validationError = nil
    saving = true
    let item = ProxyItem(
      id: UUID().uuidString.lowercased(),
      kind: "http",
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

  private func proxyLabel(_ title: String) -> some View {
    Text(title)
      .font(.system(size: 14))
      .foregroundStyle(.secondary)
      .padding(.bottom, -12)
  }

  private func proxyInputGroup<First: View, Second: View>(
    first: First,
    second: Second
  ) -> some View {
    VStack(spacing: 0) {
      first
        .textFieldStyle(.plain)
        .font(.system(size: 14))
        .padding(.horizontal, 16)
        .frame(height: 52)
      Divider()
      second
        .textFieldStyle(.plain)
        .font(.system(size: 14))
        .padding(.horizontal, 16)
        .frame(height: 52)
    }
    .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
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
  private var settings: ProxySettingsItem?
  private(set) var state: State = .loading
  private(set) var isSaving = false
  private(set) var operationError: String?

  var enabled: Bool {
    settings?.enabled ?? false
  }

  var proxies: [ProxyItem] {
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
      ProxySettingsItem(
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
      ProxySettingsItem(
        enabled: settings.enabled,
        selectedProxyId: id,
        proxies: settings.proxies
      ))
  }

  func save(_ proxy: ProxyItem) async -> Bool {
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
      ProxySettingsItem(
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
      ProxySettingsItem(
        enabled: !deletedSelected && !proxies.isEmpty && settings.enabled,
        selectedProxyId: deletedSelected ? nil : settings.selectedProxyId,
        proxies: proxies
      ))
  }

  func dismissError() {
    operationError = nil
  }

  @discardableResult
  private func persist(_ next: ProxySettingsItem) async -> Bool {
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
