import SwiftUI

struct MixinSearchField: View {
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @Environment(\.mixinTheme) private var theme
  @Binding var text: String
  let focus: FocusState<Bool>.Binding
  let placeholder: String
  var shortcutHint: String?
  var onSubmit: () -> Void = {}
  var onExit: (() -> Void)?

  var body: some View {
    HStack(spacing: 8) {
      Image(systemName: "magnifyingglass")
        .foregroundStyle(theme.secondaryText)

      TextField(placeholder, text: $text)
        .textFieldStyle(.plain)
        .focused(focus)
        .onSubmit(onSubmit)
        .onExitCommand {
          if let onExit {
            onExit()
          } else if text.isEmpty {
            focus.wrappedValue = false
          } else {
            text = ""
          }
        }

      if text.isEmpty, !focus.wrappedValue, let shortcutHint {
        Text(shortcutHint)
          .font(.caption)
          .foregroundStyle(theme.secondaryText)
          .transition(.opacity)
      }

      if !text.isEmpty {
        Button {
          text = ""
        } label: {
          Image(systemName: "xmark.circle.fill")
            .foregroundStyle(theme.secondaryText)
        }
        .buttonStyle(.plain)
        .help("Clear")
        .transition(.scale.combined(with: .opacity))
      }
    }
    .padding(.horizontal, 14)
    .frame(height: 36)
    .background(theme.background, in: Capsule())
    .overlay {
      if focus.wrappedValue {
        Capsule()
          .stroke(theme.accent.opacity(0.55), lineWidth: 1)
      }
    }
    .contentShape(Capsule())
    .onTapGesture {
      focus.wrappedValue = true
    }
    .animation(
      reduceMotion ? nil : .easeOut(duration: 0.14),
      value: text.isEmpty
    )
    .animation(
      reduceMotion ? nil : .easeOut(duration: 0.14),
      value: focus.wrappedValue
    )
  }
}
