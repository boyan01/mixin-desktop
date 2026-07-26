import AppKit
import Observation
import SwiftUI

struct MentionComposer: View {
  @Environment(\.colorScheme) private var colorScheme
  let account: SwiftAccountHandle
  let conversationID: String
  let conversationCategory: String?
  let conversationOwnerID: String?
  let currentUserID: String
  let encrypted: Bool
  @Binding var text: String
  let disabled: Bool
  let focusRevision: Int
  let onPasteFiles: ([URL]) -> Void
  let onSubmit: () -> Void
  let onSubmitPost: () -> Void

  @State private var model = MentionComposerModel()
  @State private var selectionRequest: MentionSelectionRequest?
  @State private var editorHeight: CGFloat = 40

  var body: some View {
    ZStack(alignment: .leading) {
      MentionTextEditor(
        text: $text,
        height: $editorHeight,
        highlightedIdentities: model.knownMentionIdentityNumbers,
        candidatesVisible: !model.candidates.isEmpty,
        selectionRequest: selectionRequest,
        disabled: disabled,
        focusRevision: focusRevision,
        onEditingChanged: { text, selection, hasMarkedText in
          model.updateInput(
            text: text,
            selectionUTF16: selection,
            hasMarkedText: hasMarkedText
          )
        },
        onMoveCandidate: model.moveSelection,
        onSelectCandidate: selectCandidate,
        onDismissCandidates: model.dismiss,
        onPasteFiles: onPasteFiles,
        onSubmit: onSubmit,
        onSubmitPost: onSubmitPost
      )
      if text.isEmpty {
        Text(encrypted ? "End-to-End Encryption" : "Message")
          .font(.system(size: 14))
          .foregroundStyle(.tertiary)
          .padding(.leading, 8)
          .allowsHitTesting(false)
      }
    }
    .frame(height: editorHeight)
    .background(inputBackground, in: RoundedRectangle(cornerRadius: 4))
    .overlay(alignment: .top) {
      if !model.candidates.isEmpty {
        MentionCandidatePanel(
          users: model.candidates,
          keyword: model.keyword,
          selectedIndex: model.selectedIndex,
          onHover: model.selectIndex,
          onSelect: selectCandidate
        )
        .frame(height: min(CGFloat(model.candidates.count) * 50, 200))
        .offset(y: -min(CGFloat(model.candidates.count) * 50, 200) - 4)
        .transition(.move(edge: .bottom).combined(with: .opacity))
        .zIndex(10)
      }
    }
    .animation(.easeOut(duration: 0.15), value: model.candidates.isEmpty)
    .task(id: configurationID) {
      await model.configure(
        account: account,
        conversationID: conversationID,
        conversationCategory: conversationCategory,
        conversationOwnerID: conversationOwnerID,
        currentUserID: currentUserID,
        initialText: text
      )
    }
    .onDisappear {
      model.stop()
    }
  }

  private var inputBackground: Color {
    colorScheme == .dark
      ? Color.white.opacity(0.08)
      : Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255)
  }

  private var configurationID: String {
    [
      conversationID,
      conversationCategory ?? "",
      conversationOwnerID ?? "",
      currentUserID,
    ].joined(separator: ":")
  }

  private func selectCandidate(_ index: Int?) {
    guard let insertion = model.selectCandidate(at: index, in: text) else {
      return
    }
    text = insertion.text
    selectionRequest = MentionSelectionRequest(
      offset: insertion.selectionUTF16,
      revision: (selectionRequest?.revision ?? 0) + 1
    )
  }
}

@MainActor
@Observable
final class MentionComposerModel {
  private(set) var candidates: [SwiftConversationParticipantItem] = []
  private(set) var keyword = ""
  private(set) var selectedIndex = 0
  private(set) var knownMentionIdentityNumbers = Set<String>()

  private enum Mode {
    case disabled
    case group
    case bot
  }

  private var account: SwiftAccountHandle?
  private var conversationID: String?
  private var currentUserID: String?
  private var mode = Mode.disabled
  private var loadTask: Task<Void, Never>?
  private var namesTask: Task<Void, Never>?
  private var inputText = ""
  private var selectionUTF16 = 0
  private var hasMarkedText = false
  private var requestRevision = 0

  func configure(
    account: SwiftAccountHandle,
    conversationID: String,
    conversationCategory: String?,
    conversationOwnerID: String?,
    currentUserID: String,
    initialText: String
  ) async {
    stop()
    self.account = account
    self.conversationID = conversationID
    self.currentUserID = currentUserID
    inputText = initialText
    selectionUTF16 = initialText.utf16.count

    if conversationCategory == "GROUP" {
      mode = .group
    } else if let conversationOwnerID,
      let owner = try? await account.userProfile(userId: conversationOwnerID),
      owner.isBot
    {
      mode = .bot
    } else {
      mode = .disabled
    }

    guard self.conversationID == conversationID else {
      return
    }
    updateInput(
      text: inputText,
      selectionUTF16: selectionUTF16,
      hasMarkedText: false
    )
  }

  func updateInput(
    text: String,
    selectionUTF16: Int,
    hasMarkedText: Bool
  ) {
    inputText = text
    self.selectionUTF16 = selectionUTF16
    self.hasMarkedText = hasMarkedText
    resolveMentionNames(in: text)

    guard mode != .disabled,
      !hasMarkedText,
      let match = Self.mentionMatch(in: text, selectionUTF16: selectionUTF16)
    else {
      dismiss()
      return
    }

    if keyword != match.keyword {
      selectedIndex = 0
    }
    keyword = match.keyword
    loadCandidates(keyword: match.keyword)
  }

  func moveSelection(_ delta: Int) {
    guard !candidates.isEmpty else {
      return
    }
    selectedIndex = min(max(selectedIndex + delta, 0), candidates.count - 1)
  }

  func selectIndex(_ index: Int) {
    guard candidates.indices.contains(index) else {
      return
    }
    selectedIndex = index
  }

  func selectCandidate(at index: Int?, in text: String) -> MentionInsertion? {
    guard !hasMarkedText,
      let match = Self.mentionMatch(in: text, selectionUTF16: selectionUTF16),
      candidates.indices.contains(index ?? selectedIndex)
    else {
      return nil
    }
    let user = candidates[index ?? selectedIndex]
    let nsText = text as NSString
    let replacement = "@\(user.identityNumber) "
    let updated = nsText.replacingCharacters(in: match.range, with: replacement)
    let cursor = match.range.location + (replacement as NSString).length
    knownMentionIdentityNumbers.insert(user.identityNumber)
    dismiss()
    inputText = updated
    selectionUTF16 = cursor
    resolveMentionNames(in: updated)
    return MentionInsertion(text: updated, selectionUTF16: cursor)
  }

  func dismiss() {
    requestRevision += 1
    loadTask?.cancel()
    loadTask = nil
    candidates = []
    keyword = ""
    selectedIndex = 0
  }

  func stop() {
    requestRevision += 1
    loadTask?.cancel()
    namesTask?.cancel()
    loadTask = nil
    namesTask = nil
    account = nil
    conversationID = nil
    currentUserID = nil
    mode = .disabled
    candidates = []
    keyword = ""
    selectedIndex = 0
    knownMentionIdentityNumbers = []
  }

  private func loadCandidates(keyword: String) {
    guard let account, let conversationID else {
      dismiss()
      return
    }
    requestRevision += 1
    let revision = requestRevision
    let mode = mode
    let currentUserID = currentUserID
    loadTask?.cancel()
    loadTask = Task {
      do {
        let users: [SwiftConversationParticipantItem]
        switch mode {
        case .group where keyword.isEmpty:
          users = try await account.conversationParticipants(
            conversationId: conversationID
          ).filter { $0.userId != currentUserID }
        case .group:
          users = try await account.searchGroupUsers(
            conversationId: conversationID,
            keyword: keyword
          )
        case .bot:
          users = try await account.searchBotGroupUsers(
            conversationId: conversationID,
            keyword: keyword
          )
        case .disabled:
          users = []
        }
        guard !Task.isCancelled,
          revision == requestRevision,
          self.conversationID == conversationID
        else {
          return
        }
        candidates = users
        selectedIndex = 0
      } catch {
        guard !Task.isCancelled, revision == requestRevision else {
          return
        }
        candidates = []
        selectedIndex = 0
      }
    }
  }

  private func resolveMentionNames(in text: String) {
    guard let account else {
      return
    }
    namesTask?.cancel()
    namesTask = Task {
      guard let names = try? await account.mentionNames(contents: [text]),
        !Task.isCancelled
      else {
        return
      }
      knownMentionIdentityNumbers.formUnion(names.keys)
    }
  }

  private static func mentionMatch(
    in text: String,
    selectionUTF16: Int
  ) -> (range: NSRange, keyword: String)? {
    let nsText = text as NSString
    guard selectionUTF16 >= 0, selectionUTF16 <= nsText.length else {
      return nil
    }
    let prefixRange = NSRange(location: 0, length: selectionUTF16)
    let pattern = try? NSRegularExpression(pattern: #"@(\S*)$"#)
    guard
      let match = pattern?.firstMatch(
        in: text,
        range: prefixRange
      ), match.range.location != NSNotFound
    else {
      return nil
    }
    return (
      match.range,
      nsText.substring(with: match.range(at: 1))
    )
  }
}

struct MentionInsertion {
  let text: String
  let selectionUTF16: Int
}

private struct MentionSelectionRequest: Equatable {
  let offset: Int
  let revision: Int
}

private struct MentionCandidatePanel: View {
  let users: [SwiftConversationParticipantItem]
  let keyword: String
  let selectedIndex: Int
  let onHover: (Int) -> Void
  let onSelect: (Int?) -> Void

  var body: some View {
    ScrollViewReader { proxy in
      ScrollView {
        LazyVStack(spacing: 0) {
          ForEach(Array(users.enumerated()), id: \.element.userId) { index, user in
            Button {
              onSelect(index)
            } label: {
              HStack(spacing: 8) {
                MentionAvatar(user: user)
                VStack(alignment: .leading, spacing: 2) {
                  MentionHighlightedText(
                    value: user.fullName,
                    keyword: keyword,
                    font: .systemFont(ofSize: 14)
                  )
                  MentionHighlightedText(
                    value: user.identityNumber,
                    keyword: keyword,
                    font: .systemFont(ofSize: 12),
                    secondary: true
                  )
                }
                Spacer()
              }
              .padding(8)
              .frame(height: 50)
              .contentShape(Rectangle())
              .background(
                index == selectedIndex
                  ? Color.accentColor.opacity(0.14)
                  : Color.clear)
            }
            .buttonStyle(.plain)
            .onHover { hovering in
              if hovering {
                onHover(index)
              }
            }
            .id(index)
          }
        }
      }
      .onChange(of: selectedIndex) {
        withAnimation(.easeIn(duration: 0.15)) {
          proxy.scrollTo(selectedIndex, anchor: .center)
        }
      }
    }
    .background(.regularMaterial)
    .clipShape(RoundedRectangle(cornerRadius: 6))
    .overlay {
      RoundedRectangle(cornerRadius: 6)
        .stroke(Color(nsColor: .separatorColor))
    }
    .shadow(radius: 8, y: 3)
    .accessibilityLabel("Mention suggestions")
  }
}

private struct MentionAvatar: View {
  let user: SwiftConversationParticipantItem

  var body: some View {
    MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
      image.resizable().scaledToFill()
    } placeholder: {
      ZStack {
        Circle().fill(Color.accentColor.opacity(0.16))
        Text(user.fullName.first.map(String.init) ?? "?")
          .font(.caption.weight(.semibold))
      }
    }
    .frame(width: 32, height: 32)
    .clipShape(Circle())
  }
}

private struct MentionHighlightedText: View {
  let value: String
  let keyword: String
  let font: NSFont
  var secondary = false

  var body: some View {
    Text(attributedValue)
      .lineLimit(1)
  }

  private var attributedValue: AttributedString {
    let result = NSMutableAttributedString(
      string: value,
      attributes: [
        .font: font,
        .foregroundColor: secondary
          ? NSColor.secondaryLabelColor
          : NSColor.labelColor,
      ]
    )
    if !keyword.isEmpty {
      let range = (value as NSString).range(
        of: keyword,
        options: [.caseInsensitive, .diacriticInsensitive]
      )
      if range.location != NSNotFound {
        result.addAttribute(
          .foregroundColor,
          value: NSColor.controlAccentColor,
          range: range
        )
      }
    }
    return AttributedString(result)
  }
}

private struct MentionTextEditor: NSViewRepresentable {
  @Binding var text: String
  @Binding var height: CGFloat
  let highlightedIdentities: Set<String>
  let candidatesVisible: Bool
  let selectionRequest: MentionSelectionRequest?
  let disabled: Bool
  let focusRevision: Int
  let onEditingChanged: (String, Int, Bool) -> Void
  let onMoveCandidate: (Int) -> Void
  let onSelectCandidate: (Int?) -> Void
  let onDismissCandidates: () -> Void
  let onPasteFiles: ([URL]) -> Void
  let onSubmit: () -> Void
  let onSubmitPost: () -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(parent: self)
  }

  func makeNSView(context: Context) -> MentionEditorScrollView {
    let scrollView = MentionEditorScrollView()
    let textView = MentionTextView()
    textView.delegate = context.coordinator
    textView.isRichText = false
    textView.importsGraphics = false
    textView.drawsBackground = false
    textView.isVerticallyResizable = true
    textView.isHorizontallyResizable = false
    textView.autoresizingMask = [.width]
    textView.textContainer?.widthTracksTextView = true
    textView.textContainer?.lineFragmentPadding = 0
    textView.textContainerInset = NSSize(width: 8, height: 8)
    textView.font = .systemFont(ofSize: 14)
    textView.textColor = .labelColor
    textView.insertionPointColor = .controlAccentColor
    textView.string = text
    textView.eventHandler = context.coordinator.handleKeyEvent
    textView.pasteHandler = context.coordinator.handlePaste

    scrollView.documentView = textView
    scrollView.hasVerticalScroller = true
    scrollView.autohidesScrollers = true
    scrollView.drawsBackground = false
    scrollView.backgroundColor = .clear
    scrollView.borderType = .noBorder
    scrollView.contentView.postsBoundsChangedNotifications = true
    scrollView.wantsLayer = true
    scrollView.layer?.cornerRadius = 5
    context.coordinator.textView = textView
    context.coordinator.appliedFocusRevision = focusRevision
    context.coordinator.applyMentionStyles()
    DispatchQueue.main.async {
      context.coordinator.updateHeight()
    }
    return scrollView
  }

  func updateNSView(_ scrollView: MentionEditorScrollView, context: Context) {
    context.coordinator.parent = self
    guard let textView = scrollView.documentView as? MentionTextView else {
      return
    }
    textView.isEditable = !disabled
    textView.pasteHandler = context.coordinator.handlePaste
    if textView.string != text {
      context.coordinator.isApplyingUpdate = true
      textView.string = text
      context.coordinator.isApplyingUpdate = false
    }
    if let selectionRequest,
      context.coordinator.appliedSelectionRevision != selectionRequest.revision
    {
      let offset = min(max(selectionRequest.offset, 0), (textView.string as NSString).length)
      textView.setSelectedRange(NSRange(location: offset, length: 0))
      textView.scrollRangeToVisible(NSRange(location: offset, length: 0))
      context.coordinator.appliedSelectionRevision = selectionRequest.revision
    }
    if context.coordinator.appliedFocusRevision != focusRevision {
      let offset = (textView.string as NSString).length
      textView.setSelectedRange(NSRange(location: offset, length: 0))
      textView.scrollRangeToVisible(NSRange(location: offset, length: 0))
      textView.window?.makeFirstResponder(textView)
      context.coordinator.appliedFocusRevision = focusRevision
    }
    context.coordinator.applyMentionStyles()
    context.coordinator.updateHeight()
  }

  final class Coordinator: NSObject, NSTextViewDelegate {
    var parent: MentionTextEditor
    weak var textView: MentionTextView?
    var isApplyingUpdate = false
    var appliedSelectionRevision = -1
    var appliedFocusRevision = -1

    init(parent: MentionTextEditor) {
      self.parent = parent
    }

    func textDidChange(_ notification: Notification) {
      guard !isApplyingUpdate, let textView else {
        return
      }
      parent.text = textView.string
      applyMentionStyles()
      publishEditingState()
      updateHeight()
    }

    func textViewDidChangeSelection(_ notification: Notification) {
      guard !isApplyingUpdate else {
        return
      }
      publishEditingState()
    }

    func handleKeyEvent(_ event: NSEvent) -> Bool {
      guard let textView else {
        return false
      }
      let candidateVisible = parent.candidatesVisible
      let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
      let composing = textView.hasMarkedText()

      if !composing {
        if [36, 76].contains(event.keyCode) {
          if candidateVisible {
            parent.onSelectCandidate(nil)
            return true
          }
          if modifiers.contains([.command, .shift]) {
            parent.onSubmitPost()
            return true
          }
          if !modifiers.contains(.shift) {
            parent.onSubmit()
            return true
          }
        }
        switch event.keyCode {
        case 125 where candidateVisible:
          parent.onMoveCandidate(1)
          return true
        case 126 where candidateVisible:
          parent.onMoveCandidate(-1)
          return true
        case 48 where candidateVisible:
          parent.onMoveCandidate(1)
          return true
        case 53 where candidateVisible:
          parent.onDismissCandidates()
          return true
        case 45 where candidateVisible && modifiers.contains(.control):
          parent.onMoveCandidate(1)
          return true
        case 35 where candidateVisible && modifiers.contains(.control):
          parent.onMoveCandidate(-1)
          return true
        default:
          break
        }
      }
      return false
    }

    func handlePaste(_ pasteboard: NSPasteboard) -> Bool {
      let files = ClipboardAttachmentReader.files(from: pasteboard)
      guard !files.isEmpty else {
        return false
      }
      parent.onPasteFiles(files)
      return true
    }

    func applyMentionStyles() {
      guard let textView else {
        return
      }
      let fullRange = NSRange(location: 0, length: textView.textStorage?.length ?? 0)
      textView.textStorage?.setAttributes(
        [
          .font: NSFont.systemFont(ofSize: 14),
          .foregroundColor: NSColor.labelColor,
        ],
        range: fullRange
      )
      guard let expression = try? NSRegularExpression(pattern: #"@(\d{4,})"#) else {
        return
      }
      for match in expression.matches(in: textView.string, range: fullRange) {
        let identity = (textView.string as NSString).substring(with: match.range(at: 1))
        if parent.highlightedIdentities.contains(identity) {
          textView.textStorage?.addAttribute(
            .foregroundColor,
            value: NSColor.controlAccentColor,
            range: match.range
          )
        }
      }
    }

    private func publishEditingState() {
      guard let textView else {
        return
      }
      parent.onEditingChanged(
        textView.string,
        textView.selectedRange().location,
        textView.hasMarkedText()
      )
    }

    func updateHeight() {
      guard let textView,
        let layoutManager = textView.layoutManager,
        let textContainer = textView.textContainer
      else {
        return
      }
      layoutManager.ensureLayout(for: textContainer)
      let contentHeight = ceil(
        layoutManager.usedRect(for: textContainer).height
          + textView.textContainerInset.height * 2
      )
      let height = min(max(contentHeight, 40), 136)
      guard abs(parent.height - height) > 0.5 else {
        return
      }
      DispatchQueue.main.async { [weak self] in
        guard let self, abs(self.parent.height - height) > 0.5 else {
          return
        }
        self.parent.height = height
      }
    }
  }
}

private final class MentionEditorScrollView: NSScrollView {}

private final class MentionTextView: NSTextView {
  var eventHandler: ((NSEvent) -> Bool)?
  var pasteHandler: ((NSPasteboard) -> Bool)?

  override func keyDown(with event: NSEvent) {
    if eventHandler?(event) == true {
      return
    }
    super.keyDown(with: event)
  }

  override func paste(_ sender: Any?) {
    if pasteHandler?(.general) == true {
      return
    }
    super.paste(sender)
  }
}

private enum ClipboardAttachmentReader {
  private struct ImageType {
    let pasteboardType: NSPasteboard.PasteboardType
    let extensionName: String
  }

  nonisolated private static let imageTypes = [
    ImageType(pasteboardType: .png, extensionName: "png"),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("public.jpeg"),
      extensionName: "jpg"
    ),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("com.compuserve.gif"),
      extensionName: "gif"
    ),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("org.webmproject.webp"),
      extensionName: "webp"
    ),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("com.microsoft.bmp"),
      extensionName: "bmp"
    ),
    ImageType(pasteboardType: .tiff, extensionName: "png"),
  ]

  static func files(from pasteboard: NSPasteboard) -> [URL] {
    let urls =
      (pasteboard.readObjects(
        forClasses: [NSURL.self],
        options: [.urlReadingFileURLsOnly: true]
      ) ?? [])
      .compactMap { object in
        (object as? NSURL).map { $0 as URL }
      }
      .filter(\.isFileURL)
      .filter { FileManager.default.fileExists(atPath: $0.path) }
    if !urls.isEmpty {
      return urls
    }

    return pasteboard.pasteboardItems?.compactMap(writeImage) ?? []
  }

  private nonisolated static func writeImage(_ item: NSPasteboardItem) -> URL? {
    for type in imageTypes {
      guard let data = item.data(forType: type.pasteboardType) else {
        continue
      }
      let output: Data
      if type.pasteboardType == .tiff {
        guard let bitmap = NSBitmapImageRep(data: data),
          let png = bitmap.representation(using: .png, properties: [:])
        else {
          continue
        }
        output = png
      } else {
        output = data
      }
      let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("mixin-paste-\(UUID().uuidString)")
        .appendingPathExtension(type.extensionName)
      do {
        try output.write(to: url, options: .atomic)
        return url
      } catch {
        continue
      }
    }
    return nil
  }
}
