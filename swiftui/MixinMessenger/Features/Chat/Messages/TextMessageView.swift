import SwiftUI

struct TextMessageView: View {
    let message: SwiftMessageItem
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
    let outgoing: Bool
    let recalledText: String?
    let onReedit: (String) -> Void

    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: "arrow.uturn.backward.circle")
                .foregroundStyle(.secondary)
            Text(outgoing
                ? "You deleted this message"
                : "This message was deleted")
            if let recalledText {
                Button("Re-edit") {
                    onReedit(recalledText)
                }
                .buttonStyle(.plain)
                .foregroundStyle(Color.accentColor)
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
    let message: SwiftMessageItem

    var body: some View {
        Text(
            message.action
                ?? message.participantFullName
                ?? (message.content.isEmpty ? message.category : message.content)
        )
        .foregroundStyle(.secondary)
    }
}
