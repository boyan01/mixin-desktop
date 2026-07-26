import Foundation

enum MixinLocalizations {
    static let supportedLocaleIdentifiers = [
        "en",
        "es",
        "id",
        "ja",
        "ms",
        "ru",
        "zh-Hans",
        "zh-Hant-HK",
        "zh-Hant-TW",
    ]

    static var unableToCompleteOperation: String {
        String(localized: "Unable to complete the operation")
    }

    static var unknownError: String {
        String(localized: "Unknown error")
    }

    static var operationCancelled: String {
        String(localized: "Operation cancelled")
    }

    static var sessionExpired: String {
        String(localized: "Your session has expired. Please sign in again.")
    }

    static var notFound: String {
        String(localized: "Not found")
    }

    static var attachmentUploadFailed: String {
        String(localized: "Failed to upload message attachment")
    }

    static var invalidRequest: String {
        String(localized: "The request data is invalid.")
    }

    static var accessLimited: String {
        String(localized: "Access is limited.")
    }

    static var tooManyRequests: String {
        String(localized: "Too many requests. Please try again later.")
    }

    static var serverUnavailable: String {
        String(localized: "The server is temporarily unavailable.")
    }
}
