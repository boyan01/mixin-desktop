import Observation
import SwiftUI

struct StickerMessageDetailSheet: View {
    @State private var model: StickerPanelModel
    let stickerID: String

    init(
        account: SwiftAccountHandle,
        desktop: SwiftDesktopHandle,
        accountID: String,
        stickerID: String
    ) {
        _model = State(initialValue: StickerPanelModel(
            account: account,
            desktop: desktop,
            accountID: accountID
        ))
        self.stickerID = stickerID
    }

    var body: some View {
        StickerDetailView(model: model, stickerID: stickerID)
    }
}

struct StickerStoreView: View {
    @Bindable var model: StickerPanelModel

    var body: some View {
        NavigationStack {
            Group {
                if model.storeLoading, model.store.isEmpty {
                    ProgressView("Loading sticker store…")
                } else if let error = model.storeError, model.store.isEmpty {
                    ContentUnavailableView {
                        Label("Unable to load sticker store", systemImage: "exclamationmark.triangle")
                    } description: {
                        Text(error)
                    } actions: {
                        Button("Retry") {
                            Task {
                                await model.loadStore(force: true)
                            }
                        }
                    }
                } else if model.store.isEmpty {
                    ContentUnavailableView("No Sticker Albums", systemImage: "square.grid.2x2")
                } else {
                    List(model.store, id: \.album.albumId) { section in
                        NavigationLink {
                            StickerStoreAlbumView(
                                model: model,
                                section: section
                            )
                        } label: {
                            StickerStoreAlbumRow(
                                model: model,
                                section: section
                            )
                        }
                    }
                    .listStyle(.plain)
                }
            }
            .navigationTitle("Sticker Store")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    NavigationLink {
                        StickerAlbumManageView(model: model)
                    } label: {
                        Label("Manage", systemImage: "slider.horizontal.3")
                    }
                }
            }
        }
        .frame(width: 480, height: 600)
        .task {
            await model.loadStore(force: true)
        }
    }
}

private struct StickerStoreAlbumRow: View {
    @Bindable var model: StickerPanelModel
    let section: SwiftStickerAlbumSection

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(section.album.name)
                    .font(.headline)
                if section.album.isVerified {
                    Image(systemName: "checkmark.seal.fill")
                        .foregroundStyle(Color.accentColor)
                }
                Spacer()
                Button(section.album.added ? "Added" : "Add") {
                    guard !section.album.added else {
                        return
                    }
                    Task {
                        await model.setAlbumAdded(section.album.albumId, added: true)
                    }
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
                .disabled(section.album.added || model.mutating)
            }
            HStack(spacing: 8) {
                ForEach(section.stickers.prefix(4), id: \.stickerId) { sticker in
                    StickerImage(
                        urlString: sticker.assetUrl,
                        assetType: sticker.assetType,
                        stickerID: sticker.stickerId
                    )
                    .frame(width: 68, height: 68)
                }
            }
        }
        .padding(.vertical, 8)
    }
}

private struct StickerStoreAlbumView: View {
    @Bindable var model: StickerPanelModel
    let section: SwiftStickerAlbumSection

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                LazyVGrid(
                    columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 4),
                    spacing: 10
                ) {
                    ForEach(section.stickers, id: \.stickerId) { sticker in
                        StickerImage(
                            urlString: sticker.assetUrl,
                            assetType: sticker.assetType,
                            stickerID: sticker.stickerId
                        )
                        .frame(height: 86)
                    }
                }
                .padding(24)
            }

            Divider()
            Button(section.album.added ? "Remove Stickers" : "Add Stickers") {
                Task {
                    await model.setAlbumAdded(
                        section.album.albumId,
                        added: !section.album.added
                    )
                }
            }
            .buttonStyle(.borderedProminent)
            .tint(section.album.added ? .red : .accentColor)
            .disabled(model.mutating)
            .padding(18)
        }
        .navigationTitle(section.album.name)
    }
}

private struct StickerAlbumManageView: View {
    @Bindable var model: StickerPanelModel
    @State private var albums: [SwiftStickerAlbumSection] = []

    var body: some View {
        Group {
            if albums.isEmpty {
                ContentUnavailableView("No Sticker Albums", systemImage: "square.grid.2x2")
            } else {
                List {
                    ForEach(Array(albums.enumerated()), id: \.element.album.albumId) { index, section in
                        HStack(spacing: 12) {
                            StickerImage(
                                urlString: section.album.iconUrl,
                                assetType: nil
                            )
                            .frame(width: 52, height: 52)
                            Text(section.album.name)
                                .lineLimit(1)
                            Spacer()
                            Button {
                                move(index, by: -1)
                            } label: {
                                Image(systemName: "chevron.up")
                            }
                            .buttonStyle(.plain)
                            .disabled(index == albums.startIndex)
                            .help("Move Up")
                            Button {
                                move(index, by: 1)
                            } label: {
                                Image(systemName: "chevron.down")
                            }
                            .buttonStyle(.plain)
                            .disabled(index == albums.index(before: albums.endIndex))
                            .help("Move Down")
                            Button(role: .destructive) {
                                let albumID = section.album.albumId
                                albums.remove(at: index)
                                Task {
                                    await model.setAlbumAdded(albumID, added: false)
                                }
                            } label: {
                                Image(systemName: "trash")
                            }
                            .buttonStyle(.plain)
                            .help("Remove Album")
                        }
                        .padding(.vertical, 6)
                    }
                }
                .listStyle(.plain)
            }
        }
        .navigationTitle("My Stickers")
        .onAppear {
            albums = model.library?.albums ?? []
        }
    }

    private func move(_ index: Int, by offset: Int) {
        let destination = index + offset
        guard albums.indices.contains(destination) else {
            return
        }
        albums.swapAt(index, destination)
        Task {
            await model.setAlbumOrder(albums.map(\.album.albumId))
        }
    }
}

struct StickerDetailView: View {
    @Bindable var model: StickerPanelModel
    let stickerID: String
    @State private var detail: SwiftStickerDetailItem?
    @State private var selectedStickerID: String?
    @State private var error: String?
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 16) {
            HStack {
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
            }

            if let detail {
                let selected = detail.albumStickers
                    .first(where: { $0.stickerId == selectedStickerID })
                    ?? detail.sticker
                StickerImage(
                    urlString: selected.assetUrl,
                    assetType: selected.assetType,
                    stickerID: selected.stickerId
                )
                .frame(width: 256, height: 256)

                if let album = detail.album {
                    HStack {
                        Text(album.name)
                            .font(.headline)
                        Spacer()
                        Button(album.added ? "Remove Stickers" : "Add Stickers") {
                            Task {
                                await model.setAlbumAdded(
                                    album.albumId,
                                    added: !album.added
                                )
                                dismiss()
                            }
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(album.added ? .red : .accentColor)
                    }
                    ScrollView(.horizontal) {
                        HStack(spacing: 8) {
                            ForEach(detail.albumStickers, id: \.stickerId) { sticker in
                                Button {
                                    selectedStickerID = sticker.stickerId
                                } label: {
                                    StickerImage(
                                        urlString: sticker.assetUrl,
                                        assetType: sticker.assetType,
                                        stickerID: sticker.stickerId
                                    )
                                    .frame(width: 62, height: 62)
                                    .padding(4)
                                    .background(
                                        selected.stickerId == sticker.stickerId
                                            ? Color.accentColor.opacity(0.12)
                                            : Color.clear
                                    )
                                    .clipShape(RoundedRectangle(cornerRadius: 8))
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
            } else if let error {
                ContentUnavailableView {
                    Label("Unable to load sticker", systemImage: "exclamationmark.triangle")
                } description: {
                    Text(error)
                } actions: {
                    Button("Retry") {
                        Task {
                            await load()
                        }
                    }
                }
            } else {
                ProgressView()
                    .frame(width: 256, height: 256)
            }
        }
        .padding(24)
        .frame(width: 480, height: 500)
        .task(id: stickerID) {
            await load()
        }
    }

    private func load() async {
        detail = nil
        error = nil
        do {
            let value = try await model.account.stickerDetail(stickerId: stickerID)
            detail = value
            selectedStickerID = value.sticker.stickerId
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }
}
