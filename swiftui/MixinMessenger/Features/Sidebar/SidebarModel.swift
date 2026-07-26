import Foundation
import Observation
import SwiftUI

@MainActor
@Observable
final class SidebarModel {
    private(set) var circles: [SwiftCircleItem] = []
    private(set) var unseenCounts: [SwiftConversationUnseenCount] = []
    private(set) var errorMessage: String?

    private var circleSubscription: SwiftCircleSubscription?
    private var unseenSubscription: SwiftUnseenCountSubscription?
    private var circleTask: Task<Void, Never>?
    private var unseenTask: Task<Void, Never>?
    private var started = false

    func start(account: SwiftAccountHandle) async {
        guard !started else {
            return
        }
        started = true
        do {
            circles = try await account.circles()
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
        }

        let circleSubscription = account.circleChanges()
        self.circleSubscription = circleSubscription
        circleTask = Task { [weak self] in
            while !Task.isCancelled,
                  let circles = await circleSubscription.next()
            {
                self?.circles = circles
            }
        }

        let unseenSubscription = account.unseenCountChanges()
        self.unseenSubscription = unseenSubscription
        unseenTask = Task { [weak self] in
            while !Task.isCancelled,
                  let counts = await unseenSubscription.next()
            {
                self?.unseenCounts = counts
            }
        }
    }

    func stop() {
        circleSubscription?.cancel()
        unseenSubscription?.cancel()
        circleSubscription = nil
        unseenSubscription = nil
        circleTask?.cancel()
        unseenTask?.cancel()
        circleTask = nil
        unseenTask = nil
        started = false
    }

    func updateCircle(
        _ circle: SwiftCircleItem,
        name: String,
        account: SwiftAccountHandle
    ) async -> Bool {
        do {
            try await account.updateCircle(
                circleId: circle.circleId,
                name: name
            )
            return true
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func deleteCircle(
        _ circle: SwiftCircleItem,
        account: SwiftAccountHandle
    ) async -> Bool {
        do {
            try await account.deleteCircle(circleId: circle.circleId)
            return true
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func reorderCircles(
        fromOffsets: IndexSet,
        toOffset: Int,
        account: SwiftAccountHandle
    ) async {
        let original = circles
        circles.move(fromOffsets: fromOffsets, toOffset: toOffset)
        let reordered = circles
        do {
            try await account.reorderCircles(
                circleIds: reordered.map(\.circleId)
            )
        } catch {
            if circles.map(\.circleId) == reordered.map(\.circleId) {
                circles = original
            }
            errorMessage = MixinErrorPresenter.message(for: error)
        }
    }

    func clearError() {
        errorMessage = nil
    }

    func unseenCount(for section: HomeSection) -> (count: Int64, muted: Int64) {
        let item = unseenCounts.first { item in
            switch section {
            case .chats:
                item.category == "chats" && item.circleId == nil
            case .contacts:
                item.category == "contacts" && item.circleId == nil
            case .groups:
                item.category == "groups" && item.circleId == nil
            case .bots:
                item.category == "bots" && item.circleId == nil
            case .strangers:
                item.category == "strangers" && item.circleId == nil
            case let .circle(circleID):
                item.circleId == circleID
            case .settings:
                false
            }
        }
        return (item?.count ?? 0, item?.mutedCount ?? 0)
    }
}
