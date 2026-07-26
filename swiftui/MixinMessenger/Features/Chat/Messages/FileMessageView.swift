import SwiftUI

struct FileMessageView: View {
    let message: SwiftMessageItem
    let outgoing: Bool
    let progress: () -> Double
    let onAttachmentAction: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            if message.mediaStatus.isComplete {
                Text(fileExtension)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .frame(width: 38, height: 38)
                    .background(.secondary.opacity(0.12), in: Circle())
            } else {
                AttachmentStatusOverlay(
                    message: message,
                    outgoing: outgoing,
                    progress: progress,
                    completedSymbol: nil
                )
                .frame(width: 38, height: 38)
            }

            VStack(alignment: .leading, spacing: 3) {
                Text(message.mediaName ?? "File")
                    .font(.system(size: 14))
                    .lineLimit(1)
                    .truncationMode(.middle)
                if let size = message.mediaSize, size > 0 {
                    Text(ByteCountFormatter.string(
                        fromByteCount: size,
                        countStyle: .file
                    ))
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                }
            }
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
        return value.isEmpty ? "FILE" : value.uppercased()
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
