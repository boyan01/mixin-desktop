import Foundation
import Observation

enum DeviceTransferDirection: Equatable {
    case restore
    case backup

    var title: String {
        switch self {
        case .restore:
            "sync from other device"
        case .backup:
            "sync to other device"
        }
    }

    var symbolName: String {
        switch self {
        case .restore:
            "arrow.down.to.line.compact"
        case .backup:
            "arrow.up.to.line.compact"
        }
    }
}

enum DeviceTransferSetupPage: Equatable {
    case choices
    case explanation(DeviceTransferDirection)
    case waiting(DeviceTransferDirection)
}

enum DeviceTransferSheetMode: Equatable {
    case setup
    case progress(DeviceTransferDirection)
}

struct DeviceTransferAlert: Identifiable {
    enum Kind {
        case approval(DeviceTransferDirection)
        case notice
    }

    let id = UUID()
    let kind: Kind
    let title: String
    let message: String
}

@MainActor
@Observable
final class DeviceTransferController {
    private let account: SwiftAccountHandle
    private var subscription: SwiftDeviceTransferSubscription?
    private var eventTask: Task<Void, Never>?
    private var activity: NSObjectProtocol?

    private(set) var sheetMode: DeviceTransferSheetMode?
    private(set) var setupPage: DeviceTransferSetupPage = .choices
    private(set) var progress = 0.0
    private(set) var bytesPerSecond = 0.0
    private(set) var alert: DeviceTransferAlert?
    private(set) var commandInFlight = false

    init(account: SwiftAccountHandle) {
        self.account = account
    }

    func start() {
        stopSubscription()
        let subscription = account.deviceTransferEvents()
        self.subscription = subscription
        eventTask = Task { [weak self] in
            while !Task.isCancelled,
                  let event = await subscription.next()
            {
                guard let self else {
                    return
                }
                receive(event)
            }
        }
    }

    func stop() {
        stopSubscription()
        endActivity()
        sheetMode = nil
        alert = nil
    }

    func openSetup() {
        guard sheetMode == nil else {
            return
        }
        setupPage = .choices
        sheetMode = .setup
    }

    func showExplanation(_ direction: DeviceTransferDirection) {
        setupPage = .explanation(direction)
    }

    func goBack() {
        switch setupPage {
        case .choices:
            dismissSetup()
        case .explanation:
            setupPage = .choices
        case let .waiting(direction):
            setupPage = .explanation(direction)
            sendCancellation(for: direction)
        }
    }

    func begin(_ direction: DeviceTransferDirection) {
        guard !commandInFlight else {
            return
        }
        setupPage = .waiting(direction)
        commandInFlight = true
        Task {
            defer {
                commandInFlight = false
            }
            do {
                try await account.deviceTransferCommand(
                    command: direction == .restore ? .pullToRemote : .pushToRemote
                )
            } catch {
                setupPage = .explanation(direction)
                showFailure(error, title: "Unable to Start Transfer")
            }
        }
    }

    func dismissPresentedSheet() {
        switch sheetMode {
        case .setup:
            dismissSetup()
        case let .progress(direction):
            cancelTransfer(direction)
        case nil:
            break
        }
    }

    func dismissSetup() {
        if case let .waiting(direction) = setupPage {
            sendCancellation(for: direction)
        }
        sheetMode = nil
        setupPage = .choices
    }

    func cancelTransfer(_ direction: DeviceTransferDirection) {
        sheetMode = nil
        endActivity()
        sendCancellation(for: direction)
    }

    func respondToApproval(approved: Bool) {
        guard let alert,
              case let .approval(direction) = alert.kind
        else {
            return
        }
        self.alert = nil
        Task {
            do {
                try await account.deviceTransferCommand(
                    command: approvalCommand(for: direction, approved: approved)
                )
            } catch {
                showFailure(error, title: "Unable to Respond to Transfer")
            }
        }
    }

    func dismissAlert() {
        alert = nil
    }

    static func formatNetworkSpeed(_ bytesPerSecond: Double) -> String {
        let kilobytes = max(0, bytesPerSecond) / 1024
        if kilobytes < 1024 {
            return String(format: "%.2f KB/s", kilobytes)
        }
        return String(format: "%.2f MB/s", kilobytes / 1024)
    }

    private func receive(_ event: DeviceTransferEventItem) {
        switch event {
        case .restoreConnected:
            presentProgress(.restore)
        case .backupServerCreated:
            presentProgress(.backup)
        case .restoreStart, .backupStart:
            if sheetMode == .setup {
                sheetMode = nil
                setupPage = .choices
            }
        case .restoreSucceed, .backupSucceed:
            finishTransfer(
                title: "",
                message: "Transfer completed"
            )
        case .restoreFailed, .backupFailed:
            finishTransfer(
                title: "",
                message: "Transfer failed"
            )
        case let .restoreProgress(value):
            updateProgress(value, for: .restore)
        case let .backupProgress(value):
            updateProgress(value, for: .backup)
        case let .restoreNetworkSpeed(bytesPerSecond):
            updateNetworkSpeed(bytesPerSecond, for: .restore)
        case let .backupNetworkSpeed(bytesPerSecond):
            updateNetworkSpeed(bytesPerSecond, for: .backup)
        case .backupRequestReceived:
            alert = DeviceTransferAlert(
                kind: .approval(.restore),
                title: "",
                message: "Are you sure to sync the chat history from the phone?"
            )
        case .restoreRequestReceived:
            alert = DeviceTransferAlert(
                kind: .approval(.backup),
                title: "",
                message: "Are you sure to sync the chat history to the phone?"
            )
        case let .connectionFailed(reason):
            sheetMode = nil
            endActivity()
            alert = DeviceTransferAlert(
                kind: .notice,
                title: "",
                message: reason == .versionNotMatched
                    ? "Protocol version does not match, transfer failed. Please upgrade the application first."
                    : "Transfer failed"
            )
        }
    }

    private func presentProgress(_ direction: DeviceTransferDirection) {
        if sheetMode != .progress(direction) {
            progress = 0
            bytesPerSecond = 0
        }
        setupPage = .choices
        sheetMode = .progress(direction)
        beginActivity()
    }

    private func updateProgress(_ value: Double, for direction: DeviceTransferDirection) {
        presentProgressIfNeeded(direction)
        progress = min(max(value, 0), 100)
    }

    private func updateNetworkSpeed(
        _ value: Double,
        for direction: DeviceTransferDirection
    ) {
        presentProgressIfNeeded(direction)
        bytesPerSecond = max(value, 0)
    }

    private func presentProgressIfNeeded(_ direction: DeviceTransferDirection) {
        guard sheetMode != .progress(direction) else {
            return
        }
        presentProgress(direction)
    }

    private func finishTransfer(title: String, message: String) {
        sheetMode = nil
        setupPage = .choices
        endActivity()
        alert = DeviceTransferAlert(
            kind: .notice,
            title: title,
            message: message
        )
    }

    private func approvalCommand(
        for direction: DeviceTransferDirection,
        approved: Bool
    ) -> DeviceTransferCommand {
        switch (direction, approved) {
        case (.restore, true):
            .confirmRestore
        case (.restore, false):
            .cancelRestoreRequest
        case (.backup, true):
            .confirmBackup
        case (.backup, false):
            .cancelBackupRequest
        }
    }

    private func sendCancellation(for direction: DeviceTransferDirection) {
        Task {
            do {
                try await account.deviceTransferCommand(
                    command: direction == .restore ? .cancelRestore : .cancelBackup
                )
            } catch {
                showFailure(error, title: "Unable to Cancel Transfer")
            }
        }
    }

    private func showFailure(_ error: Error, title: String) {
        alert = DeviceTransferAlert(
            kind: .notice,
            title: title,
            message: MixinErrorPresenter.message(for: error)
        )
    }

    private func beginActivity() {
        guard activity == nil else {
            return
        }
        activity = ProcessInfo.processInfo.beginActivity(
            options: [.idleSystemSleepDisabled, .suddenTerminationDisabled],
            reason: "Transferring Mixin chat history"
        )
    }

    private func endActivity() {
        guard let activity else {
            return
        }
        ProcessInfo.processInfo.endActivity(activity)
        self.activity = nil
    }

    private func stopSubscription() {
        subscription?.cancel()
        subscription = nil
        eventTask?.cancel()
        eventTask = nil
    }
}
