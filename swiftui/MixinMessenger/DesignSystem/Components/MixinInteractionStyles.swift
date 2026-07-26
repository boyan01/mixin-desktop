import SwiftUI

struct MixinActionButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        MixinActionButtonBody(configuration: configuration)
    }
}

private struct MixinActionButtonBody: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.isEnabled) private var isEnabled
    @Environment(\.mixinTheme) private var theme
    @State private var hovering = false

    let configuration: ButtonStyle.Configuration

    var body: some View {
        configuration.label
            .frame(width: 36, height: 36)
            .contentShape(Circle())
            .background(background, in: Circle())
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.94 : 1)
            .opacity(isEnabled ? 1 : 0.4)
            .animation(
                reduceMotion ? nil : .easeOut(duration: 0.12),
                value: configuration.isPressed
            )
            .animation(
                reduceMotion ? nil : .easeOut(duration: hovering ? 0.12 : 0.06),
                value: hovering
            )
            .onHover { hovering = $0 }
    }

    private var background: Color {
        if configuration.isPressed {
            return theme.icon.opacity(0.12)
        }
        if hovering {
            return theme.icon.opacity(0.07)
        }
        return .clear
    }
}

struct MixinActionIcon: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.mixinTheme) private var theme
    @State private var hovering = false

    let systemName: String
    var selected = false

    var body: some View {
        Image(systemName: systemName)
            .foregroundStyle(selected ? theme.accent : theme.icon)
            .frame(width: 36, height: 36)
            .contentShape(Circle())
            .background(
                hovering ? theme.icon.opacity(0.07) : .clear,
                in: Circle()
            )
            .animation(
                reduceMotion ? nil : .easeOut(duration: hovering ? 0.12 : 0.06),
                value: hovering
            )
            .onHover { hovering = $0 }
    }
}

struct MixinRowButtonStyle: ButtonStyle {
    let selected: Bool

    func makeBody(configuration: Configuration) -> some View {
        MixinRowButtonBody(
            configuration: configuration,
            selected: selected
        )
    }
}

private struct MixinRowButtonBody: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.mixinTheme) private var theme
    @State private var hovering = false

    let configuration: ButtonStyle.Configuration
    let selected: Bool

    var body: some View {
        configuration.label
            .frame(maxWidth: .infinity)
            .contentShape(Rectangle())
            .background(background, in: RoundedRectangle(cornerRadius: 8))
            .animation(
                reduceMotion ? nil : .easeOut(duration: hovering ? 0.12 : 0.06),
                value: hovering
            )
            .animation(
                reduceMotion ? nil : .easeOut(duration: 0.12),
                value: configuration.isPressed
            )
            .animation(
                reduceMotion ? nil : .easeInOut(duration: 0.16),
                value: selected
            )
            .onHover { hovering = $0 }
    }

    private var background: Color {
        if selected {
            return theme.listSelected
        }
        if configuration.isPressed {
            return theme.icon.opacity(0.1)
        }
        if hovering {
            return theme.icon.opacity(0.045)
        }
        return .clear
    }
}
