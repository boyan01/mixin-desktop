import Foundation
import SwiftUI

struct ProfileIdentityBadge: View {
    let isVerified: Bool
    let isBot: Bool
    let membership: String?

    var body: some View {
        if let plan = activeMembershipPlan {
            Image(systemName: plan == "prosperity" ? "crown.fill" : "star.circle.fill")
                .foregroundStyle(color(for: plan))
                .help("Mixin \(plan.capitalized)")
                .accessibilityLabel("Mixin \(plan.capitalized) member")
        } else if isVerified {
            Image(systemName: "checkmark.seal.fill")
                .foregroundStyle(.blue)
                .help("Verified")
                .accessibilityLabel("Verified account")
        } else if isBot {
            Image(systemName: "bolt.circle.fill")
                .foregroundStyle(.secondary)
                .help("Bot")
                .accessibilityLabel("Bot account")
        }
    }

    private var activeMembershipPlan: String? {
        guard let membership,
              let data = membership.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data)
                as? [String: Any],
              let plan = object["plan"] as? String,
              ["advance", "elite", "prosperity"].contains(plan),
              let expiration = object["expired_at"] as? String,
              let expirationDate = Self.fractionalDateFormatter.date(from: expiration)
                ?? Self.dateFormatter.date(from: expiration),
              expirationDate > Date()
        else {
            return nil
        }
        return plan
    }

    private func color(for plan: String) -> Color {
        switch plan {
        case "advance":
            .blue
        case "elite":
            .purple
        default:
            .orange
        }
    }

    private static let dateFormatter: ISO8601DateFormatter = {
        ISO8601DateFormatter()
    }()

    private static let fractionalDateFormatter: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()
}
