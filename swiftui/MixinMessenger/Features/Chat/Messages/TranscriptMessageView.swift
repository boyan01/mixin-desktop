import SwiftUI

struct TranscriptMessageView: View {
    @Environment(SettingsPreferencesModel.self) private var preferences
    let message: MessageItem
    let onOpen: () -> Void
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                Spacer().frame(width: 4)
                Text("Transcript")
                    .font(.system(size: 16 + preferences.chatFontSizeDelta))
                    .foregroundStyle(theme.text)
                Spacer()
                Image("PostDetail")
                    .resizable()
                    .frame(width: 20, height: 20)
                    .background(.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 8))
            }
            .padding(.leading, 4)
            .padding(.bottom, 4)
            VStack(alignment: .leading, spacing: 3) {
                ForEach(message.transcriptPreviewLines.prefix(4), id: \.self) { line in
                    Text(line)
                        .font(.system(size: 12 + preferences.chatFontSizeDelta))
                        .foregroundStyle(theme.secondaryText)
                        .lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
            .background(.black.opacity(0.04), in: RoundedRectangle(cornerRadius: 6))
        }
        .frame(width: 260)
        .contentShape(Rectangle())
        .onTapGesture(perform: onOpen)
    }
}

extension MessageItem {
    func scaledMediaSize(
        maximum: CGSize,
        minimumWidth: CGFloat
    ) -> CGSize {
        let width = max(1, CGFloat(mediaWidth ?? 1))
        let height = max(1, CGFloat(mediaHeight ?? 1))
        let scale = min(maximum.width / width, maximum.height / height, 1)
        let scaledWidth = max(minimumWidth, width * scale)
        return CGSize(
            width: min(maximum.width, scaledWidth),
            height: min(maximum.height, max(120, scaledWidth * height / width))
        )
    }
}
