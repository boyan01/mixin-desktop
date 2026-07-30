import SwiftUI

struct MixinTheme: Sendable {
    let primary: Color
    let accent: Color
    let text: Color
    let icon: Color
    let secondaryText: Color
    let statusBackground: Color
    let stickerPlaceholder: Color
    let searchFieldBackground: Color
    let dateTime: Color
    let highlight: Color
    let encrypt: Color
    let waveformBackground: Color
    let waveformForeground: Color
    let sidebarSelected: Color
    let listSelected: Color
    let chatBackground: Color
    let background: Color
    let divider: Color
    let popUp: Color
    let destructive: Color
    let success: Color
    let warning: Color
    let settingCellBackground: Color

    static let light = MixinTheme(
        primary: Color(red: 1, green: 1, blue: 1),
        accent: Color(red: 61 / 255, green: 117 / 255, blue: 227 / 255),
        text: Color(red: 51 / 255, green: 51 / 255, blue: 51 / 255),
        icon: Color(red: 47 / 255, green: 48 / 255, blue: 50 / 255),
        secondaryText: Color(red: 184 / 255, green: 189 / 255, blue: 199 / 255),
        statusBackground: Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255),
        stickerPlaceholder: Color(red: 236 / 255, green: 236 / 255, blue: 236 / 255),
        searchFieldBackground: Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255),
        dateTime: Color(red: 213 / 255, green: 211 / 255, blue: 243 / 255),
        highlight: Color(red: 167 / 255, green: 242 / 255, blue: 89 / 255),
        encrypt: Color(red: 255 / 255, green: 247 / 255, blue: 173 / 255),
        waveformBackground: Color(red: 221 / 255, green: 221 / 255, blue: 221 / 255),
        waveformForeground: Color(red: 155 / 255, green: 155 / 255, blue: 155 / 255),
        sidebarSelected: Color.black.opacity(0.08),
        listSelected: Color(red: 246 / 255, green: 247 / 255, blue: 250 / 255),
        chatBackground: Color(red: 237 / 255, green: 238 / 255, blue: 238 / 255),
        background: Color(red: 246 / 255, green: 247 / 255, blue: 250 / 255),
        divider: Color(red: 229 / 255, green: 231 / 255, blue: 235 / 255),
        popUp: .white,
        destructive: Color(red: 229 / 255, green: 120 / 255, blue: 116 / 255),
        success: Color(red: 80 / 255, green: 189 / 255, blue: 92 / 255),
        warning: Color(red: 244 / 255, green: 171 / 255, blue: 45 / 255),
        settingCellBackground: .white
    )

    static let dark = MixinTheme(
        primary: Color(red: 44 / 255, green: 49 / 255, blue: 54 / 255),
        accent: Color(red: 65 / 255, green: 145 / 255, blue: 1),
        text: Color.white.opacity(0.9),
        icon: Color.white.opacity(0.9),
        secondaryText: Color.white.opacity(0.4),
        statusBackground: Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255),
        stickerPlaceholder: Color(red: 40 / 255, green: 44 / 255, blue: 48 / 255),
        searchFieldBackground: Color.white.opacity(0.08),
        dateTime: Color(red: 213 / 255, green: 211 / 255, blue: 243 / 255),
        highlight: Color(red: 134 / 255, green: 184 / 255, blue: 82 / 255),
        encrypt: Color(red: 255 / 255, green: 247 / 255, blue: 173 / 255),
        waveformBackground: Color.white.opacity(0.4),
        waveformForeground: .white,
        sidebarSelected: Color.white.opacity(0.06),
        listSelected: Color.white.opacity(0.06),
        chatBackground: Color(red: 35 / 255, green: 39 / 255, blue: 43 / 255),
        background: Color(red: 40 / 255, green: 44 / 255, blue: 48 / 255),
        divider: Color.black.opacity(0.16),
        popUp: Color(red: 62 / 255, green: 65 / 255, blue: 72 / 255),
        destructive: Color(red: 246 / 255, green: 112 / 255, blue: 112 / 255),
        success: Color(red: 96 / 255, green: 209 / 255, blue: 108 / 255),
        warning: Color(red: 243 / 255, green: 177 / 255, blue: 64 / 255),
        settingCellBackground: Color.white.opacity(0.06)
    )
}

private struct MixinThemeKey: EnvironmentKey {
    static let defaultValue = MixinTheme.light
}

extension EnvironmentValues {
    var mixinTheme: MixinTheme {
        get { self[MixinThemeKey.self] }
        set { self[MixinThemeKey.self] = newValue }
    }
}

private struct MixinThemeModifier: ViewModifier {
    @Environment(\.colorScheme) private var colorScheme

    func body(content: Content) -> some View {
        let theme = colorScheme == .dark ? MixinTheme.dark : .light
        content
            .environment(\.mixinTheme, theme)
            .tint(theme.accent)
    }
}

extension View {
    func mixinTheme() -> some View {
        modifier(MixinThemeModifier())
    }
}
