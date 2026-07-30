import SwiftUI

struct AppAction: Decodable, Hashable {
    let label: String
    let action: String
    let color: String

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        label = try values.decodeIfPresent(String.self, forKey: .label) ?? ""
        action = try values.decodeIfPresent(String.self, forKey: .action) ?? ""
        color = try values.decodeIfPresent(String.self, forKey: .color) ?? ""
    }

    private enum CodingKeys: CodingKey {
        case label
        case action
        case color
    }

    private var url: URL? {
        URL(string: action)
    }

    var isExternalLink: Bool {
        guard let url, ["http", "https"].contains(url.scheme?.lowercased() ?? "") else {
            return false
        }
        let mixinActionPaths: Set<String> = [
            "codes", "pay", "users", "transfer", "device", "send", "address",
            "withdrawal", "apps", "snapshots", "conversations", "multisigs", "swap",
            "markets", "membership",
        ]
        let host = url.host?.lowercased()
        let path = url.pathComponents.dropFirst().first?.lowercased()
        return !(["mixin.one", "www.mixin.one"].contains(host ?? "")
            && path.map(mixinActionPaths.contains) == true)
    }

    var isSendUserLink: Bool {
        guard let url,
              URLComponents(url: url, resolvingAgainstBaseURL: false)?
                .queryItems?
                .first(where: { $0.name == "user" })?
                .value?
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty == false
        else {
            return false
        }
        let host = url.host?.lowercased()
        let path = url.pathComponents.dropFirst().first?.lowercased()
        return (url.scheme?.lowercased() == "mixin" && host == "send")
            || (["mixin.one", "www.mixin.one"].contains(host ?? "") && path == "send")
    }
}

struct AppCardContent: Decodable {
    struct Cover: Decodable {
        let url: String
        let thumbnail: String?
        let mimeType: String
        let width: Int
        let height: Int

        private enum CodingKeys: String, CodingKey {
            case url
            case thumbnail
            case mimeType = "mime_type"
            case width
            case height
        }
    }

    let appID: String
    let iconURL: String
    let coverURL: String
    let cover: Cover?
    let title: String
    let description: String
    let action: String
    let actions: [AppAction]

    private enum CodingKeys: String, CodingKey {
        case appID = "app_id"
        case iconURL = "icon_url"
        case coverURL = "cover_url"
        case cover
        case title
        case description
        case action
        case actions
    }

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        appID = try values.decodeIfPresent(String.self, forKey: .appID) ?? ""
        iconURL = try values.decodeIfPresent(String.self, forKey: .iconURL) ?? ""
        coverURL = try values.decodeIfPresent(String.self, forKey: .coverURL) ?? ""
        cover = try values.decodeIfPresent(Cover.self, forKey: .cover)
        title = try values.decodeIfPresent(String.self, forKey: .title) ?? ""
        description = try values.decodeIfPresent(String.self, forKey: .description) ?? ""
        action = try values.decodeIfPresent(String.self, forKey: .action) ?? ""
        actions = try values.decodeIfPresent([AppAction].self, forKey: .actions) ?? []
    }
}

struct AppMessageView: View {
    let message: MessageItem
    var outgoing = false
    let onAction: (String, String) -> Void
    @Environment(\.mixinTheme) private var theme
    @Environment(\.colorScheme) private var colorScheme

    private var card: AppCardContent? {
        guard message.category.uppercased() == "APP_CARD" else {
            return nil
        }
        return try? JSONDecoder().decode(
            AppCardContent.self,
            from: Data(message.content.utf8)
        )
    }

    private var actions: [AppAction]? {
        guard message.category.uppercased() == "APP_BUTTON_GROUP" else {
            return nil
        }
        return try? JSONDecoder().decode(
            [AppAction].self,
            from: Data(message.content.utf8)
        )
    }

    var body: some View {
        if let card {
            cardContent(card)
        } else if let actions {
            actionGrid(actions, title: "")
        } else {
            Label("Unsupported app message", systemImage: "app.dashed")
                .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private func cardContent(_ card: AppCardContent) -> some View {
        if card.action.isEmpty {
            actionsCardContent(card)
        } else {
            compactCardHeader(card)
        }
    }

    private func compactCardHeader(_ card: AppCardContent) -> some View {
        Button {
            onAction(card.action, card.title)
        } label: {
            HStack(spacing: 8) {
                MixinRemoteImage(url: URL(string: card.iconURL)) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Rectangle().fill(theme.background)
                }
                .frame(width: 40, height: 40)
                .clipShape(RoundedRectangle(cornerRadius: 4))

                VStack(alignment: .leading, spacing: 0) {
                    Text(card.title)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                        .lineLimit(1)
                    Text(card.description.split(separator: "\n").first.map(String.init) ?? "")
                        .font(.system(size: 12))
                        .foregroundStyle(theme.secondaryText)
                        .lineLimit(1)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func actionsCardContent(_ card: AppCardContent) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            actionsCardBody(card)
                .background {
                    MessageBubbleShape(outgoing: outgoing, showsNip: true)
                        .fill(actionsCardBubbleColor)
                        .shadow(color: .black.opacity(0.22), radius: 0.6, y: 0.3)
                }
                .clipShape(MessageBubbleShape(outgoing: outgoing, showsNip: true))
            if !card.actions.isEmpty {
                actionGrid(card.actions, title: card.title)
                    .padding(outgoing ? .trailing : .leading, 9)
            }
        }
        .frame(minWidth: 320, maxWidth: 375, alignment: .leading)
    }

    private func actionsCardBody(_ card: AppCardContent) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            cardCover(card)
            Text(card.title)
                .font(.system(size: 16, weight: .bold))
                .foregroundStyle(theme.text)
                .padding(.horizontal, 12)
                .padding(.top, card.resolvedCoverURL.isEmpty ? 0 : 10)
            Text(card.description)
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
                .padding(.horizontal, 12)
                .padding(.top, 8)
            Spacer().frame(height: 10)
        }
    }

    private var actionsCardBubbleColor: Color {
        if outgoing {
            return colorScheme == .dark
                ? Color(red: 59 / 255, green: 79 / 255, blue: 103 / 255)
                : Color(red: 197 / 255, green: 237 / 255, blue: 253 / 255)
        }
        return colorScheme == .dark
            ? Color(red: 52 / 255, green: 59 / 255, blue: 67 / 255)
            : .white
    }

    @ViewBuilder
    private func cardCover(_ card: AppCardContent) -> some View {
        if let coverURL = URL(string: card.resolvedCoverURL), !card.resolvedCoverURL.isEmpty {
            MixinRemoteImage(url: coverURL) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Rectangle().fill(theme.background)
            }
            .aspectRatio(card.coverURL.isEmpty ? max(card.coverAspectRatio, 1.5) : 1, contentMode: .fill)
            .clipped()
        } else {
            Spacer().frame(height: 10)
        }
    }

    private func actionGrid(_ actions: [AppAction], title: String) -> some View {
        AppActionButtonLayout {
            ForEach(Array(actions.enumerated()), id: \.offset) { _, action in
                Button(action.label) {
                    onAction(action.action, title)
                }
                .buttonStyle(.plain)
                .foregroundStyle(Color(hex: action.color) ?? .accentColor)
                .font(.system(size: 14))
                .padding(.horizontal, 14)
                .frame(height: 30)
                .background(theme.primary)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(alignment: .topTrailing) {
                    if action.isSendUserLink {
                        Image(systemName: "paperplane.fill")
                            .font(.system(size: 8))
                            .foregroundStyle(theme.secondaryText)
                            .padding(6)
                    } else if action.isExternalLink {
                        Image(systemName: "arrow.up.right")
                            .font(.system(size: 6, weight: .bold))
                            .foregroundStyle(theme.secondaryText)
                            .padding(6)
                    }
                }
            }
        }
        .frame(minWidth: 240, maxWidth: 340)
    }
}

struct AppActionButtonLayout: Layout {
    private let spacing: CGFloat = 8

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout ()
    ) -> CGSize {
        let views = Array(subviews)
        let naturalSizes = views.map { $0.sizeThatFits(.unspecified) }
        let width = proposal.width ?? naturalSizes.map(\.width).max() ?? 0
        let rows = makeRows(sizes: naturalSizes, width: width)
        let height = rows.reduce(CGFloat.zero) { total, row in
            total + row.map { naturalSizes[$0].height }.max()!
        } + CGFloat(max(rows.count - 1, 0)) * spacing
        return CGSize(width: width, height: height)
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout ()
    ) {
        let views = Array(subviews)
        let naturalSizes = views.map { $0.sizeThatFits(.unspecified) }
        let rows = makeRows(sizes: naturalSizes, width: bounds.width)
        var y = bounds.minY
        for row in rows {
            let height = row.map { naturalSizes[$0].height }.max()!
            let itemWidth = (bounds.width - CGFloat(row.count - 1) * spacing) / CGFloat(row.count)
            for (column, index) in row.enumerated() {
                views[index].place(
                    at: CGPoint(x: bounds.minX + CGFloat(column) * (itemWidth + spacing), y: y),
                    anchor: .topLeading,
                    proposal: ProposedViewSize(width: itemWidth, height: height)
                )
            }
            y += height + spacing
        }
    }

    private func makeRows(sizes: [CGSize], width: CGFloat) -> [[Int]] {
        let threeColumnWidth = (width - spacing * 2) / 3
        let twoColumnWidth = (width - spacing) / 2
        var rows: [[Int]] = []
        var row: [Int] = []
        var occupied = 0

        for (index, size) in sizes.enumerated() {
            if size.width <= threeColumnWidth {
                row.append(index)
                occupied = occupied == 0 || occupied == 2 || occupied == 4
                    ? occupied + 2
                    : 6
            } else if size.width <= twoColumnWidth {
                if occupied == 0 {
                    row.append(index)
                    occupied = 3
                } else if occupied == 2 || occupied == 3 {
                    row.append(index)
                    occupied = 6
                } else {
                    rows.append(row)
                    row = [index]
                    occupied = 2
                }
            } else if occupied == 0 {
                row.append(index)
                occupied = 6
            } else {
                rows.append(row)
                rows.append([index])
                row = []
                occupied = 0
            }

            if occupied == 6 {
                rows.append(row)
                row = []
                occupied = 0
            }
        }
        if !row.isEmpty {
            rows.append(row)
        }
        return rows
    }
}

extension AppCardContent {
    var resolvedCoverURL: String {
        coverURL.isEmpty ? cover?.url ?? "" : coverURL
    }

    var coverAspectRatio: CGFloat {
        guard let cover, cover.height > 0 else { return 1 }
        return CGFloat(cover.width) / CGFloat(cover.height)
    }
}

extension Color {
    init?(hex: String) {
        var value = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        if value.hasPrefix("#") {
            value.removeFirst()
        }
        guard
            value.count == 6,
            let rgb = UInt64(value, radix: 16)
        else {
            return nil
        }
        self.init(
            red: Double((rgb >> 16) & 0xff) / 255,
            green: Double((rgb >> 8) & 0xff) / 255,
            blue: Double(rgb & 0xff) / 255
        )
    }
}
