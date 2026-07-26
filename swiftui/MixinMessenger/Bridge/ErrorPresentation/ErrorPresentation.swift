import Foundation
import Observation
import OSLog
import SwiftUI

enum MixinErrorPresenter {
    private static let logger = Logger(
        subsystem: Bundle.main.bundleIdentifier ?? "one.mixin.messenger.desktop",
        category: "ErrorPresentation"
    )

    static func message(
        for error: Error,
        fileID: StaticString = #fileID,
        function: StaticString = #function,
        line: UInt = #line
    ) -> String {
        let location = "\(fileID):\(line) \(function)"
        let detail = String(reflecting: error)
        logger.error(
            "\(location, privacy: .public): \(detail, privacy: .public)"
        )
        return switch error {
        case SwiftClientError.Unauthorized:
            MixinLocalizations.sessionExpired
        case SwiftClientError.NotFound:
            MixinLocalizations.notFound
        case SwiftClientError.Cancelled:
            MixinLocalizations.operationCancelled
        case let SwiftClientError.InvalidArgument(message):
            message.isEmpty ? MixinLocalizations.invalidRequest : displayMessage(from: message)
        case let SwiftClientError.Internal(message):
            displayMessage(from: message)
        default:
            displayMessage(from: error.localizedDescription)
        }
    }

    static func displayMessage(from rawMessage: String) -> String {
        let message = rawMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !message.isEmpty else {
            return MixinLocalizations.unknownError
        }
        if message.contains("attachment_upload_failed:") {
            return MixinLocalizations.attachmentUploadFailed
        }
        if let apiError = MixinAPIError.parse(message) {
            return apiError.displayMessage
        }
        return message
    }
}

private struct MixinAPIError {
    let code: Int
    let description: String

    private static let expression = try? NSRegularExpression(
        pattern: #"(?:server error: )?Error: status: -?\d+, code: (-?\d+), description: ([\s\S]*)"#,
        options: []
    )

    static func parse(_ value: String) -> MixinAPIError? {
        guard let expression else {
            return nil
        }
        let range = NSRange(value.startIndex..., in: value)
        guard let match = expression.firstMatch(in: value, range: range),
              let codeRange = Range(match.range(at: 1), in: value),
              let code = Int(value[codeRange]),
              let descriptionRange = Range(match.range(at: 2), in: value)
        else {
            return nil
        }
        return MixinAPIError(
            code: code,
            description: String(value[descriptionRange])
                .trimmingCharacters(in: .whitespacesAndNewlines)
        )
    }

    var displayMessage: String {
        switch code {
        case 10002:
            MixinLocalizations.invalidRequest
        case 403:
            MixinLocalizations.accessLimited
        case 404:
            MixinLocalizations.notFound
        case 429:
            MixinLocalizations.tooManyRequests
        case 500, 30103:
            MixinLocalizations.serverUnavailable
        default:
            description.isEmpty ? "\(MixinLocalizations.unknownError) (\(code))" : description
        }
    }
}

enum MixinNoticeKind: Equatable {
    case information
    case loading
    case success
    case failure

    var systemImage: String {
        switch self {
        case .information: "info.circle.fill"
        case .loading: "progress.indicator"
        case .success: "checkmark.circle.fill"
        case .failure: "xmark.circle.fill"
        }
    }
}

struct MixinNotice: Identifiable {
    let id = UUID()
    let kind: MixinNoticeKind
    let message: String
}

@MainActor
@Observable
final class MixinNoticeCenter {
    private(set) var notice: MixinNotice?
    @ObservationIgnored private var dismissTask: Task<Void, Never>?

    func show(
        _ message: String,
        kind: MixinNoticeKind = .information,
        dismissAfter delay: Duration? = .seconds(2)
    ) {
        dismissTask?.cancel()
        notice = MixinNotice(kind: kind, message: message)
        guard let delay else {
            return
        }
        dismissTask = Task {
            try? await Task.sleep(for: delay)
            guard !Task.isCancelled else {
                return
            }
            notice = nil
        }
    }

    func show(error: Error) {
        show(MixinErrorPresenter.message(for: error), kind: .failure)
    }

    func dismiss() {
        dismissTask?.cancel()
        dismissTask = nil
        notice = nil
    }
}

struct MixinNoticeOverlay: ViewModifier {
    @Environment(MixinNoticeCenter.self) private var noticeCenter

    func body(content: Content) -> some View {
        content.overlay(alignment: .top) {
            if let notice = noticeCenter.notice {
                MixinNoticeView(notice: notice)
                    .padding(.top, 16)
                    .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.16), value: noticeCenter.notice?.id)
    }
}

private struct MixinNoticeView: View {
    let notice: MixinNotice

    var body: some View {
        HStack(spacing: 10) {
            if notice.kind == .loading {
                ProgressView()
                    .controlSize(.small)
            } else {
                Image(systemName: notice.kind.systemImage)
                    .foregroundStyle(iconColor)
            }
            Text(notice.message)
                .font(.callout)
                .lineLimit(3)
                .textSelection(.enabled)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 11)
        .frame(minWidth: 130)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
        .shadow(color: .black.opacity(0.18), radius: 16, y: 6)
        .accessibilityElement(children: .combine)
    }

    private var iconColor: Color {
        switch notice.kind {
        case .information, .loading: .accentColor
        case .success: .green
        case .failure: .red
        }
    }
}

extension View {
    func mixinNoticeOverlay() -> some View {
        modifier(MixinNoticeOverlay())
    }
}
