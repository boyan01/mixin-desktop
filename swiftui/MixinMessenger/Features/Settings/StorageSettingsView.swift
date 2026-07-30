import Observation
import SwiftUI

struct DataStorageSettingsView: View {
  @Environment(\.mixinTheme) private var theme
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
      AppScrollView {
        VStack(alignment: .leading, spacing: 0) {
          Spacer().frame(height: 40)
          storageGroup {
            VStack(spacing: 0) {
              downloadCell("Photos", value: Binding(
                get: { model.photoAutoDownload }, set: { model.setPhotoAutoDownload($0) }
              ))
              downloadCell("Videos", value: Binding(
                get: { model.videoAutoDownload }, set: { model.setVideoAutoDownload($0) }
              ))
              downloadCell("Files", value: Binding(
                get: { model.fileAutoDownload }, set: { model.setFileAutoDownload($0) }
              ))
            }
          }
          Text("Choose which media types are downloaded automatically.")
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)
            .padding(.leading, 20)
            .padding(.top, 10)
            .padding(.bottom, 14)
            .frame(maxWidth: 600, alignment: .leading)
          storageGroup {
            NavigationLink {
              StorageUsageListView(account: account)
            } label: {
              HStack(spacing: 4) {
                Text("Storage Usage")
                  .font(.system(size: 16))
                  .foregroundStyle(theme.text)
                Spacer(minLength: 0)
                Image("SettingsArrow")
                  .resizable()
                  .renderingMode(.template)
                  .foregroundStyle(theme.secondaryText)
                  .frame(width: 30, height: 30)
              }
              .padding(.leading, 16)
              .padding(.trailing, 10)
              .padding(.vertical, 17)
              .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("settings-storage-usage")
          }
          if let error = model.error {
            storageGroup {
              HStack(spacing: 12) {
                Text(error)
                  .font(.system(size: 14))
                  .foregroundStyle(theme.destructive)
                Spacer()
                Button("Retry") { Task { await model.load() } }
                  .font(.system(size: 14))
              }
              .padding(.horizontal, 16)
              .padding(.vertical, 17)
            }
          }
        }
      }
      .background(theme.background)
      .navigationTitle("Data and Storage")
      .task {
        await model.load()
      }
    }
  }

  private func storageGroup<Content: View>(
    @ViewBuilder content: () -> Content
  ) -> some View {
    content()
      .background(theme.settingCellBackground)
      .clipShape(RoundedRectangle(cornerRadius: 8))
      .padding(.horizontal, 10)
      .padding(.bottom, 10)
      .frame(maxWidth: 600)
  }

  private func downloadCell(_ title: String, value: Binding<Bool>) -> some View {
    HStack {
      Text(title)
        .font(.system(size: 16))
        .foregroundStyle(theme.text)
      Spacer(minLength: 4)
      Toggle(title, isOn: value)
        .labelsHidden()
        .scaleEffect(0.7)
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 17)
  }
}

private struct StorageUsageListView: View {
  @Environment(\.mixinTheme) private var theme
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
      case .failed:
        Color.clear
      case .loaded(let entries):
        if entries.isEmpty {
          Color.clear
        } else {
          AppScrollView {
            LazyVStack(spacing: 10) {
              ForEach(entries, id: \.conversation.conversationId) { entry in
                NavigationLink {
                  StorageUsageDetailView(
                    account: account,
                    conversationId: entry.conversation.conversationId,
                    name: entry.conversation.name
                  )
                } label: {
                  StorageConversationRow(entry: entry)
                }
                .buttonStyle(.plain)
              }
            }
            .padding(.vertical, 40)
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
  @Environment(\.mixinTheme) private var theme
  let entry: ConversationStorageUsage

  var body: some View {
    HStack(spacing: 8) {
      ConversationAvatar(conversation: entry.conversation, size: 50)

      VStack(alignment: .leading, spacing: 3) {
        Text(entry.conversation.name)
          .font(.system(size: 16))
          .foregroundStyle(theme.text)
          .lineLimit(1)
        Text(
          ByteCountFormatter.string(
            fromByteCount: entry.sizeBytes,
            countStyle: .file
          )
        )
        .font(.system(size: 14))
        .foregroundStyle(theme.secondaryText)
      }
      Spacer(minLength: 4)
      Image("SettingsArrow")
        .resizable()
        .renderingMode(.template)
        .foregroundStyle(theme.secondaryText)
        .frame(width: 30, height: 30)
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 17)
    .background(theme.settingCellBackground)
    .clipShape(RoundedRectangle(cornerRadius: 8))
    .frame(maxWidth: 600)
  }
}

private struct StorageUsageDetailView: View {
  @Environment(\.mixinTheme) private var theme
  @State private var model: StorageUsageDetailModel

  init(account: SwiftAccountHandle, conversationId: String, name: String) {
    _model = State(
      initialValue: StorageUsageDetailModel(
        account: account,
        conversationId: conversationId,
        name: name
      ))
  }

  var body: some View {
    AppScrollView {
      VStack(alignment: .leading, spacing: 0) {
        Spacer().frame(height: 40)
        categoryGroup {
          VStack(spacing: 0) {
            ForEach(model.categories, id: \.category) { item in
              Button {
                model.toggle(item.category)
              } label: {
                HStack(spacing: 0) {
                  let selected = model.selectedCategories.contains(item.category)
                  ZStack {
                    Circle().fill(selected ? theme.accent : theme.secondaryText)
                    if selected {
                      Image(systemName: "checkmark")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(.white)
                    }
                  }
                  .frame(width: 16, height: 16)
                  Spacer().frame(width: 30)
                  Text(model.label(for: item.category))
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                  Spacer(minLength: 4)
                  Text(ByteCountFormatter.string(
                    fromByteCount: item.sizeBytes,
                    countStyle: .file
                  ))
                  .font(.system(size: 14))
                  .foregroundStyle(theme.secondaryText)
                }
                .padding(.leading, 16)
                .padding(.trailing, 10)
                .padding(.vertical, 17)
                .contentShape(Rectangle())
              }
              .buttonStyle(.plain)
            }
          }
        }
        if let error = model.error {
          categoryGroup {
            HStack(spacing: 12) {
              Text(error)
                .font(.system(size: 14))
                .foregroundStyle(theme.destructive)
              Spacer()
              Button("Retry") { Task { await model.load() } }
                .font(.system(size: 14))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 17)
          }
        }
      }
    }
    .background(theme.background)
    .navigationTitle(model.name)
    .toolbar {
      ToolbarItem {
        Button {
          Task { await model.clearSelected() }
        } label: {
          Text("Clear")
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(theme.accent)
        }
        .disabled(model.selectedCategories.isEmpty || model.isClearing)
        .accessibilityIdentifier("storage-clear")
      }
    }
    .task {
      await model.start()
    }
    .onDisappear {
      model.stop()
    }
  }

  private func categoryGroup<Content: View>(
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
    case loaded([ConversationStorageUsage])
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
  private(set) var categories: [StorageCategoryUsage] =
    StorageUsageDetailModel.emptyCategories
  private(set) var selectedCategories: Set<String> = []
  private(set) var error: String?
  private(set) var isClearing = false
  private var monitor: StorageDirectoryMonitor?

  private static let emptyCategories = [
    StorageCategoryUsage(category: "photos", sizeBytes: 0),
    StorageCategoryUsage(category: "videos", sizeBytes: 0),
    StorageCategoryUsage(category: "audio", sizeBytes: 0),
    StorageCategoryUsage(category: "files", sizeBytes: 0),
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
