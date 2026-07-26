import Foundation
import Observation
import SwiftUI

struct GiphyItem: Identifiable, Hashable {
    let id: String
    let url: String
    let previewURL: String
    let width: Int32?
    let height: Int32?
}

enum GiphyConfiguration {
    static var apiKey: String? {
        let environmentValue = ProcessInfo.processInfo.environment["MIXIN_GIPHY_KEY"]
        let bundleValue = Bundle.main.object(forInfoDictionaryKey: "MIXIN_GIPHY_KEY") as? String
        return [environmentValue, bundleValue]
            .compactMap { $0?.trimmingCharacters(in: .whitespacesAndNewlines) }
            .first { !$0.isEmpty && !$0.contains("$(") }
    }
}

struct GiphyPickerView: View {
    @State private var model: GiphyPickerModel
    @State private var query = ""
    let onSelect: (GiphyItem) async -> Void

    init(
        apiKey: String,
        onSelect: @escaping (GiphyItem) async -> Void
    ) {
        _model = State(initialValue: GiphyPickerModel(apiKey: apiKey))
        self.onSelect = onSelect
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 7) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField("Search GIFs", text: $query)
                    .textFieldStyle(.plain)
                if !query.isEmpty {
                    Button {
                        query = ""
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal, 12)
            .frame(height: 47)

            Divider()

            Group {
                if model.loading, model.items.isEmpty {
                    ProgressView("Loading GIFs…")
                } else if let error = model.error, model.items.isEmpty {
                    ContentUnavailableView {
                        Label("Unable to load GIFs", systemImage: "exclamationmark.triangle")
                    } description: {
                        Text(error)
                    } actions: {
                        Button("Retry") {
                            Task {
                                await model.reload(query: query)
                            }
                        }
                    }
                } else if model.items.isEmpty {
                    ContentUnavailableView.search(text: query)
                } else {
                    gifGrid
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task {
            await model.reload(query: "")
        }
        .task(id: query) {
            try? await Task.sleep(for: .seconds(1))
            guard !Task.isCancelled else {
                return
            }
            await model.reload(query: query)
        }
    }

    private var gifGrid: some View {
        ScrollView {
            LazyVGrid(
                columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4),
                spacing: 8
            ) {
                ForEach(model.items) { item in
                    Button {
                        Task {
                            await onSelect(item)
                        }
                    } label: {
                        MixinAsyncImage(url: URL(string: item.previewURL)) { phase in
                            switch phase {
                            case let .success(image):
                                image
                                    .resizable()
                                    .scaledToFill()
                            case .failure:
                                Image(systemName: "photo")
                                    .foregroundStyle(.secondary)
                            case .empty:
                                ProgressView()
                                    .controlSize(.small)
                            }
                        }
                        .frame(height: 90)
                        .frame(maxWidth: .infinity)
                        .background(Color.secondary.opacity(0.08))
                        .clipShape(RoundedRectangle(cornerRadius: 7))
                    }
                    .buttonStyle(.plain)
                    .onAppear {
                        guard item.id == model.items.last?.id else {
                            return
                        }
                        Task {
                            await model.loadMore()
                        }
                    }
                }
                if model.loading {
                    ProgressView()
                        .frame(maxWidth: .infinity, minHeight: 70)
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
        }
    }
}

@MainActor
@Observable
private final class GiphyPickerModel {
    private(set) var items: [GiphyItem] = []
    private(set) var loading = false
    private(set) var hasMore = true
    private(set) var error: String?

    private let apiKey: String
    private var query = ""
    private var requestVersion = 0
    private let limit = 51

    init(apiKey: String) {
        self.apiKey = apiKey
    }

    func reload(query: String) async {
        self.query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        items = []
        hasMore = true
        error = nil
        requestVersion += 1
        await load(version: requestVersion)
    }

    func loadMore() async {
        await load(version: requestVersion)
    }

    private func load(version: Int) async {
        guard !loading, hasMore, version == requestVersion else {
            return
        }
        loading = true
        defer {
            if version == requestVersion {
                loading = false
            }
        }
        do {
            var components = URLComponents(
                string: "https://api.giphy.com/v1/gifs/\(query.isEmpty ? "trending" : "search")"
            )
            components?.queryItems = [
                query.isEmpty ? nil : URLQueryItem(name: "q", value: query),
                URLQueryItem(name: "limit", value: String(limit)),
                URLQueryItem(name: "offset", value: String(items.count)),
                URLQueryItem(name: "api_key", value: apiKey),
            ].compactMap { $0 }
            guard let url = components?.url else {
                throw URLError(.badURL)
            }
            let (data, response) = try await URLSession.shared.data(from: url)
            guard let http = response as? HTTPURLResponse,
                  (200 ..< 300).contains(http.statusCode)
            else {
                throw URLError(.badServerResponse)
            }
            let responseBody = try JSONDecoder().decode(GiphyResponse.self, from: data)
            guard version == requestVersion else {
                return
            }
            let page = responseBody.data.compactMap(\.item)
            items.append(contentsOf: page)
            hasMore = page.count >= limit
            error = nil
        } catch is CancellationError {
            return
        } catch {
            guard version == requestVersion else {
                return
            }
            self.error = MixinErrorPresenter.message(for: error)
        }
    }
}

private struct GiphyResponse: Decodable {
    let data: [GiphyGIF]
}

private struct GiphyGIF: Decodable {
    let id: String
    let images: GiphyImages

    var item: GiphyItem? {
        guard !images.fixedWidth.url.isEmpty,
              !images.fixedWidthDownsampled.url.isEmpty
        else {
            return nil
        }
        return GiphyItem(
            id: id,
            url: images.fixedWidth.url,
            previewURL: images.fixedWidthDownsampled.url,
            width: Int32(images.fixedWidth.width),
            height: Int32(images.fixedWidth.height)
        )
    }
}

private struct GiphyImages: Decodable {
    let fixedWidthDownsampled: GiphyImage
    let fixedWidth: GiphyImage

    enum CodingKeys: String, CodingKey {
        case fixedWidthDownsampled = "fixed_width_downsampled"
        case fixedWidth = "fixed_width"
    }
}

private struct GiphyImage: Decodable {
    let url: String
    let width: String
    let height: String
}
