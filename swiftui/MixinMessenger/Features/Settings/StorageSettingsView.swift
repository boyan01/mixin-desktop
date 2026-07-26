import Observation
import SwiftUI

struct DataStorageSettingsView: View {
  let desktop: SwiftDesktopHandle
  let account: SwiftAccountHandle
  @State private var model: DataStorageSettingsModel

  init(desktop: SwiftDesktopHandle, account: SwiftAccountHandle) {
    self.desktop = desktop
    self.account = account
    _model = State(initialValue: DataStorageSettingsModel(desktop: desktop))
  }

  var body: some View {
    NavigationStack {
      Form {
        Section {
          Toggle(
            "Photos",
            isOn: Binding(
              get: { model.photoAutoDownload },
              set: { model.setPhotoAutoDownload($0) }
            ))
          Toggle(
            "Videos",
            isOn: Binding(
              get: { model.videoAutoDownload },
              set: { model.setVideoAutoDownload($0) }
            ))
          Toggle(
            "Files",
            isOn: Binding(
              get: { model.fileAutoDownload },
              set: { model.setFileAutoDownload($0) }
            ))
        } header: {
          Text("Automatic Media Download")
        } footer: {
          Text("Choose which media types are downloaded automatically.")
        }

        Section {
          NavigationLink {
            StorageUsageListView(account: account)
          } label: {
            Text("Storage Usage")
          }
          .accessibilityIdentifier("settings-storage-usage")
        }

        if let error = model.error {
          Section {
            Text(error)
              .foregroundStyle(.red)
            Button("Retry") {
              Task {
                await model.load()
              }
            }
          }
        }
      }
      .formStyle(.grouped)
      .settingsFormLayout()
      .navigationTitle("Data and Storage")
      .task {
        await model.load()
      }
    }
  }
}

private struct StorageUsageListView: View {
  let account: SwiftAccountHandle
  @State private var model: StorageUsageListModel

  init(account: SwiftAccountHandle) {
    self.account = account
    _model = State(initialValue: StorageUsageListModel(account: account))
  }

  var body: some View {
    Group {
      switch model.state {
      case .loading:
        ProgressView("Calculating storage usage…")
      case .failed(let message):
        ContentUnavailableView {
          Label("Storage Usage Unavailable", systemImage: "exclamationmark.triangle")
        } description: {
          Text(message)
        } actions: {
          Button("Retry") {
            Task {
              await model.load()
            }
          }
        }
      case .loaded(let entries):
        if entries.isEmpty {
          ContentUnavailableView(
            "No Cached Media",
            systemImage: "internaldrive",
            description: Text("Downloaded media will appear here.")
          )
        } else {
          List(entries, id: \.conversation.conversationId) { entry in
            NavigationLink {
              StorageUsageDetailView(
                account: account,
                conversationId: entry.conversation.conversationId,
                name: entry.conversation.name
              )
            } label: {
              StorageConversationRow(entry: entry)
            }
          }
        }
      }
    }
    .navigationTitle("Storage Usage")
    .task {
      await model.start()
    }
    .onDisappear {
      model.stop()
    }
  }
}

private struct StorageConversationRow: View {
  let entry: SwiftConversationStorageUsage

  var body: some View {
    HStack(spacing: 12) {
      MixinRemoteImage(url: URL(string: entry.conversation.iconUrl)) { image in
        image.resizable().scaledToFill()
      } placeholder: {
        Image(systemName: "person.2.circle.fill")
          .resizable()
          .foregroundStyle(.secondary)
      }
      .frame(width: 44, height: 44)
      .clipShape(Circle())

      VStack(alignment: .leading, spacing: 3) {
        Text(entry.conversation.name)
          .lineLimit(1)
        Text(
          ByteCountFormatter.string(
            fromByteCount: entry.sizeBytes,
            countStyle: .file
          )
        )
        .font(.caption)
        .foregroundStyle(.secondary)
      }
    }
    .padding(.vertical, 3)
  }
}

private struct StorageUsageDetailView: View {
  @State private var model: StorageUsageDetailModel
  @State private var confirmClear = false

  init(account: SwiftAccountHandle, conversationId: String, name: String) {
    _model = State(
      initialValue: StorageUsageDetailModel(
        account: account,
        conversationId: conversationId,
        name: name
      ))
  }

  var body: some View {
    Form {
      Section {
        ForEach(model.categories, id: \.category) { item in
          Button {
            model.toggle(item.category)
          } label: {
            HStack {
              Image(
                systemName: model.selectedCategories.contains(item.category)
                  ? "checkmark.circle.fill"
                  : "circle"
              )
              .foregroundStyle(
                model.selectedCategories.contains(item.category)
                  ? Color.accentColor
                  : Color.secondary)
              Text(model.label(for: item.category))
              Spacer()
              Text(
                ByteCountFormatter.string(
                  fromByteCount: item.sizeBytes,
                  countStyle: .file
                )
              )
              .foregroundStyle(.secondary)
            }
            .contentShape(Rectangle())
          }
          .buttonStyle(.plain)
        }
      }

      if let error = model.error {
        Section {
          Text(error)
            .foregroundStyle(.red)
          Button("Retry") {
            Task {
              await model.load()
            }
          }
        }
      }
    }
    .formStyle(.grouped)
    .navigationTitle(model.name)
    .toolbar {
      ToolbarItem {
        Button("Clear", role: .destructive) {
          confirmClear = true
        }
        .disabled(model.selectedCategories.isEmpty || model.isClearing)
        .accessibilityIdentifier("storage-clear")
      }
    }
    .confirmationDialog(
      "Clear selected cached media?",
      isPresented: $confirmClear
    ) {
      Button("Clear", role: .destructive) {
        Task {
          await model.clearSelected()
        }
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("The media can be downloaded again from the conversation.")
    }
    .task {
      await model.start()
    }
    .onDisappear {
      model.stop()
    }
  }
}

@MainActor
@Observable
private final class DataStorageSettingsModel {
  private let desktop: SwiftDesktopHandle
  private(set) var photoAutoDownload = true
  private(set) var videoAutoDownload = true
  private(set) var fileAutoDownload = true
  private(set) var error: String?
  private var loaded = false
  private var updateVersions: [String: Int] = [:]

  init(desktop: SwiftDesktopHandle) {
    self.desktop = desktop
  }

  func load() async {
    guard !loaded else {
      return
    }
    do {
      async let photo = desktop.photoAutoDownload()
      async let video = desktop.videoAutoDownload()
      async let file = desktop.fileAutoDownload()
      (photoAutoDownload, videoAutoDownload, fileAutoDownload) =
        try await (photo, video, file)
      error = nil
      loaded = true
    } catch {
      self.error = MixinErrorPresenter.message(for: error)
    }
  }

  func setPhotoAutoDownload(_ value: Bool) {
    update("photo", keyPath: \.photoAutoDownload, value: value) { desktop, value in
      try await desktop.setPhotoAutoDownload(value: value)
    }
  }

  func setVideoAutoDownload(_ value: Bool) {
    update("video", keyPath: \.videoAutoDownload, value: value) { desktop, value in
      try await desktop.setVideoAutoDownload(value: value)
    }
  }

  func setFileAutoDownload(_ value: Bool) {
    update("file", keyPath: \.fileAutoDownload, value: value) { desktop, value in
      try await desktop.setFileAutoDownload(value: value)
    }
  }

  private func update(
    _ key: String,
    keyPath: ReferenceWritableKeyPath<DataStorageSettingsModel, Bool>,
    value: Bool,
    persist: @escaping (SwiftDesktopHandle, Bool) async throws -> Void
  ) {
    let previous = self[keyPath: keyPath]
    let version = (updateVersions[key] ?? 0) + 1
    updateVersions[key] = version
    self[keyPath: keyPath] = value
    error = nil
    Task {
      do {
        try await persist(desktop, value)
      } catch {
        guard updateVersions[key] == version else {
          return
        }
        self[keyPath: keyPath] = previous
        self.error = MixinErrorPresenter.message(for: error)
      }
    }
  }
}

@MainActor
@Observable
private final class StorageUsageListModel {
  enum State {
    case loading
    case loaded([SwiftConversationStorageUsage])
    case failed(String)
  }

  private let account: SwiftAccountHandle
  private(set) var state: State = .loading
  private var monitor: StorageDirectoryMonitor?

  init(account: SwiftAccountHandle) {
    self.account = account
  }

  func start() async {
    if monitor == nil, let directory = try? account.mediaDirectory() {
      let monitor = StorageDirectoryMonitor(directory: directory) { [weak self] in
        Task {
          await self?.load(showLoading: false)
        }
      }
      self.monitor = monitor
      monitor.start()
    }
    await load()
  }

  func load(showLoading: Bool = true) async {
    if showLoading {
      state = .loading
    }
    do {
      state = .loaded(try await account.storageUsage())
    } catch {
      state = .failed(MixinErrorPresenter.message(for: error))
    }
  }

  func stop() {
    monitor?.stop()
    monitor = nil
  }
}

@MainActor
@Observable
private final class StorageUsageDetailModel {
  private let account: SwiftAccountHandle
  private let conversationId: String
  let name: String
  private(set) var categories: [SwiftStorageCategoryUsage] =
    StorageUsageDetailModel.emptyCategories
  private(set) var selectedCategories: Set<String> = []
  private(set) var error: String?
  private(set) var isClearing = false
  private var monitor: StorageDirectoryMonitor?

  private static let emptyCategories = [
    SwiftStorageCategoryUsage(category: "photos", sizeBytes: 0),
    SwiftStorageCategoryUsage(category: "videos", sizeBytes: 0),
    SwiftStorageCategoryUsage(category: "audio", sizeBytes: 0),
    SwiftStorageCategoryUsage(category: "files", sizeBytes: 0),
  ]

  init(account: SwiftAccountHandle, conversationId: String, name: String) {
    self.account = account
    self.conversationId = conversationId
    self.name = name
  }

  func start() async {
    if monitor == nil, let directory = try? account.mediaDirectory() {
      let monitor = StorageDirectoryMonitor(directory: directory) { [weak self] in
        Task {
          await self?.load()
        }
      }
      self.monitor = monitor
      monitor.start()
    }
    await load()
  }

  func load() async {
    do {
      categories = try await account.conversationStorageUsage(
        conversationId: conversationId
      )
      selectedCategories.formIntersection(categories.map(\.category))
      error = nil
    } catch {
      self.error = MixinErrorPresenter.message(for: error)
    }
  }

  func toggle(_ category: String) {
    if !selectedCategories.insert(category).inserted {
      selectedCategories.remove(category)
    }
  }

  func clearSelected() async {
    guard !selectedCategories.isEmpty else {
      return
    }
    isClearing = true
    error = nil
    do {
      try await account.clearConversationStorage(
        conversationId: conversationId,
        categories: Array(selectedCategories)
      )
      selectedCategories.removeAll()
      await load()
    } catch {
      self.error = MixinErrorPresenter.message(for: error)
    }
    isClearing = false
  }

  func label(for category: String) -> String {
    switch category {
    case "photos": "Photos"
    case "videos": "Videos"
    case "audio": "Audio"
    default: "Files"
    }
  }

  func stop() {
    monitor?.stop()
    monitor = nil
  }
}
