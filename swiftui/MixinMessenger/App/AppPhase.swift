import Foundation

enum AppPhase {
    case launching
    case signedOut
    case signedIn(AccountSession)
    case recovery(AppRecovery)
}

struct AppRecovery {
    enum Kind {
        case database(DatabaseOpenFailure)
        case savedLogin
        case startup
    }

    let kind: Kind
    let message: String
    let diagnostic: String
}

struct DatabaseOpenFailure {
    static let marker = "mixin_database_open_error:"

    let resultCode: Int
    let explanation: String

    static func parse(_ value: String) -> DatabaseOpenFailure? {
        guard let markerRange = value.range(of: marker) else {
            return nil
        }
        let payload = value[markerRange.upperBound...]
        guard let separator = payload.firstIndex(of: ":"),
              let resultCode = Int(payload[..<separator])
        else {
            return nil
        }
        return DatabaseOpenFailure(
            resultCode: resultCode,
            explanation: String(payload[payload.index(after: separator)...])
        )
    }
}
