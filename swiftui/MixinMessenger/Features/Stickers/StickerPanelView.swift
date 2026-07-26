import AppKit
import Foundation
import Observation
import SwiftUI
import UniformTypeIdentifiers

struct StickerPanelView: View {
    @State private var model: StickerPanelModel
    @State private var selectedTab: StickerPanelTab = .emoji
    @State private var storePresented = false
    @State private var detailStickerID: String?
    @Binding var draft: String
    let gifAPIKey: String?
    let onSendSticker: (String) async -> Bool
    let onSendGIF: (GiphyItem) async -> Bool
    let onSent: () -> Void

    init(
        account: SwiftAccountHandle,
        desktop: SwiftDesktopHandle,
        accountID: String,
        draft: Binding<String>,
        gifAPIKey: String?,
        onSendSticker: @escaping (String) async -> Bool,
        onSendGIF: @escaping (GiphyItem) async -> Bool,
        onSent: @escaping () -> Void
    ) {
        _model = State(initialValue: StickerPanelModel(
            account: account,
            desktop: desktop,
            accountID: accountID
        ))
        _draft = draft
        self.gifAPIKey = gifAPIKey
        self.onSendSticker = onSendSticker
        self.onSendGIF = onSendGIF
        self.onSent = onSent
    }

    var body: some View {
        VStack(spacing: 0) {
            panelContent
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            Divider()
            tabBar
        }
        .frame(width: 464, height: 407)
        .task {
            await model.load()
        }
        .sheet(isPresented: $storePresented) {
            StickerStoreView(model: model)
        }
        .sheet(isPresented: Binding(
            get: { detailStickerID != nil },
            set: { if !$0 { detailStickerID = nil } }
        )) {
            if let detailStickerID {
                StickerDetailView(
                    model: model,
                    stickerID: detailStickerID
                )
            }
        }
    }

    @ViewBuilder
    private var panelContent: some View {
        if model.loading, model.library == nil {
            ProgressView("Loading stickers…")
        } else if let error = model.error, model.library == nil {
            ContentUnavailableView {
                Label("Unable to load stickers", systemImage: "exclamationmark.triangle")
            } description: {
                Text(error)
            } actions: {
                Button("Retry") {
                    Task {
                        await model.load(force: true)
                    }
                }
            }
        } else {
            switch selectedTab {
            case .emoji:
                EmojiPickerView(
                    recent: model.recentEmojis,
                    onSelect: useEmoji
                )
            case .recent:
                StickerGrid(
                    stickers: model.library?.recent ?? [],
                    emptyTitle: "No Recent Stickers",
                    onSelect: sendSticker,
                    onDetail: { detailStickerID = $0 }
                )
            case .favorite:
                StickerGrid(
                    stickers: model.library?.personal ?? [],
                    emptyTitle: "No Favorite Stickers",
                    includesAddButton: true,
                    onAdd: {
                        Task {
                            await model.pickAndAddSticker()
                        }
                    },
                    onSelect: sendSticker,
                    onDelete: { stickerID in
                        Task {
                            await model.removeSticker(stickerID)
                        }
                    },
                    onDetail: { detailStickerID = $0 }
                )
            case .gif:
                if let gifAPIKey {
                    GiphyPickerView(
                        apiKey: gifAPIKey,
                        onSelect: { gif in
                            if await onSendGIF(gif) {
                                onSent()
                            }
                        }
                    )
                }
            case let .album(albumID):
                StickerGrid(
                    stickers: model.library?.albums
                        .first(where: { $0.album.albumId == albumID })?
                        .stickers ?? [],
                    emptyTitle: "No Stickers",
                    onSelect: sendSticker,
                    onDetail: { detailStickerID = $0 }
                )
            }
        }
    }

    private var tabBar: some View {
        ScrollView(.horizontal) {
            HStack(spacing: 4) {
                Button {
                    Task {
                        await model.markStoreViewed()
                        storePresented = true
                    }
                } label: {
                    ZStack(alignment: .topTrailing) {
                        Image(systemName: "storefront")
                        if model.hasNewAlbum {
                            Circle()
                                .fill(.red)
                                .frame(width: 7, height: 7)
                        }
                    }
                    .frame(width: 36, height: 34)
                }
                .buttonStyle(.plain)
                .help("Sticker Store")

                tabButton(.emoji, icon: "face.smiling")
                tabButton(.recent, icon: "clock")
                tabButton(.favorite, icon: "heart")
                if gifAPIKey != nil {
                    tabButton(.gif, title: "GIF")
                }
                ForEach(model.library?.albums ?? [], id: \.album.albumId) { section in
                    Button {
                        selectedTab = .album(section.album.albumId)
                    } label: {
                        StickerImage(
                            urlString: section.album.iconUrl,
                            assetType: nil
                        )
                        .frame(width: 28, height: 28)
                        .padding(3)
                        .background(
                            selectedTab == .album(section.album.albumId)
                                ? Color.accentColor.opacity(0.14)
                                : Color.clear
                        )
                        .clipShape(RoundedRectangle(cornerRadius: 7))
                    }
                    .buttonStyle(.plain)
                    .help(section.album.name)
                }
            }
            .padding(.horizontal, 8)
            .frame(height: 49)
        }
        .background(Color.secondary.opacity(0.06))
    }

    private func tabButton(
        _ tab: StickerPanelTab,
        icon: String? = nil,
        title: String? = nil
    ) -> some View {
        Button {
            selectedTab = tab
        } label: {
            Group {
                if let icon {
                    Image(systemName: icon)
                } else {
                    Text(title ?? "")
                        .font(.caption.bold())
                }
            }
            .frame(width: 36, height: 34)
            .background(selectedTab == tab ? Color.accentColor.opacity(0.14) : Color.clear)
            .clipShape(RoundedRectangle(cornerRadius: 7))
        }
        .buttonStyle(.plain)
    }

    private func useEmoji(_ emoji: String) {
        draft.append(emoji)
        Task {
            await model.recordEmoji(emoji)
        }
    }

    private func sendSticker(_ stickerID: String) {
        Task {
            if await onSendSticker(stickerID) {
                await model.reloadLocal()
                onSent()
            }
        }
    }
}

private enum StickerPanelTab: Hashable {
    case emoji
    case recent
    case favorite
    case gif
    case album(String)
}

@MainActor
@Observable
final class StickerPanelModel {
    private(set) var library: SwiftStickerLibrary?
    private(set) var store: [SwiftStickerAlbumSection] = []
    private(set) var recentEmojis: [String] = []
    private(set) var hasNewAlbum = false
    private(set) var loading = false
    private(set) var storeLoading = false
    private(set) var mutating = false
    private(set) var error: String?
    private(set) var storeError: String?

    let account: SwiftAccountHandle
    private let desktop: SwiftDesktopHandle
    private let accountID: String
    private var initialized = false

    init(
        account: SwiftAccountHandle,
        desktop: SwiftDesktopHandle,
        accountID: String
    ) {
        self.account = account
        self.desktop = desktop
        self.accountID = accountID
    }

    func load(force: Bool = false) async {
        guard !loading, force || !initialized else {
            return
        }
        loading = true
        error = nil
        defer { loading = false }
        do {
            async let localLibrary = account.stickerLibrary()
            async let storedEmoji = desktop.setting(key: "recent_used_emoji")
            async let storedNewAlbum = desktop.setting(key: "has_new_sticker_album_\(accountID)")
            library = try await localLibrary
            recentEmojis = Self.decodeEmoji(try await storedEmoji)
            hasNewAlbum = try await storedNewAlbum == "true"
            initialized = true
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
            return
        }
        await refreshRemoteIfNeeded(force: force)
    }

    func reloadLocal() async {
        do {
            library = try await account.stickerLibrary()
            error = nil
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    func loadStore(force: Bool = false) async {
        guard !storeLoading else {
            return
        }
        storeLoading = true
        storeError = nil
        defer { storeLoading = false }
        do {
            store = try await account.stickerStore()
            if force {
                let newAlbum = try await account.refreshStickers()
                hasNewAlbum = hasNewAlbum || newAlbum
                store = try await account.stickerStore()
                await reloadLocal()
            }
        } catch {
            storeError = MixinErrorPresenter.message(for: error)
        }
    }

    func setAlbumAdded(_ albumID: String, added: Bool) async {
        guard !mutating else {
            return
        }
        mutating = true
        defer { mutating = false }
        do {
            try await account.setStickerAlbumAdded(albumId: albumID, added: added)
            async let localReload: Void = reloadLocal()
            async let storeReload: Void = loadStore()
            _ = await (localReload, storeReload)
        } catch {
            storeError = MixinErrorPresenter.message(for: error)
        }
    }

    func setAlbumOrder(_ albumIDs: [String]) async {
        do {
            try await account.setStickerAlbumOrder(albumIds: albumIDs)
            await reloadLocal()
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    func removeSticker(_ stickerID: String) async {
        guard !mutating else {
            return
        }
        mutating = true
        defer { mutating = false }
        do {
            try await account.removeSticker(stickerId: stickerID)
            await reloadLocal()
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    func pickAndAddSticker() async {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.png, .jpeg, .gif, .webP]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        guard await panel.begin() == .OK, let url = panel.url else {
            return
        }
        let access = url.startAccessingSecurityScopedResource()
        defer {
            if access {
                url.stopAccessingSecurityScopedResource()
            }
        }
        mutating = true
        defer { mutating = false }
        do {
            try await account.addStickerFromPath(path: url.path)
            await reloadLocal()
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    func recordEmoji(_ emoji: String) async {
        recentEmojis = [emoji] + Array(
            recentEmojis
                .filter { $0 != emoji }
                .prefix(34)
        )
        guard let data = try? JSONEncoder().encode(recentEmojis),
              let value = String(data: data, encoding: .utf8)
        else {
            return
        }
        try? await desktop.setSetting(key: "recent_used_emoji", value: value)
    }

    func markStoreViewed() async {
        guard hasNewAlbum else {
            return
        }
        hasNewAlbum = false
        try? await desktop.setSetting(
            key: "has_new_sticker_album_\(accountID)",
            value: "false"
        )
    }

    private func refreshRemoteIfNeeded(force: Bool) async {
        let refreshKey = "sticker_refresh_at_\(accountID)"
        let lastRefresh = Int64((try? await desktop.setting(key: refreshKey)) ?? "")
        let now = Int64(Date().timeIntervalSince1970 * 1000)
        let refreshInterval: Int64 = 24 * 60 * 60 * 1000
        guard force || lastRefresh == nil || now - (lastRefresh ?? 0) >= refreshInterval else {
            return
        }
        do {
            let newAlbum = try await account.refreshStickers()
            if newAlbum {
                hasNewAlbum = true
                try await desktop.setSetting(
                    key: "has_new_sticker_album_\(accountID)",
                    value: "true"
                )
            }
            try await desktop.setSetting(key: refreshKey, value: String(now))
            await reloadLocal()
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }

    private static func decodeEmoji(_ value: String?) -> [String] {
        guard let value,
              let data = value.data(using: .utf8),
              let emojis = try? JSONDecoder().decode([String].self, from: data)
        else {
            return []
        }
        return emojis
    }
}

private struct StickerGrid: View {
    let stickers: [SwiftStickerItem]
    let emptyTitle: String
    var includesAddButton = false
    var onAdd: (() -> Void)?
    let onSelect: (String) -> Void
    var onDelete: ((String) -> Void)?
    let onDetail: (String) -> Void

    var body: some View {
        if stickers.isEmpty, !includesAddButton {
            ContentUnavailableView(emptyTitle, systemImage: "face.smiling")
        } else {
            ScrollView {
                LazyVGrid(
                    columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4),
                    spacing: 8
                ) {
                    if includesAddButton {
                        Button {
                            onAdd?()
                        } label: {
                            Image(systemName: "plus")
                                .font(.system(size: 28))
                                .frame(maxWidth: .infinity, minHeight: 82)
                        }
                        .buttonStyle(.plain)
                        .background(Color.secondary.opacity(0.06))
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .help("Add Sticker")
                    }
                    ForEach(stickers, id: \.stickerId) { sticker in
                        Button {
                            onSelect(sticker.stickerId)
                        } label: {
                            StickerImage(
                                urlString: sticker.assetUrl,
                                assetType: sticker.assetType,
                                stickerID: sticker.stickerId
                            )
                            .frame(maxWidth: .infinity, minHeight: 82, maxHeight: 82)
                            .padding(5)
                        }
                        .buttonStyle(.plain)
                        .background(Color.secondary.opacity(0.001))
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .contextMenu {
                            Button("Details") {
                                onDetail(sticker.stickerId)
                            }
                            if let onDelete {
                                Button("Delete", role: .destructive) {
                                    onDelete(sticker.stickerId)
                                }
                            }
                        }
                        .help(sticker.name)
                    }
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 10)
            }
        }
    }
}
