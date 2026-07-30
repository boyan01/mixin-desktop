import SwiftUI

struct SixDigitPasscodeField: View {
    @Environment(\.mixinTheme) private var theme
    @Binding var value: String
    let isFocused: FocusState<Bool>.Binding
    let onComplete: (String) -> Void

    var body: some View {
        ZStack {
            SecureField("", text: $value)
                .textFieldStyle(.plain)
                .focused(isFocused)
                .frame(width: 1, height: 1)
                .opacity(0.01)
                .onSubmit {
                    submitIfComplete()
                }

            HStack(spacing: 22.8) {
                ForEach(0 ..< 6, id: \.self) { index in
                    Circle()
                        .fill(index < value.count ? theme.text : Color.clear)
                        .overlay {
                            Circle()
                                .stroke(theme.text, lineWidth: 1)
                        }
                        .frame(width: 15, height: 15)
                }
            }
            .contentShape(Rectangle())
            .onTapGesture {
                isFocused.wrappedValue = true
            }
        }
        .frame(width: 204, height: 36)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Six digit passcode")
        .accessibilityValue("\(value.count) of 6 digits entered")
        .onChange(of: value) {
            let sanitized = String(value.filter(isPasscodeDigit).prefix(6))
            if sanitized != value {
                value = sanitized
                return
            }
            submitIfComplete()
        }
    }

    private func submitIfComplete() {
        guard value.count == 6 else {
            return
        }
        onComplete(value)
    }
}

func isPasscodeDigit(_ character: Character) -> Bool {
    let scalars = character.unicodeScalars
    guard scalars.count == 1, let value = scalars.first?.value else {
        return false
    }
    return (48 ... 57).contains(value)
}
