import Foundation

enum AppLogger {
    static func verbose(_ message: @autoclosure () -> String) {
        write(message(), level: .verbose)
    }

    static func debug(_ message: @autoclosure () -> String) {
        write(message(), level: .debug)
    }

    static func info(_ message: @autoclosure () -> String) {
        write(message(), level: .info)
    }

    static func warning(_ message: @autoclosure () -> String) {
        write(message(), level: .warning)
    }

    static func error(
        _ message: String,
        error: Error? = nil,
        stackTrace: String? = nil
    ) {
        var detail = message
        if let error {
            detail += " (\(String(reflecting: type(of: error)))(\(errorDescription(error))))"
        }
        detail += ":\n\(stackTrace ?? Thread.callStackSymbols.joined(separator: "\n"))"
        write(detail, level: .error)
    }

    static func wtf(_ message: @autoclosure () -> String) {
        write(message(), level: .wtf)
    }

    private static func write(_ message: String, level: Level) {
        logSwift(level: level.rawValue, message: message)
    }

    private static func errorDescription(_ error: Error) -> String {
        let description = String(reflecting: error)
        let lines = description.split(
            separator: "\n",
            omittingEmptySubsequences: false
        )
        guard lines.count > 10 else {
            return description
        }
        return lines.prefix(9).joined(separator: "\n")
            + "\n... \(lines.count - 9) lines omitted"
    }

    private enum Level: String {
        case verbose
        case debug
        case info
        case warning
        case error
        case wtf
    }
}
