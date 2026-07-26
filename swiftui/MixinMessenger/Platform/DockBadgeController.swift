import AppKit
import Observation

@MainActor
@Observable
final class DockBadgeController {
    private var subscription: SwiftUnseenMessageCountSubscription?
    private var task: Task<Void, Never>?

    func start(account: SwiftAccountHandle) {
        stop()
        let subscription = account.unseenMessageCountChanges()
        self.subscription = subscription
        task = Task {
            while !Task.isCancelled,
                  let count = await subscription.next()
            {
                NSApplication.shared.dockTile.badgeLabel = count > 0
                    ? String(count)
                    : nil
            }
        }
    }

    func stop() {
        subscription?.cancel()
        subscription = nil
        task?.cancel()
        task = nil
        NSApplication.shared.dockTile.badgeLabel = nil
    }
}
