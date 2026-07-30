import SwiftUI

struct TextMessageView: View {
    let message: MessageItem
    let mentionNames: [String: String]
    let fontSize: CGFloat

    var body: some View {
        MessageRichText(
            content: message.displayText,
            baseFontSize: fontSize,
            mentionNames: mentionNames
        )
    }
}

struct RecallMessageView: View {
    @Environment(SettingsPreferencesModel.self) private var preferences
    @Environment(\.mixinTheme) private var theme

    let outgoing: Bool
    let recalledText: String?
    let onReedit: (String) -> Void

    var body: some View {
        HStack(spacing: 4) {
            Image("Recall")
                .renderingMode(.template)
                .resizable()
                .frame(width: 16, height: 16)
                .foregroundStyle(theme.secondaryText)
            Text(outgoing
                ? "You deleted this message"
                : "This message was deleted")
                .font(.system(size: 16 + preferences.chatFontSizeDelta))
                .foregroundStyle(theme.text)
            if let recalledText {
                Button("Re-edit") {
                    onReedit(recalledText)
                }
                .buttonStyle(.plain)
                .font(.system(size: 16 + preferences.chatFontSizeDelta))
                .foregroundStyle(theme.accent)
            }
        }
    }
}

struct SystemConversationMessageView: View {
    let text: String
    let fontSize: CGFloat

    var body: some View {
        Text(text)
            .font(.system(size: fontSize))
            .foregroundStyle(Color.black)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(
                Color(red: 202 / 255, green: 234 / 255, blue: 201 / 255),
                in: RoundedRectangle(cornerRadius: 10)
            )
    }
}

struct SystemFallbackMessageView: View {
    let message: MessageItem

    var body: some View {
        Text(
            message.action
                ?? message.participantFullName
                ?? (message.content.isEmpty ? message.category : message.content)
        )
        .foregroundStyle(.secondary)
    }
}
