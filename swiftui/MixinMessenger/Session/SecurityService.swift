import Foundation
import LocalAuthentication
import Observation

@MainActor
@Observable
final class SecurityService {
    private enum Setting {
        static let passcode = "passcode"
        static let biometric = "biometric"
        static let lockDuration = "lockDuration"
    }

    private let desktop: SwiftDesktopHandle
    private let prefix: String
    private var passcode: String?
    private var lockTask: Task<Void, Never>?

    private(set) var isInitialized = false
    private(set) var loadError: String?
    private(set) var biometricEnabled = false
    private(set) var autoLockMinutes = 1
    private(set) var isLocked = false

    var hasPasscode: Bool {
        passcode != nil
    }

    init(desktop: SwiftDesktopHandle, accountID: String) {
        self.desktop = desktop
        prefix = "security.\(accountID)."
    }

    func load() async {
        guard !isInitialized else {
            return
        }
        loadError = nil
        do {
            async let storedPasscode = desktop.setting(key: key(Setting.passcode))
            async let storedBiometric = desktop.setting(key: key(Setting.biometric))
            async let storedDuration = desktop.setting(key: key(Setting.lockDuration))

            let values = try await (
                storedPasscode,
                storedBiometric,
                storedDuration
            )
            try Task.checkCancellation()
            let decodedPasscode: String? = try decode(values.0)
            if let decodedPasscode, !Self.isValidPasscode(decodedPasscode) {
                throw SecuritySettingsError.corruptedPasscode
            }
            passcode = decodedPasscode
            biometricEnabled = try decode(values.1) ?? false
            autoLockMinutes = try decode(values.2) ?? 1
            guard Self.supportedAutoLockMinutes.contains(autoLockMinutes) else {
                throw SecuritySettingsError.corruptedAutoLockDuration
            }
            isLocked = passcode != nil
            isInitialized = true
        } catch is CancellationError {
            return
        } catch {
            loadError = MixinErrorPresenter.message(for: error)
        }
    }

    func retryLoad() async {
        isInitialized = false
        await load()
    }

    func setPasscode(_ value: String?) async throws {
        if let value, !Self.isValidPasscode(value) {
            throw SecuritySettingsError.invalidPasscode
        }

        if let value {
            try await desktop.setSetting(
                key: key(Setting.passcode),
                value: try encode(value)
            )
            passcode = value
            return
        }

        try await desktop.setSetting(key: key(Setting.passcode), value: nil)
        try await desktop.setSetting(
            key: key(Setting.biometric),
            value: try encode(false)
        )
        try await desktop.setSetting(key: key(Setting.lockDuration), value: nil)
        passcode = nil
        biometricEnabled = false
        autoLockMinutes = 1
        isLocked = false
        cancelPendingLock()
    }

    func setBiometricEnabled(_ enabled: Bool) async throws {
        if enabled, !canUseBiometrics() {
            throw SecuritySettingsError.biometricUnavailable
        }
        try await desktop.setSetting(
            key: key(Setting.biometric),
            value: try encode(enabled)
        )
        biometricEnabled = enabled
    }

    func setAutoLockMinutes(_ minutes: Int) async throws {
        guard Self.supportedAutoLockMinutes.contains(minutes) else {
            throw SecuritySettingsError.invalidAutoLockDuration
        }
        try await desktop.setSetting(
            key: key(Setting.lockDuration),
            value: try encode(minutes)
        )
        autoLockMinutes = minutes
    }

    func lockNow() {
        guard hasPasscode else {
            return
        }
        cancelPendingLock()
        isLocked = true
    }

    func unlock(passcode candidate: String) -> Bool {
        guard candidate == passcode else {
            return false
        }
        isLocked = false
        return true
    }

    func unlockWithBiometrics() async -> Bool {
        guard biometricEnabled else {
            return false
        }
        let context = LAContext()
        context.localizedCancelTitle = "Use Passcode"
        var authorizationError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            error: &authorizationError
        ) else {
            return false
        }
        do {
            let authenticated = try await context.evaluatePolicy(
                .deviceOwnerAuthenticationWithBiometrics,
                localizedReason: "Unlock Mixin Messenger"
            )
            if authenticated {
                isLocked = false
            }
            return authenticated
        } catch {
            return false
        }
    }

    func appDidLeaveActive() {
        cancelPendingLock()
        guard isInitialized,
              hasPasscode,
              !isLocked,
              autoLockMinutes > 0
        else {
            return
        }
        let duration = Duration.seconds(autoLockMinutes * 60)
        lockTask = Task { [weak self] in
            do {
                try await Task.sleep(for: duration)
            } catch {
                return
            }
            guard !Task.isCancelled else {
                return
            }
            self?.lockNow()
        }
    }

    func appDidBecomeActive() {
        cancelPendingLock()
    }

    func shutdown() {
        cancelPendingLock()
    }

    func canUseBiometrics() -> Bool {
        let context = LAContext()
        var authorizationError: NSError?
        return context.canEvaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            error: &authorizationError
        )
    }

    static let supportedAutoLockMinutes = [0, 1, 5, 60, 300]

    private static func isValidPasscode(_ value: String) -> Bool {
        value.count == 6 && value.allSatisfy(isPasscodeDigit)
    }

    private func key(_ suffix: String) -> String {
        prefix + suffix
    }

    private func cancelPendingLock() {
        lockTask?.cancel()
        lockTask = nil
    }

    private func encode<T: Encodable>(_ value: T) throws -> String {
        let data = try JSONEncoder().encode(value)
        guard let encoded = String(data: data, encoding: .utf8) else {
            throw SecuritySettingsError.encodingFailed
        }
        return encoded
    }

    private func decode<T: Decodable>(_ value: String?) throws -> T? {
        guard let value else {
            return nil
        }
        return try JSONDecoder().decode(T.self, from: Data(value.utf8))
    }
}

enum SecuritySettingsError: LocalizedError {
    case invalidPasscode
    case corruptedPasscode
    case invalidAutoLockDuration
    case corruptedAutoLockDuration
    case biometricUnavailable
    case encodingFailed

    var errorDescription: String? {
        switch self {
        case .invalidPasscode:
            "Passcode must contain exactly 6 digits."
        case .corruptedPasscode:
            "The saved screen passcode is invalid."
        case .invalidAutoLockDuration:
            "The selected auto-lock duration is invalid."
        case .corruptedAutoLockDuration:
            "The saved auto-lock duration is invalid."
        case .biometricUnavailable:
            "Touch ID is not available on this Mac."
        case .encodingFailed:
            "The security setting could not be encoded."
        }
    }
}
