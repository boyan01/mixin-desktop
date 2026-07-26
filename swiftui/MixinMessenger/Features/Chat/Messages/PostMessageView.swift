import SwiftUI

struct PostMessageView: View {
    let message: SwiftMessageItem
    let onOpen: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label("Post", systemImage: "doc.richtext")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Image(systemName: "arrow.up.left.and.arrow.down.right")
                    .foregroundStyle(.secondary)
            }
            Text(message.postPreview)
                .lineLimit(10)
                .frame(maxWidth: 400, maxHeight: 360, alignment: .topLeading)
        }
        .contentShape(Rectangle())
        .onTapGesture(perform: onOpen)
    }
}
