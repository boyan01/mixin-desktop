import AppKit
import Foundation

@MainActor
final class MessageActionHandler {
    private let account: SwiftAccountHandle
    private let conversationID: String
    private let currency: String

    init(
        account: SwiftAccountHandle,
        conversationID: String,
        currency: String
    ) {
        self.account = account
        self.conversationID = conversationID
        self.currency = currency
    }

    func open(_ rawAction: String, title: String = "") async throws {
        let action = rawAction.trimmingCharacters(in: .whitespacesAndNewlines)
        if action.hasPrefix("input:") {
            let content = String(action.dropFirst(6))
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard !content.isEmpty else {
                return
            }
            _ = try await account.sendText(
                conversationId: conversationID,
                content: content,
                quoteMessageId: nil,
                silent: false
            )
            return
        }
        guard let url = URL(string: action), url.scheme != nil else {
            return
        }
        if let mixinURL = url.mixinApplicationURL {
            NSWorkspace.shared.open(mixinURL)
        } else {
            BotWebViewWindow.open(
                url: url,
                title: title,
                conversationID: conversationID,
                currency: currency
            )
        }
    }

    func openBotHome(appID: String, title: String = "") async throws {
        guard
            let rawURL = try await account.botHomeUri(appId: appID),
            let url = URL(string: rawURL)
        else {
            return
        }
        BotWebViewWindow.open(
            url: url,
            title: title,
            conversationID: conversationID,
            currency: currency
        )
    }
}

private extension URL {
    var mixinApplicationURL: URL? {
        if scheme?.lowercased() == "mixin" {
            return self
        }
        guard
            ["mixin.one", "www.mixin.one"].contains(host?.lowercased() ?? ""),
            var components = URLComponents(
                url: self,
                resolvingAgainstBaseURL: false
            )
        else {
            return nil
        }
        let segments = components.path
            .split(separator: "/")
            .map(String.init)
        guard let action = segments.first else {
            return nil
        }
        components.scheme = "mixin"
        components.host = action
        components.path = segments.dropFirst().isEmpty
            ? ""
            : "/" + segments.dropFirst().joined(separator: "/")
        return components.url
    }
}
