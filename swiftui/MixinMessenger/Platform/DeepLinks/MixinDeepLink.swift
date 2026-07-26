import Foundation

enum MixinLinkKind: String {
    case codes
    case pay
    case users
    case transfer
    case device
    case send
    case address
    case withdrawal
    case apps
    case snapshots
    case multisigs
    case swap
    case markets
    case membership

    var displayName: String {
        switch self {
        case .codes:
            "Invitation code"
        case .pay:
            "Payment"
        case .users:
            "User"
        case .transfer:
            "Transfer"
        case .device:
            "Device"
        case .send:
            "Send"
        case .address:
            "Address"
        case .withdrawal:
            "Withdrawal"
        case .apps:
            "App"
        case .snapshots:
            "Snapshot"
        case .multisigs:
            "Multisig"
        case .swap:
            "Swap"
        case .markets:
            "Market"
        case .membership:
            "Membership"
        }
    }
}

enum ProtocolSendCategory: String, CaseIterable, Identifiable {
    case text
    case image
    case sticker
    case contact
    case post
    case appCard = "app_card"

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .text: "Text"
        case .image: "Image"
        case .sticker: "Sticker"
        case .contact: "Contact"
        case .post: "Post"
        case .appCard: "Card"
        }
    }
}

struct ProtocolSendPayload: Hashable {
    let category: ProtocolSendCategory
    let content: String

    init?(category: String?, encodedData: String?) {
        guard let category,
              let category = ProtocolSendCategory(rawValue: category.lowercased()),
              let encodedData,
              !encodedData.isEmpty,
              let decoded = Self.decodeBase64(encodedData),
              let decodedText = String(data: decoded, encoding: .utf8)
        else {
            return nil
        }

        let content: String
        switch category {
        case .image:
            guard let value = Self.jsonString("url", from: decoded) else {
                return nil
            }
            content = value
        case .contact:
            guard let value = Self.jsonString("user_id", from: decoded),
                  UUID(uuidString: value) != nil
            else {
                return nil
            }
            content = value
        case .appCard:
            guard let object = try? JSONSerialization.jsonObject(with: decoded),
                  object is [String: Any],
                  let normalized = try? JSONSerialization.data(withJSONObject: object),
                  let value = String(data: normalized, encoding: .utf8)
            else {
                return nil
            }
            content = value
        case .sticker:
            guard UUID(uuidString: decodedText) != nil else {
                return nil
            }
            content = decodedText
        case .text, .post:
            content = decodedText
        }

        guard !content.isEmpty else {
            return nil
        }
        self.category = category
        self.content = content
    }

    private static func decodeBase64(_ value: String) -> Data? {
        var normalized = value
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let remainder = normalized.count % 4
        if remainder != 0 {
            normalized.append(String(repeating: "=", count: 4 - remainder))
        }
        return Data(base64Encoded: normalized, options: .ignoreUnknownCharacters)
    }

    private static func jsonString(_ key: String, from data: Data) -> String? {
        guard let object = try? JSONSerialization.jsonObject(with: data),
              let dictionary = object as? [String: Any],
              let value = dictionary[key] as? String,
              !value.isEmpty
        else {
            return nil
        }
        return value
    }
}

struct ProtocolSendRequest: Identifiable, Hashable {
    let id = UUID()
    let payload: ProtocolSendPayload
    let conversationID: String?
    let userID: String?
}

enum MixinDeepLink {
    case conversation(id: String, start: String?)
    case user(identityNumber: String)
    case app(id: String, open: Bool, source: URL)
    case code(value: String, source: URL)
    case snapshot(traceID: String)
    case send(ProtocolSendRequest)
    case external
    case unsupported(MixinLinkKind)
    case invalid

    init(url: URL) {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false) else {
            self = .invalid
            return
        }

        let scheme = components.scheme?.lowercased()
        let host = components.host?.lowercased()
        let path = components.path
            .split(separator: "/")
            .map(String.init)

        if scheme != "mixin", host != "mixin.one", host != "www.mixin.one" {
            self = .external
            return
        }

        let type: String?
        let value: String?
        if scheme == "mixin" {
            type = host
            value = path.count == 1 ? path[0] : nil
        } else {
            type = path.first
            value = path.count > 1 ? path[1] : nil
        }

        if type == "conversations", let value, !value.isEmpty {
            let start = components.queryItems?
                .first(where: { $0.name == "start" })?
                .value?
                .trimmingCharacters(in: .whitespacesAndNewlines)
            self = .conversation(
                id: value,
                start: start?.isEmpty == false ? start : nil
            )
            return
        }

        if type == "codes", let value, !value.isEmpty {
            self = .code(value: value, source: url)
            return
        }

        if type == "users", let value, !value.isEmpty {
            self = .user(identityNumber: value)
            return
        }

        if type == "apps", let value, !value.isEmpty {
            self = .app(
                id: value,
                open: components.queryItems?.contains {
                    $0.name == "action" && $0.value == "open"
                } == true,
                source: url
            )
            return
        }

        if scheme == "mixin",
           type == "snapshots",
           let traceID = components.queryItems?
               .first(where: { $0.name == "trace" })?
               .value?
               .trimmingCharacters(in: .whitespacesAndNewlines),
           !traceID.isEmpty
        {
            self = .snapshot(traceID: traceID)
            return
        }

        if type == "send" {
            let queryItems = components.queryItems ?? []
            func query(_ name: String) -> String? {
                queryItems.first(where: { $0.name == name })?.value
            }
            guard let payload = ProtocolSendPayload(
                category: query("category"),
                encodedData: query("data")
            ) else {
                self = .invalid
                return
            }
            let conversationID = query("conversation")?.trimmingCharacters(
                in: .whitespacesAndNewlines
            )
            let userID = query("user")?.trimmingCharacters(
                in: .whitespacesAndNewlines
            )
            if let userID, !userID.isEmpty, UUID(uuidString: userID) == nil {
                self = .invalid
                return
            }
            self = .send(
                ProtocolSendRequest(
                    payload: payload,
                    conversationID: conversationID?.isEmpty == false
                        ? conversationID
                        : nil,
                    userID: userID?.isEmpty == false ? userID : nil
                )
            )
            return
        }

        guard let type, let kind = MixinLinkKind(rawValue: type) else {
            self = .invalid
            return
        }
        self = .unsupported(kind)
    }
}
