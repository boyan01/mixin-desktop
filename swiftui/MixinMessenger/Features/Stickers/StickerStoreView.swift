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
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                StickerStoreHeader(
                    title: "Sticker Store",
                    onClose: { dismiss() },
                    leading: AnyView(
                        NavigationLink {
                            StickerAlbumManageView(
                                model: model,
                                onClose: { dismiss() }
                            )
                        } label: {
                            Image("Setting")
                                .resizable()
                                .renderingMode(.template)
                                .foregroundStyle(theme.icon)
                                .frame(width: 24, height: 24)
                                .frame(width: 56, height: 64)
                        }
                        .buttonStyle(.plain)
                        .help("Manage stickers")
                    )
                )

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
                } else {
                    AppScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(model.store, id: \.album.albumId) { section in
                                NavigationLink {
                                    StickerStoreAlbumView(
                                        model: model,
                                        section: section,
                                        onClose: { dismiss() }
                                    )
                                } label: {
                                    StickerStoreAlbumRow(
                                        model: model,
                                        section: section
                                    )
                                }
                                .buttonStyle(.plain)
                            }
                        }
                        .padding(.vertical, 16)
                    }
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
    let section: StickerAlbumSection

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(section.album.name)
                .font(.system(size: 16, weight: .semibold))
                .padding(.bottom, 4)
            HStack(spacing: 0) {
                HStack(spacing: 8) {
                    ForEach(section.stickers.prefix(4), id: \.stickerId) { sticker in
                        StickerImage(
                            urlString: sticker.assetUrl,
                            assetType: sticker.assetType,
                            stickerID: sticker.stickerId
                        )
                        .frame(width: 72, height: 72)
                    }
                }
                Spacer(minLength: 0)
                Button(section.album.added ? "Added" : "Add") {
                    Task {
                        await model.setAlbumAdded(
                            section.album.albumId,
                            added: !section.album.added
                        )
                    }
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
                .font(.system(size: 14))
                .opacity(section.album.added ? 0.4 : 1)
                .disabled(model.mutating)
            }
        }
        .frame(height: 104, alignment: .top)
        .padding(.horizontal, 24)
    }
}

private struct StickerStoreAlbumView: View {
    @Bindable var model: StickerPanelModel
    let section: StickerAlbumSection
    let onClose: () -> Void
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            StickerStoreHeader(title: section.album.name, onClose: onClose)
            ZStack(alignment: .bottom) {
                AppScrollView {
                    LazyVGrid(
                        columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4),
                        spacing: 10
                    ) {
                        ForEach(section.stickers, id: \.stickerId) { sticker in
                            StickerImage(
                                urlString: sticker.assetUrl,
                                assetType: sticker.assetType,
                                stickerID: sticker.stickerId
                            )
                            .aspectRatio(1, contentMode: .fit)
                        }
                    }
                    .padding(.horizontal, 24)
                    .padding(.bottom, 112)
                }

                LinearGradient(
                    colors: [theme.popUp.opacity(0), theme.popUp.opacity(0.36), theme.popUp],
                    startPoint: .top,
                    endPoint: .bottom
                )
                .frame(height: 93)
                .overlay(alignment: .bottom) {
                    VStack(spacing: 0) {
                        Button(section.album.added ? "Remove Stickers" : "Add Stickers") {
                            Task {
                                await model.setAlbumAdded(
                                    section.album.albumId,
                                    added: !section.album.added
                                )
                            }
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(section.album.added ? theme.destructive : theme.accent)
                        .disabled(model.mutating)
                        Spacer().frame(height: 40)
                    }
                }
            }
        }
    }
}

private struct StickerAlbumManageView: View {
    @Bindable var model: StickerPanelModel
    let onClose: () -> Void
    @State private var albums: [StickerAlbumSection] = []

    var body: some View {
        VStack(spacing: 0) {
            StickerStoreHeader(title: "My Stickers", onClose: onClose)
            AppScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(Array(albums.enumerated()), id: \.element.album.albumId) { index, section in
                        HStack(spacing: 12) {
                            StickerImage(
                                urlString: section.album.iconUrl,
                                assetType: nil
                            )
                            .frame(width: 72, height: 72)
                            Text(section.album.name)
                                .font(.system(size: 16, weight: .semibold))
                                .lineLimit(1)
                            Spacer()
                            Button(role: .destructive) {
                                let albumID = section.album.albumId
                                albums.remove(at: index)
                                Task {
                                    await model.setAlbumAdded(albumID, added: false)
                                }
                            } label: {
                                Image("Delete")
                                    .resizable()
                                    .renderingMode(.template)
                                    .foregroundStyle(.secondary)
                                    .frame(width: 24, height: 24)
                            }
                            .buttonStyle(.plain)
                            .help("Remove Album")
                        }
                        .frame(height: 72)
                        .padding(.horizontal, 24)
                        .padding(.vertical, 16)
                        .contentShape(Rectangle())
                        .draggable(section.album.albumId)
                        .dropDestination(for: String.self) { items, _ in
                            guard
                                let sourceID = items.first,
                                let sourceIndex = albums.firstIndex(where: {
                                    $0.album.albumId == sourceID
                                }),
                                sourceIndex != index
                            else {
                                return false
                            }
                            let destination = index > sourceIndex
                                ? index + 1
                                : index
                            let moved = albums.remove(at: sourceIndex)
                            albums.insert(moved, at: min(destination, albums.count))
                            Task {
                                await model.setAlbumOrder(
                                    albums.map(\.album.albumId)
                                )
                            }
                            return true
                        }
                    }
                }
            }
        }
        .onAppear {
            albums = model.library?.albums ?? []
        }
    }
}

private struct StickerStoreHeader: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme

    let title: String
    let onClose: () -> Void
    let leading: AnyView?

    init(
        title: String,
        onClose: @escaping () -> Void,
        leading: AnyView? = nil
    ) {
        self.title = title
        self.onClose = onClose
        self.leading = leading
    }

    var body: some View {
        ZStack {
            Text(title)
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(theme.text)
                .lineLimit(1)

            HStack(spacing: 0) {
                if let leading {
                    leading
                } else {
                    Button {
                        dismiss()
                    } label: {
                        Image(systemName: "chevron.left")
                            .frame(width: 56, height: 64)
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(theme.icon)
                    .help("Back")
                }
                Spacer()
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .frame(width: 40, height: 40)
                }
                .buttonStyle(.plain)
                .foregroundStyle(theme.icon)
                .padding(.trailing, 8)
                .help("Close")
            }
        }
        .frame(height: 64)
    }
}

struct StickerDetailView: View {
    @Bindable var model: StickerPanelModel
    let stickerID: String
    @State private var detail: StickerDetailItem?
    @State private var selectedStickerID: String?
    @State private var albumAdded = false
    @State private var error: String?
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                Color.clear.frame(width: 36, height: 36)
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                }
                .buttonStyle(MixinActionButtonStyle())
                .padding(.trailing, 8)
            }
            .frame(height: 64)

            if let detail {
                let selected = detail.albumStickers
                    .first(where: { $0.stickerId == selectedStickerID })
                    ?? detail.sticker
                ZStack {
                    theme.background
                    StickerImage(
                        urlString: selected.assetUrl,
                        assetType: selected.assetType,
                        stickerID: selected.stickerId
                    )
                    .frame(width: 256, height: 256)
                }
                .frame(width: 368, height: 368)
                .padding(.horizontal, 56)

                if let album = detail.album {
                    HStack(spacing: 0) {
                        Text(album.name)
                            .font(.system(size: 16, weight: .semibold))
                            .foregroundStyle(theme.text)
                        Spacer()
                        Button(
                            albumAdded
                                ? "Remove Stickers"
                                : "Add Stickers"
                        ) {
                            Task {
                                await model.setAlbumAdded(
                                    album.albumId,
                                    added: !albumAdded
                                )
                                albumAdded.toggle()
                            }
                        }
                        .buttonStyle(.plain)
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(
                            albumAdded
                                ? theme.destructive
                                : theme.accent,
                            in: RoundedRectangle(cornerRadius: 5)
                        )
                    }
                    .padding(.horizontal, 24)
                    .padding(.bottom, 8)

                    AppScrollView(.horizontal) {
                        HStack(spacing: 0) {
                            ForEach(detail.albumStickers, id: \.stickerId) { sticker in
                                Button {
                                    selectedStickerID = sticker.stickerId
                                } label: {
                                    StickerImage(
                                        urlString: sticker.assetUrl,
                                        assetType: sticker.assetType,
                                        stickerID: sticker.stickerId
                                    )
                                    .frame(width: 64, height: 64)
                                    .padding(16)
                                    .background(
                                        selected.stickerId == sticker.stickerId
                                            ? (
                                                colorScheme == .dark
                                                    ? Color.white.opacity(0.06)
                                                    : Color(
                                                        red: 229 / 255,
                                                        green: 231 / 255,
                                                        blue: 235 / 255
                                                    )
                                            )
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
        .frame(width: 480)
        .background(theme.popUp)
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
            albumAdded = value.album?.added ?? false
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
    }
}
