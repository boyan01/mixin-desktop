import SwiftUI

struct PostMessageView: View {
    @Environment(SettingsPreferencesModel.self) private var preferences
    @Environment(\.mixinTheme) private var theme

    let message: MessageItem
    var minimumHeight: CGFloat = 48
    var contentPadding: CGFloat = 0
    var background: Color?
    let onOpen: () -> Void

    var body: some View {
        ZStack(alignment: .topTrailing) {
            Text(message.postAttributedText)
                .font(.system(size: 16 + preferences.chatFontSizeDelta))
                .foregroundStyle(theme.text)
                .frame(
                    minWidth: 128,
                    maxWidth: 400,
                    minHeight: minimumHeight,
                    maxHeight: 400,
                    alignment: .topLeading
                )
                .clipped()

            Image(systemName: "arrow.up.left.and.arrow.down.right")
                .font(.system(size: 20))
                .foregroundStyle(.white)
                .frame(width: 20, height: 20)
                .background(.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 8))
        }
        .padding(contentPadding)
        .background {
            if let background {
                RoundedRectangle(cornerRadius: 8).fill(background)
            }
        }
        .frame(maxWidth: 400, alignment: .leading)
        .contentShape(Rectangle())
        .onTapGesture(perform: onOpen)
    }
}
