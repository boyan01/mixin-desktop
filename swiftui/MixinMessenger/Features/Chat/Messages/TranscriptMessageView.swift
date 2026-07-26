import SwiftUI

struct TranscriptMessageView: View {
    let message: SwiftMessageItem
    let onOpen: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label("Transcript", systemImage: "text.bubble")
                Spacer()
                Image(systemName: "arrow.up.left.and.arrow.down.right")
                    .foregroundStyle(.secondary)
            }
            VStack(alignment: .leading, spacing: 3) {
                ForEach(message.transcriptPreviewLines.prefix(4), id: \.self) { line in
                    Text(line)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(7)
            .background(.black.opacity(0.04), in: RoundedRectangle(cornerRadius: 6))
        }
        .frame(width: 260)
        .contentShape(Rectangle())
        .onTapGesture(perform: onOpen)
    }
}

extension SwiftMessageItem {
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
