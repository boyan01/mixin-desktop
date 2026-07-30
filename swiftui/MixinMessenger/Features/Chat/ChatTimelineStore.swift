import Foundation

struct ChatTimelineRow: Identifiable, Equatable {
  let messageID: String
  let startsNewDay: Bool
  let sameUserPrevious: Bool
  let sameUserNext: Bool

  var id: String { messageID }
}

struct ChatTimelineStore {
  private(set) var messages: [MessageItem] = []
  private(set) var rows: [ChatTimelineRow] = []
  private(set) var imageMessages: [MessageItem] = []
  private(set) var audioMessages: [MessageItem] = []
  private(set) var mediaRevision = 0
  private(set) var mentionContents = Set<String>()
  private(set) var mutableMessageIDs = Set<String>()

  private var indicesByID: [String: Int] = [:]

  func message(id: String) -> MessageItem? {
    guard let index = indicesByID[id] else {
      return nil
    }
    return messages[index]
  }

  func contains(_ messageID: String) -> Bool {
    indicesByID[messageID] != nil
  }

  @discardableResult
  mutating func removeAll() -> Bool {
    guard !messages.isEmpty else {
      return false
    }
    messages = []
    rows = []
    imageMessages = []
    audioMessages = []
    mentionContents = []
    mutableMessageIDs = []
    indicesByID = [:]
    mediaRevision += 1
    return true
  }

  @discardableResult
  mutating func reset(with items: [MessageItem]) -> Bool {
    let items = Self.deduplicated(items)
    guard items != messages else {
      return false
    }
    messages = items
    rebuildIndices()
    rows = messages.indices.map(makeRow)
    rebuildMediaIndexes()
    mentionContents = []
    indexMentionContents(in: messages)
    mutableMessageIDs = Set(
      messages.compactMap {
        Self.isMutable($0) ? $0.messageId : nil
      })
    return true
  }

  @discardableResult
  mutating func prepend(_ items: [MessageItem]) -> Bool {
    insert(items, atStart: true)
  }

  @discardableResult
  mutating func append(_ items: [MessageItem]) -> Bool {
    insert(items, atStart: false)
  }

  @discardableResult
  mutating func update(
    _ items: [MessageItem],
    removingIDs: Set<String> = []
  ) -> Bool {
    var affected = Set<Int>()
    var mediaChanged = false
    var changedMessages: [MessageItem] = []

    for item in Self.deduplicated(items) {
      guard let index = indicesByID[item.messageId],
        messages[index] != item
      else {
        continue
      }
      let previous = messages[index]
      messages[index] = item
      affected.formUnion([index - 1, index, index + 1])
      mediaChanged =
        mediaChanged
        || Self.isImage(previous)
        || Self.isImage(item)
        || Self.isAudio(previous)
        || Self.isAudio(item)
      updateMutableIndex(for: item)
      changedMessages.append(item)
    }

    let existingRemovals = removingIDs.compactMap { indicesByID[$0] }
    if !existingRemovals.isEmpty {
      for index in existingRemovals {
        mediaChanged =
          mediaChanged
          || Self.isImage(messages[index])
          || Self.isAudio(messages[index])
        mutableMessageIDs.remove(messages[index].messageId)
      }
      let removed = Set(existingRemovals)
      messages = messages.enumerated().compactMap {
        removed.contains($0.offset) ? nil : $0.element
      }
      rebuildIndices()
      rows = messages.indices.map(makeRow)
    } else {
      rebuildRows(at: affected)
    }

    guard !affected.isEmpty || !existingRemovals.isEmpty else {
      return false
    }
    if mediaChanged {
      rebuildMediaIndexes()
    }
    indexMentionContents(in: changedMessages)
    return true
  }

  private mutating func insert(
    _ items: [MessageItem],
    atStart: Bool
  ) -> Bool {
    let items = Self.deduplicated(items)
    guard !items.isEmpty else {
      return false
    }

    var inserted: [MessageItem] = []
    var updates: [MessageItem] = []
    for item in items {
      if indicesByID[item.messageId] == nil {
        inserted.append(item)
      } else {
        updates.append(item)
      }
    }

    var changed = update(updates)
    guard !inserted.isEmpty else {
      return changed
    }

    let oldCount = messages.count
    if atStart {
      messages.insert(contentsOf: inserted, at: 0)
      rebuildIndices()
      rows.insert(
        contentsOf: inserted.indices.map(makeRow),
        at: 0
      )
      rebuildRows(at: [inserted.count - 1, inserted.count])
    } else {
      messages.append(contentsOf: inserted)
      for (offset, message) in inserted.enumerated() {
        indicesByID[message.messageId] = oldCount + offset
      }
      rows.append(
        contentsOf: inserted.indices.map {
          makeRow(at: oldCount + $0)
        }
      )
      rebuildRows(at: [oldCount - 1, oldCount])
    }

    let insertedImages = inserted.filter(Self.isImage)
    let insertedAudio = inserted.filter(Self.isAudio)
    if !insertedImages.isEmpty || !insertedAudio.isEmpty {
      if atStart {
        imageMessages.insert(contentsOf: insertedImages, at: 0)
        audioMessages.insert(contentsOf: insertedAudio, at: 0)
      } else {
        imageMessages.append(contentsOf: insertedImages)
        audioMessages.append(contentsOf: insertedAudio)
      }
      mediaRevision += 1
    }
    for message in inserted {
      updateMutableIndex(for: message)
    }
    indexMentionContents(in: inserted)
    changed = true
    return changed
  }

  private mutating func rebuildIndices() {
    indicesByID = Dictionary(
      uniqueKeysWithValues: messages.enumerated().map {
        ($0.element.messageId, $0.offset)
      }
    )
  }

  private mutating func rebuildRows(at indices: Set<Int>) {
    for index in indices where messages.indices.contains(index) {
      rows[index] = makeRow(at: index)
    }
  }

  private func makeRow(at index: Int) -> ChatTimelineRow {
    let message = messages[index]
    let previous = index > 0 ? messages[index - 1] : nil
    let next = index + 1 < messages.count ? messages[index + 1] : nil
    let day = Self.calendarDay(message.createdAtMicros)
    let sameDayPrevious =
      previous.map {
        Self.calendarDay($0.createdAtMicros) == day
      } ?? false
    let sameDayNext =
      next.map {
        Self.calendarDay($0.createdAtMicros) == day
      } ?? false
    return ChatTimelineRow(
      messageID: message.messageId,
      startsNewDay: !sameDayPrevious,
      sameUserPrevious: sameDayPrevious
        && previous?.senderId == message.senderId
        && previous?.breaksMessageGrouping == false,
      sameUserNext: sameDayNext
        && next?.senderId == message.senderId
        && !message.breaksMessageGrouping
    )
  }

  private mutating func rebuildMediaIndexes() {
    let images = messages.filter(Self.isImage)
    let audio = messages.filter(Self.isAudio)
    guard images != imageMessages || audio != audioMessages else {
      return
    }
    imageMessages = images
    audioMessages = audio
    mediaRevision += 1
  }

  private mutating func updateMutableIndex(for message: MessageItem) {
    if Self.isMutable(message) {
      mutableMessageIDs.insert(message.messageId)
    } else {
      mutableMessageIDs.remove(message.messageId)
    }
  }

  private mutating func indexMentionContents(
    in changedMessages: [MessageItem]
  ) {
    for content
      in changedMessages
      .flatMap({ [$0.content, $0.caption, $0.quoteContent] })
      .compactMap({ $0 })
    where content.range(
      of: #"@\d{4,}"#,
      options: .regularExpression
    ) != nil {
      mentionContents.insert(content)
    }
  }

  private static func deduplicated(
    _ items: [MessageItem]
  ) -> [MessageItem] {
    var seen = Set<String>()
    return items.filter { seen.insert($0.messageId).inserted }
  }

  nonisolated private static func isImage(
    _ message: MessageItem
  ) -> Bool {
    message.category.hasSuffix("_IMAGE")
      && message.mediaStatus.isComplete
      && message.localMediaURL != nil
  }

  nonisolated private static func isAudio(
    _ message: MessageItem
  ) -> Bool {
    message.category.hasSuffix("_AUDIO")
      && ["DONE", "READ"].contains(message.mediaStatus.uppercased())
  }

  private static func isMutable(_ message: MessageItem) -> Bool {
    message.mediaStatus.uppercased() == "PENDING"
      || (message.presentationKind == .sticker
        && message.presentationImageURL == nil)
  }

  private static func calendarDay(_ micros: Int64) -> Date {
    Calendar.current.startOfDay(
      for: Date(timeIntervalSince1970: Double(micros) / 1_000_000)
    )
  }
}
