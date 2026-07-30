import AppKit
import SwiftUI

struct ComposerEditingState: Equatable {
  let text: String
  let selectionUTF16: Int
  let hasMarkedText: Bool
}

struct ComposerSelectionRequest: Equatable {
  let offset: Int
  let revision: Int
}

struct AppComposerTextView: NSViewRepresentable {
  @Binding var text: String
  @Binding var height: CGFloat

  let placeholder: String
  let textColor: NSColor
  let placeholderColor: NSColor
  let accentColor: NSColor
  let highlightedIdentities: Set<String>
  let candidatesVisible: Bool
  let selectionRequest: ComposerSelectionRequest?
  let disabled: Bool
  let focusRevision: Int
  let onEditingChanged: (ComposerEditingState) -> Void
  let onMoveCandidate: (Int) -> Void
  let onSelectCandidate: (Int?) -> Void
  let onDismissCandidates: () -> Void
  let onPasteFiles: ([URL]) -> Void
  let onSubmit: () -> Void
  let onSubmitPost: () -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(parent: self)
  }

  func makeNSView(context: Context) -> AppComposerScrollView {
    let scrollView = AppComposerScrollView()
    let textView = AppComposerNSTextView()

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
    textView.textColor = textColor
    textView.insertionPointColor = accentColor
    textView.placeholder = placeholder
    textView.placeholderColor = placeholderColor
    textView.string = text
    textView.eventHandler = context.coordinator.handleKeyEvent
    textView.pasteHandler = context.coordinator.handlePaste

    scrollView.documentView = textView
    AppScrollViewStyle.apply(to: scrollView)
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

  func updateNSView(_ scrollView: AppComposerScrollView, context: Context) {
    context.coordinator.parent = self
    guard let textView = scrollView.documentView as? AppComposerNSTextView else {
      AppLogger.error(
        "Update composer text view failed: unexpected document view"
          + " type=\(String(describing: type(of: scrollView.documentView)))"
      )
      return
    }

    textView.isEditable = !disabled
    textView.placeholder = placeholder
    textView.textColor = textColor
    textView.insertionPointColor = accentColor
    textView.placeholderColor = placeholderColor
    textView.pasteHandler = context.coordinator.handlePaste

    if textView.hasMarkedText() {
      context.coordinator.logCompositionStateIfNeeded(source: "updateNSView")
      context.coordinator.logDeferredUpdateIfNeeded()
      context.coordinator.updateHeight()
      return
    }

    context.coordinator.logCompositionStateIfNeeded(source: "updateNSView")
    if textView.string != text {
      context.coordinator.isApplyingUpdate = true
      textView.string = text
      context.coordinator.isApplyingUpdate = false
    }
    if let selectionRequest,
      context.coordinator.appliedSelectionRevision != selectionRequest.revision
    {
      let textLength = (textView.string as NSString).length
      let offset = min(max(selectionRequest.offset, 0), textLength)
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
    private static let maximumTextLength = 64 * 1024
    private static let mentionExpression: NSRegularExpression? = {
      do {
        return try NSRegularExpression(pattern: #"@(\d{4,})"#)
      } catch {
        AppLogger.error("Compile composer mention expression failed", error: error)
        return nil
      }
    }()

    var parent: AppComposerTextView
    weak var textView: AppComposerNSTextView?
    var isApplyingUpdate = false
    var appliedSelectionRevision = -1
    var appliedFocusRevision = -1
    var wasComposing = false

    init(parent: AppComposerTextView) {
      self.parent = parent
    }

    func textDidChange(_ notification: Notification) {
      guard !isApplyingUpdate, let textView else {
        return
      }
      let composing = textView.hasMarkedText()
      logCompositionStateIfNeeded(source: "textDidChange")
      parent.text = textView.string
      if !composing {
        applyMentionStyles()
      }
      publishEditingState()
      updateHeight()
    }

    func textViewDidChangeSelection(_ notification: Notification) {
      guard !isApplyingUpdate else {
        return
      }
      publishEditingState()
    }

    func textView(
      _ textView: NSTextView,
      shouldChangeTextIn affectedCharRange: NSRange,
      replacementString: String?
    ) -> Bool {
      guard let replacementString else {
        return true
      }
      let nextLength =
        (textView.string as NSString).length
        - affectedCharRange.length
        + (replacementString as NSString).length
      return nextLength <= Self.maximumTextLength
    }

    func handleKeyEvent(_ event: NSEvent) -> Bool {
      guard let textView else {
        AppLogger.error("Handle composer key event failed: text view unavailable")
        return false
      }

      let modifiers = event.modifierFlags.intersection(
        .deviceIndependentFlagsMask
      )
      guard !textView.hasMarkedText() else {
        return false
      }

      if [36, 76].contains(event.keyCode) {
        if parent.candidatesVisible {
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
      case 125 where parent.candidatesVisible:
        parent.onMoveCandidate(1)
        return true
      case 126 where parent.candidatesVisible:
        parent.onMoveCandidate(-1)
        return true
      case 48 where parent.candidatesVisible:
        parent.onMoveCandidate(1)
        return true
      case 53 where parent.candidatesVisible:
        parent.onDismissCandidates()
        return true
      case 45 where parent.candidatesVisible && modifiers.contains(.control):
        parent.onMoveCandidate(1)
        return true
      case 35 where parent.candidatesVisible && modifiers.contains(.control):
        parent.onMoveCandidate(-1)
        return true
      default:
        return false
      }
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
      guard !textView.hasMarkedText() else {
        AppLogger.error(
          "Composer attempted to style text while IME composition is active"
        )
        return
      }

      let fullRange = NSRange(
        location: 0,
        length: textView.textStorage?.length ?? 0
      )
      textView.textStorage?.setAttributes(
        [
          .font: NSFont.systemFont(ofSize: 14),
          .foregroundColor: parent.textColor,
        ],
        range: fullRange
      )

      guard let expression = Self.mentionExpression else {
        return
      }
      for match in expression.matches(in: textView.string, range: fullRange) {
        let identity = (textView.string as NSString).substring(
          with: match.range(at: 1)
        )
        guard parent.highlightedIdentities.contains(identity) else {
          continue
        }
        textView.textStorage?.addAttribute(
          .foregroundColor,
          value: parent.accentColor,
          range: match.range
        )
      }
    }

    func logCompositionStateIfNeeded(source: String) {
      guard let textView else {
        return
      }
      let composing = textView.hasMarkedText()
      guard composing != wasComposing else {
        return
      }
      wasComposing = composing
      AppLogger.debug(
        "Composer IME composition \(composing ? "started" : "ended")"
          + " source=\(source)"
          + " marked_range=\(NSStringFromRange(textView.markedRange()))"
          + " selection=\(NSStringFromRange(textView.selectedRange()))"
          + " text_length=\((textView.string as NSString).length)"
      )
    }

    func logDeferredUpdateIfNeeded() {
      guard let textView else {
        return
      }
      let textChanged = textView.string != parent.text
      let selectionChanged =
        parent.selectionRequest.map {
          appliedSelectionRevision != $0.revision
        } ?? false
      let focusChanged = appliedFocusRevision != parent.focusRevision
      guard textChanged || selectionChanged || focusChanged else {
        return
      }
      AppLogger.debug(
        "Composer deferred SwiftUI update during IME composition"
          + " text_changed=\(textChanged)"
          + " selection_changed=\(selectionChanged)"
          + " focus_changed=\(focusChanged)"
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
      let glyphHeight = ceil(layoutManager.usedRect(for: textContainer).height)
      let lineHeight = ceil(
        layoutManager.defaultLineHeight(
          for: textView.font ?? NSFont.systemFont(ofSize: 14)
        )
      )
      let verticalInset = glyphHeight <= lineHeight ? (40 - lineHeight) / 2 : 8
      if abs(textView.textContainerInset.height - verticalInset) > 0.5 {
        textView.textContainerInset = NSSize(width: 8, height: verticalInset)
        textView.needsDisplay = true
      }

      let contentHeight = ceil(glyphHeight + verticalInset * 2)
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

    private func publishEditingState() {
      guard let textView else {
        return
      }
      parent.onEditingChanged(
        ComposerEditingState(
          text: textView.string,
          selectionUTF16: textView.selectedRange().location,
          hasMarkedText: textView.hasMarkedText()
        )
      )
    }
  }
}

final class AppComposerScrollView: NSScrollView {}

final class AppComposerNSTextView: NSTextView {
  var eventHandler: ((NSEvent) -> Bool)?
  var pasteHandler: ((NSPasteboard) -> Bool)?
  var placeholder = "" {
    didSet {
      needsDisplay = true
    }
  }
  var placeholderColor = NSColor.tertiaryLabelColor {
    didSet {
      needsDisplay = true
    }
  }

  override func draw(_ dirtyRect: NSRect) {
    super.draw(dirtyRect)
    guard string.isEmpty, !hasMarkedText(), !placeholder.isEmpty else {
      return
    }
    (placeholder as NSString).draw(
      at: textContainerOrigin,
      withAttributes: [
        .font: font ?? NSFont.systemFont(ofSize: 14),
        .foregroundColor: placeholderColor,
      ]
    )
  }

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
