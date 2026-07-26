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
    let message: SwiftMessageItem
    let onAction: (String, String) -> Void

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
        VStack(alignment: .leading, spacing: 10) {
            if let coverURL = URL(string: card.resolvedCoverURL),
               !card.resolvedCoverURL.isEmpty
            {
                MixinRemoteImage(url: coverURL) { image in
                    image
                        .resizable()
                        .scaledToFill()
                } placeholder: {
                    Rectangle()
                        .fill(.quaternary)
                }
                .frame(maxWidth: 320)
                .frame(height: 150)
                .clipped()
                .clipShape(RoundedRectangle(cornerRadius: 8))
            }
            Button {
                if !card.action.isEmpty {
                    onAction(card.action, card.title)
                }
            } label: {
                HStack(alignment: .top, spacing: 10) {
                    MixinRemoteImage(url: URL(string: card.iconURL)) { image in
                        image
                            .resizable()
                            .scaledToFill()
                    } placeholder: {
                        Image(systemName: "app.fill")
                    }
                    .frame(width: 42, height: 42)
                    .clipShape(RoundedRectangle(cornerRadius: 9))
                    VStack(alignment: .leading, spacing: 3) {
                        Text(card.title)
                            .font(.headline)
                            .foregroundStyle(.primary)
                        if !card.description.isEmpty {
                            Text(card.description)
                                .font(.callout)
                                .foregroundStyle(.secondary)
                                .lineLimit(3)
                        }
                    }
                    Spacer(minLength: 0)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(card.action.isEmpty)

            if !card.actions.isEmpty {
                actionGrid(card.actions, title: card.title)
            }
        }
        .frame(
            minWidth: card.action.isEmpty ? 320 : 240,
            maxWidth: card.action.isEmpty ? 375 : 340,
            alignment: .leading
        )
    }

    private func actionGrid(_ actions: [AppAction], title: String) -> some View {
        VStack(spacing: 1) {
            ForEach(Array(actions.enumerated()), id: \.offset) { _, action in
                Button(action.label) {
                    onAction(action.action, title)
                }
                .buttonStyle(.plain)
                .foregroundStyle(Color(hex: action.color) ?? .accentColor)
                .frame(maxWidth: .infinity, minHeight: 34)
                .background(.quaternary.opacity(0.5))
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 7))
        .frame(minWidth: 240, maxWidth: 340)
    }
}

private extension AppCardContent {
    var resolvedCoverURL: String {
        coverURL.isEmpty ? cover?.url ?? "" : coverURL
    }
}

private extension Color {
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
