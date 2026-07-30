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
    HStack(spacing: 0) {
      Image(systemName: "magnifyingglass")
        .padding(.leading, 16)
        .padding(.trailing, 8)
        .foregroundStyle(theme.secondaryText)

      TextField(placeholder, text: $text)
        .textFieldStyle(.plain)
        .font(.system(size: 14))
        .foregroundStyle(theme.text)
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
          .font(.system(size: 14))
          .foregroundStyle(theme.secondaryText)
          .transition(.opacity)
      }

      if !text.isEmpty {
        Button {
          text = ""
        } label: {
          Image(systemName: "xmark")
            .font(.system(size: 16))
            .foregroundStyle(theme.secondaryText)
        }
        .buttonStyle(.plain)
        .padding(.leading, 8)
        .padding(.trailing, 16)
        .help("Clear")
        .transition(.scale.combined(with: .opacity))
      } else {
        Spacer()
          .frame(width: 40)
      }
    }
    .frame(height: 36)
    .background(theme.searchFieldBackground, in: Capsule())
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
