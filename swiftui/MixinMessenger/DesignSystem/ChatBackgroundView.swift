import SwiftUI

struct ChatBackgroundView: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        ZStack {
            theme.chatBackground
            Image("ChatBackground")
                .resizable()
                .renderingMode(.template)
                .foregroundStyle(
                    colorScheme == .dark
                        ? Color.white.opacity(0.02)
                        : Color.black.opacity(0.03)
                )
                .scaledToFill()
        }
        .clipped()
        .accessibilityHidden(true)
    }
}
