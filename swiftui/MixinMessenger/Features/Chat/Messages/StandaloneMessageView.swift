import AppKit
import MapKit
import SwiftUI

struct SpecialMessageContentView: View {
    let message: SwiftMessageItem
    let mentionNames: [String: String]
    let onStrangerAction: (String) async -> Bool

    @ViewBuilder
    var body: some View {
        switch message.category.uppercased() {
        case "MESSAGE_PIN":
            SpecialPinMessageView(
                senderName: message.senderName,
                content: message.content,
                mentionNames: mentionNames
            )
        case "SECRET":
            SpecialSecretMessageView()
        case "STRANGER":
            SpecialStrangerMessageView(
                message: message,
                onAction: onStrangerAction
            )
        default:
            if message.category.uppercased().hasSuffix("_LOCATION") {
                SpecialLocationMessageView(content: message.content)
            } else {
                UnsupportedSpecialMessageView(category: message.category)
            }
        }
    }
}

struct WaitingMessageView: View {
    let subject: String

    private let helpURL = URL(
        string: "https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p"
    )!

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(
                "Waiting for \(subject) to get online and establish an encrypted session."
            )
            .fixedSize(horizontal: false, vertical: true)

            Link("Learn More", destination: helpURL)
                .foregroundStyle(Color.accentColor)
        }
        .accessibilityElement(children: .combine)
    }
}

struct UnsupportedMessageView: View {
    let category: String

    var body: some View {
        Label(
            "This type of message is not supported. Update Mixin to the latest version.",
            systemImage: "exclamationmark.bubble"
        )
        .foregroundStyle(.secondary)
        .help(category)
    }
}

private struct SpecialLocationMessageView: View {
    let content: String

    var body: some View {
        if let location = SharedLocation.parse(content) {
            Button {
                NSWorkspace.shared.open(location.googleMapsURL)
            } label: {
                ZStack(alignment: .bottomLeading) {
                    Map(
                        initialPosition: .region(location.region),
                        interactionModes: []
                    ) {
                        Marker(location.label, coordinate: location.coordinate)
                    }
                    .allowsHitTesting(false)

                    if !location.label.isEmpty {
                        Text(location.label)
                            .font(.caption.weight(.medium))
                            .lineLimit(2)
                            .foregroundStyle(.primary)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 7)
                            .background(.regularMaterial)
                            .clipShape(RoundedRectangle(cornerRadius: 7))
                            .padding(8)
                    }
                }
                .frame(width: 260, height: 180)
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .overlay {
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(.separator.opacity(0.5), lineWidth: 1)
                }
            }
            .buttonStyle(.plain)
            .help("Open in Google Maps")
            .accessibilityLabel(
                location.label.isEmpty
                    ? "Open shared location in Google Maps"
                    : "Open \(location.label) in Google Maps"
            )
        } else {
            UnsupportedSpecialMessageView(category: "LOCATION")
        }
    }
}

private struct SpecialSecretMessageView: View {
    private let url = URL(string: "https://mixin.one/pages/1000007")!

    var body: some View {
        Button {
            NSWorkspace.shared.open(url)
        } label: {
            Label(
                "Messages in this conversation are end-to-end encrypted. Tap for more info.",
                systemImage: "lock.fill"
            )
            .font(.callout)
            .foregroundStyle(Color.black.opacity(0.82))
            .multilineTextAlignment(.center)
            .padding(10)
            .background(
                Color(red: 1, green: 247 / 255, blue: 173 / 255),
                in: RoundedRectangle(cornerRadius: 10)
            )
        }
        .buttonStyle(.plain)
        .help("Learn about end-to-end encryption")
    }
}

private struct SpecialPinMessageView: View {
    let senderName: String
    let content: String
    let mentionNames: [String: String]

    var body: some View {
        Text(summary)
            .font(.callout)
            .foregroundStyle(Color.black.opacity(0.82))
            .lineLimit(1)
            .truncationMode(.tail)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .frame(maxWidth: 400)
            .background(
                Color(red: 202 / 255, green: 234 / 255, blue: 201 / 255),
                in: RoundedRectangle(cornerRadius: 10)
            )
            .accessibilityLabel(summary)
    }

    private var summary: String {
        let payload = PinnedMessagePayload.parse(content)
        var preview = payload?.preview ?? "a message"
        for (userID, name) in mentionNames {
            preview = preview.replacingOccurrences(of: "@\(userID)", with: "@\(name)")
        }
        return "\(senderName) pinned \(preview)"
    }
}

private struct SpecialStrangerMessageView: View {
    let message: SwiftMessageItem
    let onAction: (String) async -> Bool

    @State private var resolved = false
    @State private var pendingAction: String?

    var body: some View {
        if !resolved, !sourceRelationshipResolved {
            VStack(spacing: 10) {
                Text(
                    message.senderIsBot
                        ? "Tap the button to interact with the bot"
                        : "This sender is not in your contacts"
                )
                .multilineTextAlignment(.center)

                HStack(spacing: 16) {
                    actionButton(
                        message.senderIsBot ? "Open Homepage" : "Block",
                        action: message.senderIsBot ? "open_home" : "block"
                    )
                    actionButton(
                        message.senderIsBot ? "Say Hi" : "Add Contact",
                        action: message.senderIsBot ? "say_hi" : "add_contact"
                    )
                }
            }
            .frame(minWidth: 240)
        }
    }

    private var sourceRelationshipResolved: Bool {
        ["FRIEND", "BLOCKED"].contains(message.senderRelationship.uppercased())
    }

    private func actionButton(_ title: String, action: String) -> some View {
        Button {
            pendingAction = action
            Task {
                let succeeded = await onAction(action)
                if succeeded, action == "block" || action == "add_contact" {
                    resolved = true
                }
                pendingAction = nil
            }
        } label: {
            HStack(spacing: 6) {
                if pendingAction == action {
                    ProgressView()
                        .controlSize(.small)
                }
                Text(title)
            }
            .frame(minWidth: 94)
        }
        .disabled(pendingAction != nil)
    }
}

private struct UnsupportedSpecialMessageView: View {
    let category: String

    var body: some View {
        Label(
            "This type of message is not supported. Update Mixin to the latest version.",
            systemImage: "exclamationmark.bubble"
        )
        .foregroundStyle(.secondary)
        .help(category)
    }
}

private struct SharedLocation {
    let latitude: Double
    let longitude: Double
    let name: String
    let address: String

    var coordinate: CLLocationCoordinate2D {
        CLLocationCoordinate2D(latitude: latitude, longitude: longitude)
    }

    var region: MKCoordinateRegion {
        MKCoordinateRegion(
            center: coordinate,
            span: MKCoordinateSpan(latitudeDelta: 0.012, longitudeDelta: 0.012)
        )
    }

    var label: String {
        address.isEmpty ? name : address
    }

    var googleMapsURL: URL {
        if address.isEmpty {
            return URL(
                string: "https://www.google.com/maps/place/@\(latitude),\(longitude),17z?hl=zh-CN"
            )!
        }
        var allowed = CharacterSet.urlPathAllowed
        allowed.remove(charactersIn: "/")
        let encodedAddress = address.addingPercentEncoding(
            withAllowedCharacters: allowed
        ) ?? address
        return URL(
            string: "https://www.google.com/maps/search/\(encodedAddress)/@\(latitude),\(longitude),17z?hl=zh-CN"
        )!
    }

    static func parse(_ raw: String) -> SharedLocation? {
        guard let data = raw.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let latitude = number(json["latitude"]),
              let longitude = number(json["longitude"]),
              (-90 ... 90).contains(latitude),
              (-180 ... 180).contains(longitude)
        else {
            return nil
        }
        return SharedLocation(
            latitude: latitude,
            longitude: longitude,
            name: json["name"] as? String ?? "",
            address: json["address"] as? String ?? ""
        )
    }

    private static func number(_ value: Any?) -> Double? {
        switch value {
        case let value as NSNumber:
            value.doubleValue
        case let value as String:
            Double(value)
        default:
            nil
        }
    }
}

private struct PinnedMessagePayload: Decodable {
    let messageID: String?
    let category: String
    let content: String

    enum CodingKeys: String, CodingKey {
        case messageID = "message_id"
        case category
        case content
    }

    var preview: String {
        let category = category.uppercased()
        if category.hasSuffix("_TEXT") {
            return content.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        let value: String
        if category.hasSuffix("_POST") {
            value = content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? "Post"
                : content.markdownPreview
        } else if category == "APP_BUTTON_GROUP" {
            value = AppMessagePayload.actionLabels(content)
        } else if category == "APP_CARD" {
            value = "[\(AppMessagePayload.cardTitle(content) ?? "Card")]"
        } else if category.hasPrefix("WEBRTC") || category.hasPrefix("KRAKEN") {
            value = "[Voice call]"
        } else if category == "MESSAGE_RECALL" {
            value = "[This message was deleted]"
        } else {
            value = categoryPreview(category)
        }
        return value.isEmpty ? value : ": \(value)"
    }

    static func parse(_ raw: String) -> PinnedMessagePayload? {
        guard let data = raw.data(using: .utf8) else {
            return nil
        }
        return try? JSONDecoder().decode(PinnedMessagePayload.self, from: data)
    }

    private func categoryPreview(_ category: String) -> String {
        switch category {
        case let value where value.hasSuffix("_IMAGE"): "[Image]"
        case let value where value.hasSuffix("_VIDEO"): "[Video]"
        case let value where value.hasSuffix("_LIVE"): "[Live]"
        case let value where value.hasSuffix("_AUDIO"): "[Audio]"
        case let value where value.hasSuffix("_DATA"): "[File]"
        case let value where value.hasSuffix("_STICKER"): "[Sticker]"
        case let value where value.hasSuffix("_CONTACT"): "[Contact]"
        case let value where value.hasSuffix("_LOCATION"): "[Location]"
        case let value where value.hasSuffix("_TRANSCRIPT"): "[Transcript]"
        case "SYSTEM_ACCOUNT_SNAPSHOT", "SYSTEM_SAFE_SNAPSHOT": "[Transfer]"
        case "SYSTEM_SAFE_INSCRIPTION": "[Collectible]"
        default: "Message not supported"
        }
    }
}

private enum AppMessagePayload {
    static func actionLabels(_ raw: String) -> String {
        guard let data = raw.data(using: .utf8),
              let values = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]]
        else {
            return ""
        }
        return values
            .compactMap { $0["label"] as? String }
            .map { "[\($0)]" }
            .joined()
    }

    static func cardTitle(_ raw: String) -> String? {
        guard let data = raw.data(using: .utf8),
              let value = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return nil
        }
        return value["title"] as? String
    }
}

private extension String {
    var markdownPreview: String {
        let firstLines = split(separator: "\n", omittingEmptySubsequences: false)
            .prefix(10)
            .joined(separator: " ")
            .prefix(1024)
        return String(firstLines)
            .replacingOccurrences(
                of: #"(!?\[)([^\]]+)(\]\([^)]+\))"#,
                with: "$2",
                options: .regularExpression
            )
            .replacingOccurrences(
                of: #"[*_`>#~-]+"#,
                with: "",
                options: .regularExpression
            )
            .replacingOccurrences(
                of: #"\s+"#,
                with: " ",
                options: .regularExpression
            )
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
