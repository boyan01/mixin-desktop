import Foundation
import Observation
import SwiftUI

enum ChatTimelineAnchor: Equatable {
  case latest
  case message(
    id: String,
    alignment: ChatTimelineAnchorAlignment,
    offset: CGFloat,
    highlight: Bool
  )
}

enum ChatTimelineAnchorAlignment: Equatable {
  case top
  case focus
  case center

  var unitPoint: UnitPoint {
    switch self {
    case .top:
      .top
    case .focus:
      UnitPoint(x: 0.5, y: 0.2)
    case .center:
      .center
    }
  }
}

enum ChatTimelineAppendSource: Equatable {
  case history
  case live(sentByCurrentUser: Bool)
}

enum ChatTimelineMutation: Equatable {
  case reset(anchor: ChatTimelineAnchor)
  case prepend
  case append(source: ChatTimelineAppendSource, count: Int)
  case update
  case jump(anchor: ChatTimelineAnchor)
}

struct ChatTimelineChange: Equatable {
  let revision: Int
  let mutation: ChatTimelineMutation
}

struct ChatScrollGeometry: Equatable {
  let contentOffsetY: CGFloat
  let contentHeight: CGFloat
  let containerHeight: CGFloat
  let visibleMinY: CGFloat
  let visibleMaxY: CGFloat

  init(_ geometry: ScrollGeometry) {
    contentOffsetY = geometry.contentOffset.y
    contentHeight = geometry.contentSize.height
    containerHeight = geometry.containerSize.height
    visibleMinY = geometry.visibleRect.minY
    visibleMaxY = geometry.visibleRect.maxY
  }

  var distanceToBottom: CGFloat {
    max(contentHeight - visibleMaxY, 0)
  }
}

enum ChatPaginationDirection {
  case older
  case newer
}

@MainActor
@Observable
final class ChatScrollCoordinator {
  private struct PendingPrepend {
    let contentOffsetY: CGFloat
    let contentHeight: CGFloat
  }

  private struct PendingMessageAnchor {
    let messageID: String
    let desiredOffset: CGFloat
    let initialContentHeight: CGFloat
  }

  private static let tailFollowThreshold: CGFloat = 50
  private static let jumpButtonThreshold: CGFloat = 120
  private static let preloadViewportCount: CGFloat = 3
  private static let nearbyJumpViewportCount: CGFloat = 2
  private static let correctionTolerance: CGFloat = 0.5

  private(set) var showJumpToLatest = false
  private(set) var newMessageCount = 0
  private(set) var highlightedMessageID: String?

  @ObservationIgnored private var active = false
  @ObservationIgnored private var conversationID: String?
  @ObservationIgnored private var geometry: ChatScrollGeometry?
  @ObservationIgnored private var phase: ScrollPhase = .idle
  @ObservationIgnored private var visibleMessageIDs: [String] = []
  @ObservationIgnored private var rowFrames: [String: CGRect] = [:]
  @ObservationIgnored private var pendingPrepend: PendingPrepend?
  @ObservationIgnored private var pendingMessageAnchor: PendingMessageAnchor?
  @ObservationIgnored private var blockedUntilUserScroll = false
  @ObservationIgnored private var lastHandledRevision = 0
  @ObservationIgnored private var lastPaginationOffsetY: CGFloat?
  @ObservationIgnored private var highlightTask: Task<Void, Never>?

  func reset(conversationID: String) {
    highlightTask?.cancel()
    active = true
    self.conversationID = conversationID
    showJumpToLatest = false
    newMessageCount = 0
    highlightedMessageID = nil
    geometry = nil
    phase = .idle
    visibleMessageIDs = []
    rowFrames = [:]
    pendingPrepend = nil
    pendingMessageAnchor = nil
    blockedUntilUserScroll = false
    lastHandledRevision = 0
    lastPaginationOffsetY = nil
  }

  func stop() {
    active = false
    highlightTask?.cancel()
    highlightTask = nil
    pendingPrepend = nil
    pendingMessageAnchor = nil
  }

  func handle(
    _ change: ChatTimelineChange?,
    hasNewerMessages: Bool,
    reduceMotion: Bool,
    position: Binding<ScrollPosition>
  ) {
    guard active,
      let change,
      change.revision > lastHandledRevision
    else {
      return
    }
    lastHandledRevision = change.revision

    switch change.mutation {
    case .reset(let anchor):
      pendingPrepend = nil
      blockedUntilUserScroll = true
      prepareInitialPosition(anchor, position: position)
    case .jump(let anchor):
      pendingPrepend = nil
      blockedUntilUserScroll = true
      apply(
        anchor,
        reduceMotion: reduceMotion,
        position: position
      )
    case .prepend:
      if let geometry {
        pendingPrepend = PendingPrepend(
          contentOffsetY: geometry.contentOffsetY,
          contentHeight: geometry.contentHeight
        )
      }
    case .append(let source, let count):
      guard case .live = source else {
        return
      }
      let sentByCurrentUser =
        if case .live(let sentByCurrentUser) = source {
          sentByCurrentUser
        } else {
          false
        }
      let followsTail =
        phase == .idle
        && !hasNewerMessages
        && (sentByCurrentUser || isTailEligible)
      if followsTail {
        newMessageCount = 0
        scrollToLatest(
          position: position,
          animated: !reduceMotion
        )
      } else {
        preserveVisibleAnchorIfIdle()
        newMessageCount += count
        showJumpToLatest = true
      }
    case .update:
      preserveVisibleAnchorIfIdle()
    }
  }

  func updateGeometry(
    _ newGeometry: ChatScrollGeometry,
    hasOlderMessages: Bool,
    hasNewerMessages: Bool,
    position: Binding<ScrollPosition>
  ) -> ChatPaginationDirection? {
    guard active else {
      return nil
    }
    let previous = geometry
    let wasTailEligible = isTailEligible
    geometry = newGeometry
    pruneRowFrames(using: newGeometry)

    restorePrependIfNeeded(
      using: newGeometry,
      position: position
    )
    restoreMessageAnchorIfNeeded(
      using: newGeometry,
      position: position
    )

    if let previous,
      previous.containerHeight != newGeometry.containerHeight,
      wasTailEligible,
      !hasNewerMessages,
      phase == .idle
    {
      scrollToLatest(position: position, animated: false)
    }

    showJumpToLatest =
      hasNewerMessages
      || newGeometry.distanceToBottom > Self.jumpButtonThreshold
    if !showJumpToLatest {
      newMessageCount = 0
    }

    let previousOffset = lastPaginationOffsetY
    lastPaginationOffsetY = newGeometry.contentOffsetY
    let underfilled =
      newGeometry.contentHeight
      <= newGeometry.containerHeight + Self.correctionTolerance
    if underfilled, hasOlderMessages {
      return .older
    }
    guard !blockedUntilUserScroll,
      phase != .animating,
      let previousOffset
    else {
      return nil
    }

    let delta = newGeometry.contentOffsetY - previousOffset
    let threshold =
      newGeometry.containerHeight
      * Self.preloadViewportCount
    if delta < 0,
      hasOlderMessages,
      newGeometry.visibleMinY <= threshold
    {
      return .older
    }
    if delta > 0,
      hasNewerMessages,
      newGeometry.distanceToBottom <= threshold
    {
      return .newer
    }
    return nil
  }

  func updatePhase(_ phase: ScrollPhase) {
    guard active else {
      return
    }
    self.phase = phase
    if phase == .tracking || phase == .interacting {
      blockedUntilUserScroll = false
      pendingPrepend = nil
      pendingMessageAnchor = nil
    }
  }

  func updateVisibleMessageIDs(_ ids: [String]) {
    guard active else {
      return
    }
    visibleMessageIDs = ids
  }

  func updateRowFrame(
    messageID: String,
    frame: CGRect,
    position: Binding<ScrollPosition>
  ) {
    guard active else {
      return
    }
    rowFrames[messageID] = frame
    if let geometry {
      pruneRowFrames(using: geometry)
      restoreMessageAnchorIfNeeded(
        using: geometry,
        position: position
      )
    }
  }

  func canAnimateJump(to messageID: String) -> Bool {
    guard let geometry,
      geometry.containerHeight > 0,
      let frame = rowFrames[messageID]
    else {
      return false
    }
    let viewportCenter = geometry.containerHeight / 2
    return abs(frame.midY - viewportCenter)
      <= geometry.containerHeight * Self.nearbyJumpViewportCount
  }

  func prepareInitialPosition(
    _ anchor: ChatTimelineAnchor,
    position: Binding<ScrollPosition>
  ) {
    guard active else {
      return
    }
    pendingPrepend = nil
    newMessageCount = 0
    let value: ScrollPosition
    switch anchor {
    case .latest:
      pendingMessageAnchor = nil
      value = ScrollPosition(idType: String.self, edge: .bottom)
    case .message(let id, let alignment, let offset, let highlight):
      if alignment == .top {
        pendingMessageAnchor = PendingMessageAnchor(
          messageID: id,
          desiredOffset: offset,
          initialContentHeight: geometry?.contentHeight ?? 0
        )
      } else {
        pendingMessageAnchor = nil
      }
      value = ScrollPosition(id: id, anchor: alignment.unitPoint)
      if highlight {
        flash(messageID: id)
      }
    }
    var transaction = Transaction()
    transaction.disablesAnimations = true
    withTransaction(transaction) {
      position.wrappedValue = value
    }
  }

  func scrollToLatest(
    position: Binding<ScrollPosition>,
    animated: Bool
  ) {
    guard active else {
      return
    }
    blockedUntilUserScroll = true
    newMessageCount = 0
    if animated {
      withAnimation(.easeOut(duration: 0.25)) {
        mutate(position) {
          $0.scrollTo(edge: .bottom)
        }
      }
    } else {
      var transaction = Transaction()
      transaction.disablesAnimations = true
      withTransaction(transaction) {
        mutate(position) {
          $0.scrollTo(edge: .bottom)
        }
      }
    }
  }

  func savedPosition(hasNewerMessages: Bool) -> ChatViewportPosition? {
    guard hasNewerMessages || !isTailEligible else {
      return nil
    }
    guard let messageID = visibleMessageIDs.first,
      let frame = rowFrames[messageID]
    else {
      return nil
    }
    return ChatViewportPosition(
      messageID: messageID,
      offset: frame.minY
    )
  }

  private var isTailEligible: Bool {
    guard let geometry else {
      return true
    }
    return geometry.distanceToBottom <= Self.tailFollowThreshold
  }

  private func preserveVisibleAnchorIfIdle() {
    guard phase == .idle,
      let geometry,
      let messageID = visibleMessageIDs.first,
      let frame = rowFrames[messageID]
    else {
      return
    }
    pendingMessageAnchor = PendingMessageAnchor(
      messageID: messageID,
      desiredOffset: frame.minY,
      initialContentHeight: geometry.contentHeight
    )
  }

  private func pruneRowFrames(using geometry: ChatScrollGeometry) {
    let margin = geometry.containerHeight * Self.nearbyJumpViewportCount
    let retained = Set(visibleMessageIDs)
      .union(pendingMessageAnchor.map { [$0.messageID] } ?? [])
    rowFrames = rowFrames.filter { messageID, frame in
      retained.contains(messageID)
        || (frame.maxY >= -margin
          && frame.minY <= geometry.containerHeight + margin)
    }
  }

  private func apply(
    _ anchor: ChatTimelineAnchor,
    reduceMotion: Bool,
    position: Binding<ScrollPosition>
  ) {
    switch anchor {
    case .latest:
      pendingMessageAnchor = nil
      newMessageCount = 0
      scrollToLatest(position: position, animated: false)
    case .message(let id, let alignment, let offset, let highlight):
      if alignment == .top {
        pendingMessageAnchor = PendingMessageAnchor(
          messageID: id,
          desiredOffset: offset,
          initialContentHeight: geometry?.contentHeight ?? 0
        )
      } else {
        pendingMessageAnchor = nil
      }
      let action = {
        self.mutate(position) {
          $0.scrollTo(id: id, anchor: alignment.unitPoint)
        }
      }
      if reduceMotion {
        var transaction = Transaction()
        transaction.disablesAnimations = true
        withTransaction(transaction, action)
      } else {
        withAnimation(.easeOut(duration: 0.25), action)
      }
      if highlight {
        flash(messageID: id)
      }
    }
  }

  private func restorePrependIfNeeded(
    using geometry: ChatScrollGeometry,
    position: Binding<ScrollPosition>
  ) {
    guard let pendingPrepend,
      geometry.contentHeight > pendingPrepend.contentHeight
        + Self.correctionTolerance
    else {
      return
    }
    self.pendingPrepend = nil
    let contentDelta = geometry.contentHeight - pendingPrepend.contentHeight
    var transaction = Transaction()
    transaction.disablesAnimations = true
    withTransaction(transaction) {
      mutate(position) {
        $0.scrollTo(y: pendingPrepend.contentOffsetY + contentDelta)
      }
    }
  }

  private func restoreMessageAnchorIfNeeded(
    using geometry: ChatScrollGeometry,
    position: Binding<ScrollPosition>
  ) {
    guard let pendingMessageAnchor,
      let frame = rowFrames[pendingMessageAnchor.messageID],
      geometry.contentHeight != pendingMessageAnchor.initialContentHeight
        || abs(frame.minY - pendingMessageAnchor.desiredOffset)
          > Self.correctionTolerance
    else {
      return
    }
    let correction = frame.minY - pendingMessageAnchor.desiredOffset
    self.pendingMessageAnchor = nil
    guard abs(correction) > Self.correctionTolerance else {
      return
    }
    var transaction = Transaction()
    transaction.disablesAnimations = true
    withTransaction(transaction) {
      mutate(position) {
        $0.scrollTo(y: geometry.contentOffsetY + correction)
      }
    }
  }

  private func mutate(
    _ position: Binding<ScrollPosition>,
    _ mutation: (inout ScrollPosition) -> Void
  ) {
    var value = position.wrappedValue
    mutation(&value)
    position.wrappedValue = value
  }

  private func flash(messageID: String) {
    highlightTask?.cancel()
    highlightedMessageID = messageID
    highlightTask = Task { [weak self] in
      try? await Task.sleep(for: .milliseconds(900))
      guard !Task.isCancelled else {
        return
      }
      self?.highlightedMessageID = nil
    }
  }
}
