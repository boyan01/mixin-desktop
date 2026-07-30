import SwiftUI

struct AppIconButton: View {
    private let icon: AppIcon
    private let selected: Bool
    private let help: String
    private let iconSize: CGFloat
    private let action: () -> Void

    init(
        systemName: String,
        selected: Bool = false,
        iconSize: CGFloat = 18,
        help: String,
        action: @escaping () -> Void
    ) {
        icon = .system(systemName)
        self.selected = selected
        self.iconSize = iconSize
        self.help = help
        self.action = action
    }

    init(
        assetName: String,
        selected: Bool = false,
        iconSize: CGFloat = 24,
        help: String,
        action: @escaping () -> Void
    ) {
        icon = .asset(assetName)
        self.selected = selected
        self.iconSize = iconSize
        self.help = help
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            AppIconButtonLabel(icon: icon, selected: selected, iconSize: iconSize)
        }
        .buttonStyle(.plain)
        .help(help)
        .accessibilityLabel(help)
    }
}

struct AppIconMenu<Content: View>: View {
    private let icon: AppIcon
    private let help: String
    private let iconSize: CGFloat
    private let content: Content

    init(
        systemName: String,
        iconSize: CGFloat = 18,
        help: String,
        @ViewBuilder content: () -> Content
    ) {
        icon = .system(systemName)
        self.iconSize = iconSize
        self.help = help
        self.content = content()
    }

    init(
        assetName: String,
        iconSize: CGFloat = 24,
        help: String,
        @ViewBuilder content: () -> Content
    ) {
        icon = .asset(assetName)
        self.iconSize = iconSize
        self.help = help
        self.content = content()
    }

    var body: some View {
        Menu {
            content
        } label: {
            AppIconButtonLabel(icon: icon, selected: false, iconSize: iconSize)
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .help(help)
        .accessibilityLabel(help)
    }
}

private enum AppIcon {
    case system(String)
    case asset(String)
}

private struct AppIconButtonLabel: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.mixinTheme) private var theme
    @State private var hovering = false

    let icon: AppIcon
    let selected: Bool
    let iconSize: CGFloat

    var body: some View {
        image
            .foregroundStyle(selected ? theme.accent : theme.icon)
            .frame(width: 40, height: 40)
            .contentShape(Circle())
            .background(
                hovering ? hoverColor : .clear,
                in: Circle()
            )
            .animation(
                reduceMotion ? nil : .easeOut(duration: hovering ? 0.12 : 0.06),
                value: hovering
            )
            .onHover { hovering = $0 }
    }

    @ViewBuilder
    private var image: some View {
        switch icon {
        case let .system(name):
            Image(systemName: name)
                .font(.system(size: iconSize, weight: .medium))
        case let .asset(name):
            Image(name)
                .resizable()
                .renderingMode(.template)
                .scaledToFit()
                .frame(width: iconSize, height: iconSize)
        }
    }

    private var hoverColor: Color {
        theme.icon.opacity(0.07)
    }
}
