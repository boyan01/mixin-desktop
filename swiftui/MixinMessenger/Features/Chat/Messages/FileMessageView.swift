import SwiftUI
import UniformTypeIdentifiers

struct FileMessageView: View {
    @Environment(\.mixinTheme) private var theme
    @Environment(SettingsPreferencesModel.self) private var preferences
    let message: MessageItem
    let outgoing: Bool
    let progress: () -> Double
    let onAttachmentAction: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            if message.mediaStatus.isComplete {
                Text(fileExtension)
                    .font(.system(size: 12))
                    .foregroundStyle(Color(red: 184 / 255, green: 189 / 255, blue: 199 / 255))
                    .frame(width: 38, height: 38)
                    .background(theme.statusBackground, in: Circle())
            } else {
                AttachmentStatusOverlay(
                    message: message,
                    outgoing: outgoing,
                    progress: progress,
                    completedSymbol: nil
                )
                .frame(width: 38, height: 38)
            }

            VStack(alignment: .leading, spacing: 0) {
                Text(message.mediaName ?? "")
                    .font(.system(size: 14 + preferences.chatFontSizeDelta))
                    .foregroundStyle(theme.text)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 0)
                Text(formattedSize)
                    .font(.system(size: 12 + preferences.chatFontSizeDelta))
                    .foregroundStyle(theme.secondaryText)
                    .lineLimit(1)
            }
            .frame(height: 38)
        }
        .contentShape(Rectangle())
        .onTapGesture {
            if message.mediaStatus.isComplete {
                openOrSave()
            } else {
                onAttachmentAction()
            }
        }
    }

    private var fileExtension: String {
        guard let name = message.mediaName else {
            return "FILE"
        }
        let value = URL(fileURLWithPath: name)
            .pathExtension
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !value.isEmpty, UTType(filenameExtension: value) != nil else {
            return "FILE"
        }
        return value.uppercased()
    }

    private var formattedSize: String {
        let bytes = max(0, message.mediaSize ?? 0)
        let units = ["B", "KB", "MB", "GB", "TB", "PB"]
        var unitIndex = 0
        var divisor: Int64 = 1
        while unitIndex < units.count - 1, bytes >= divisor * 1_024 {
            divisor *= 1_024
            unitIndex += 1
        }
        guard unitIndex > 0 else {
            return "\(bytes) B"
        }
        let value = Double(bytes) / Double(divisor)
        let digits = bytes.isMultiple(of: divisor) ? 0 : 2
        return String(format: "%.*f %@", digits, value, units[unitIndex])
    }

    private func openOrSave() {
        guard let url = message.localMediaURL else {
            return
        }
        if FileInteraction.shouldOpenDirectly(url) {
            MessageMediaInteraction.open(message)
        } else {
            MessageMediaInteraction.save(message)
        }
    }
}
