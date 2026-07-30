import AppKit
import SwiftUI

struct MentionComposer: View {
  @Environment(\.colorScheme) private var colorScheme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion

  let model: MentionComposerModel
  let account: SwiftAccountHandle
  let conversationID: String
  let conversationCategory: String?
  let conversationOwnerID: String?
  let currentUserID: String
  let encrypted: Bool
  @Binding var text: String
  let disabled: Bool
  let focusRevision: Int
  let selectionRequest: ComposerSelectionRequest?
  let onSelectCandidate: (Int?) -> Void
  let onPasteFiles: ([URL]) -> Void
  let onSubmit: () -> Void
  let onSubmitPost: () -> Void

  @State private var editorHeight: CGFloat = 40

  var body: some View {
    AppComposerTextView(
      text: $text,
      height: $editorHeight,
      placeholder: encrypted ? "End-to-end encrypted" : "Type message",
      textColor: textColor,
      placeholderColor: placeholderColor,
      accentColor: accentColor,
      highlightedIdentities: model.knownMentionIdentityNumbers,
      candidatesVisible: !model.candidates.isEmpty,
      selectionRequest: selectionRequest,
      disabled: disabled,
      focusRevision: focusRevision,
      onEditingChanged: model.updateInput,
      onMoveCandidate: model.moveSelection,
      onSelectCandidate: onSelectCandidate,
      onDismissCandidates: model.dismiss,
      onPasteFiles: onPasteFiles,
      onSubmit: onSubmit,
      onSubmitPost: onSubmitPost
    )
    .frame(height: editorHeight)
    .background(inputBackground, in: RoundedRectangle(cornerRadius: 4))
    .animation(
      reduceMotion ? nil : .easeOut(duration: 0.16),
      value: editorHeight
    )
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

  private var textColor: NSColor {
    colorScheme == .dark
      ? NSColor(white: 1, alpha: 0.9)
      : NSColor(red: 51 / 255, green: 51 / 255, blue: 51 / 255, alpha: 1)
  }

  private var placeholderColor: NSColor {
    colorScheme == .dark
      ? NSColor(white: 1, alpha: 0.4)
      : NSColor(red: 184 / 255, green: 189 / 255, blue: 199 / 255, alpha: 1)
  }

  private var accentColor: NSColor {
    colorScheme == .dark
      ? NSColor(red: 65 / 255, green: 145 / 255, blue: 1, alpha: 1)
      : NSColor(red: 61 / 255, green: 117 / 255, blue: 227 / 255, alpha: 1)
  }

  private var configurationID: String {
    [
      conversationID,
      conversationCategory ?? "",
      conversationOwnerID ?? "",
      currentUserID,
    ].joined(separator: ":")
  }
}
