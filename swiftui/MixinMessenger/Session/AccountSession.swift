import Observation

@MainActor
@Observable
final class AccountSession {
    let handle: SwiftAccountHandle
    let media: SwiftMediaHandle
    let security: SecurityService
    let deviceTransfer: DeviceTransferController
    private(set) var profile: SwiftAccountProfile
    private(set) var health = "ready"
    private(set) var connected = false
    private(set) var connectedBefore = false
    private var stopped = false
    private var healthSubscription: SwiftAccountHealthSubscription?
    private var healthTask: Task<Void, Never>?
    private var connectionSubscription: SwiftConnectionSubscription?
    private var connectionTask: Task<Void, Never>?
    private var securityTask: Task<Void, Never>?

    init(handle: SwiftAccountHandle, desktop: SwiftDesktopHandle) {
        self.handle = handle
        media = desktop.media()
        AudioPlaybackCoordinator.shared.bind(media)
        let profile = handle.profile()
        self.profile = profile
        security = SecurityService(
            desktop: desktop,
            accountID: profile.userId
        )
        deviceTransfer = DeviceTransferController(account: handle)
    }

    func start() {
        stopSubscriptions()
        securityTask = Task { [security] in
            await security.load()
        }
        deviceTransfer.start()
        let connectionSubscription = handle.connectionStatus()
        self.connectionSubscription = connectionSubscription
        connectionTask = Task { [weak self] in
            while !Task.isCancelled,
                  let connected = await connectionSubscription.next()
            {
                guard let self, !stopped else {
                    return
                }
                self.connected = connected
                if connected {
                    connectedBefore = true
                }
            }
        }
        let subscription = handle.accountHealth()
        healthSubscription = subscription
        healthTask = Task { [weak self] in
            while !Task.isCancelled,
                  let value = await subscription.next()
            {
                guard let self, !stopped else {
                    return
                }
                health = value
            }
        }
    }

    func updateProfile(fullName: String, biography: String) async throws {
        profile = try await handle.updateProfile(
            fullName: fullName,
            biography: biography
        )
    }

    func updateAvatar(_ avatarBase64: String) async throws {
        profile = try await handle.updateAvatar(avatarBase64: avatarBase64)
    }

    func refreshProfile() async throws {
        profile = try await handle.refreshProfile()
    }

    func refreshAccountHealth() async throws {
        try await handle.refreshAccountHealth()
    }

    func retryConnection() {
        handle.retryConnection()
    }

    func signOut() async throws {
        guard !stopped else {
            return
        }
        stopSubscriptions()
        security.shutdown()
        deviceTransfer.stop()
        try await handle.signOut()
        stopped = true
    }

    func shutdown() async {
        guard !stopped else {
            return
        }
        stopSubscriptions()
        security.shutdown()
        deviceTransfer.stop()
        stopped = true
        await handle.shutdown()
    }

    private func stopSubscriptions() {
        healthSubscription?.cancel()
        healthSubscription = nil
        healthTask?.cancel()
        healthTask = nil
        connectionSubscription?.cancel()
        connectionSubscription = nil
        connectionTask?.cancel()
        connectionTask = nil
        securityTask?.cancel()
        securityTask = nil
    }
}
