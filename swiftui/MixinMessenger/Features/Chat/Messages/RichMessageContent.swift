import AppKit
import AVKit
import Nuke
import SwiftUI

struct RichMessageContent: View {
    let message: SwiftMessageItem
    var mentionNames: [String: String] = [:]
    let outgoing: Bool
    let imageMessages: [SwiftMessageItem]
    let loadImageWindow: (String) async throws -> [SwiftImageMessageItem]
    let progress: () -> Double
    let onAttachmentAction: () -> Void
    let loadTranscript: () async throws -> [SwiftMessageItem]
    let onTranscriptAttachmentAction: (SwiftMessageItem) async -> Void
    var onAppAction: (String, String) -> Void = { action, _ in
        guard let url = URL(string: action),
              ["http", "https", "mixin"].contains(url.scheme?.lowercased() ?? "")
        else {
            return
        }
        NSWorkspace.shared.open(url)
    }

    @State private var preview: RichMessagePreview?

    var body: some View {
        content
            .sheet(item: $preview) { preview in
                switch preview {
                case .image:
                    ImageMessagePreview(
                        messages: imageMessages,
                        initialMessageID: message.messageId,
                        loadWindow: loadImageWindow
                    )
                case .video:
                    VideoMessagePreview(message: message)
                case .post:
                    PostMessagePreview(message: message)
                case .transcript:
                    TranscriptMessagePreview(
                        title: message.senderName,
                        sentByCurrentUser: outgoing,
                        mentionNames: mentionNames,
                        load: loadTranscript,
                        onAttachmentAction: onTranscriptAttachmentAction,
                        onAppAction: onAppAction
                    )
                }
            }
    }

    @ViewBuilder
    private var content: some View {
        if message.category.hasSuffix("_TRANSCRIPT") {
            TranscriptMessageView(message: message) {
                preview = .transcript
            }
        } else if message.category.hasSuffix("_IMAGE") {
            ImageMessageView(
                message: message,
                mentionNames: mentionNames,
                outgoing: outgoing,
                progress: progress,
                onAttachmentAction: onAttachmentAction
            ) {
                preview = .image
            }
        } else if message.category.hasSuffix("_VIDEO") || message.category.hasSuffix("_LIVE") {
            VideoMessageView(
                message: message,
                outgoing: outgoing,
                progress: progress,
                onAttachmentAction: onAttachmentAction
            ) {
                preview = .video
            }
        } else if message.category.hasSuffix("_DATA") {
            FileMessageView(
                message: message,
                outgoing: outgoing,
                progress: progress,
                onAttachmentAction: onAttachmentAction
            )
        } else if message.category.contains("POST") {
            PostMessageView(message: message) {
                preview = .post
            }
        } else {
            MessageRichText(
                content: message.content.isEmpty ? message.category : message.content,
                mentionNames: mentionNames
            )
        }
    }

}

enum MessageMediaInteraction {
    static func copyImage(_ message: SwiftMessageItem) {
        guard let url = message.localMediaURL,
              let image = NSImage(contentsOf: url)
        else {
            return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.writeObjects([image])
    }

    static func open(_ message: SwiftMessageItem) {
        guard let url = message.localMediaURL else {
            return
        }
        NSWorkspace.shared.open(url)
    }

    static func reveal(_ message: SwiftMessageItem) {
        guard let url = message.localMediaURL else {
            return
        }
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }

    static func save(_ message: SwiftMessageItem) {
        guard let url = message.localMediaURL else {
            return
        }
        Task {
            await FileInteraction.save(
                source: url,
                suggestedName: message.mediaName ?? url.lastPathComponent
            )
        }
    }
}

private enum RichMessagePreview: String, Identifiable {
    case image
    case video
    case post
    case transcript

    var id: String { rawValue }
}

struct AttachmentStatusOverlay: View {
    let message: SwiftMessageItem
    let outgoing: Bool
    let progress: () -> Double
    let completedSymbol: String?

    var body: some View {
        if message.mediaStatus.uppercased() == "PENDING" {
            TimelineView(.periodic(from: .now, by: 0.1)) { _ in
                status
            }
        } else {
            status
        }
    }

    @ViewBuilder
    private var status: some View {
        switch message.mediaStatus.uppercased() {
        case "PENDING":
            ZStack {
                Circle()
                    .stroke(.primary.opacity(0.2), lineWidth: 3)
                Circle()
                    .trim(from: 0, to: max(0, min(1, progress())))
                    .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 3, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                RoundedRectangle(cornerRadius: 1)
                    .fill(Color.accentColor)
                    .frame(width: 9, height: 9)
            }
            .frame(width: 38, height: 38)
            .background(.regularMaterial, in: Circle())
        case "CANCELED":
            Image(systemName: outgoing && message.mediaUrl?.isEmpty == false
                ? "arrow.up"
                : "arrow.down")
                .foregroundStyle(Color.accentColor)
                .frame(width: 38, height: 38)
                .background(.regularMaterial, in: Circle())
        case "EXPIRED":
            Image(systemName: "exclamationmark.triangle")
                .frame(width: 38, height: 38)
                .background(.regularMaterial, in: Circle())
        default:
            if let completedSymbol {
                Image(systemName: completedSymbol)
                    .foregroundStyle(.white)
                    .frame(width: 38, height: 38)
                    .background(.black.opacity(0.45), in: Circle())
            }
        }
    }
}

struct MessageMediaImage: View {
    let source: String?
    let thumbnail: String?
    let contentMode: ContentMode

    var body: some View {
        MixinAsyncImage(url: imageURL, maxPixelSize: 1_280) { phase in
            switch phase {
            case let .success(image):
                image.resizable()
            case .empty, .failure:
                thumbnailImage
            }
        }
        .aspectRatio(contentMode: contentMode)
    }

    private var imageURL: URL? {
        if let remoteURL = source?.httpURL {
            return remoteURL
        }
        return source?.localFileURL
    }

    @ViewBuilder
    private var thumbnailImage: some View {
        if let image = thumbnail?.embeddedImage {
            Image(nsImage: image)
                .resizable()
                .aspectRatio(contentMode: contentMode)
        } else {
            Rectangle()
                .fill(.secondary.opacity(0.18))
                .overlay {
                    Image(systemName: "photo")
                        .font(.title)
                        .foregroundStyle(.secondary)
                }
        }
    }
}

private struct ImageMessagePreview: View {
    @Environment(\.dismiss) private var dismiss
    @State private var messages: [ImagePreviewItem]
    let loadWindow: (String) async throws -> [SwiftImageMessageItem]

    @State private var index: Int
    @State private var zoom = 1.0
    @State private var rotation = Angle.zero
    @State private var image: NSImage?

    init(
        messages: [SwiftMessageItem],
        initialMessageID: String,
        loadWindow: @escaping (String) async throws -> [SwiftImageMessageItem]
    ) {
        let items = messages.map(ImagePreviewItem.init)
        _messages = State(initialValue: items)
        self.loadWindow = loadWindow
        _index = State(initialValue:
            items.firstIndex(where: { $0.messageID == initialMessageID }) ?? 0
        )
    }

    private var message: ImagePreviewItem? {
        messages.indices.contains(index) ? messages[index] : nil
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(message?.senderName ?? "Image")
                    .font(.headline)
                Spacer()
                Button {
                    select(index - 1)
                } label: {
                    Image(systemName: "chevron.left")
                }
                .disabled(index == 0)
                Button {
                    select(index + 1)
                } label: {
                    Image(systemName: "chevron.right")
                }
                .disabled(index + 1 >= messages.count)
                Button {
                    zoom = max(0.5, zoom * 0.8)
                } label: {
                    Image(systemName: "minus.magnifyingglass")
                }
                Button {
                    zoom = min(5, zoom * 1.25)
                } label: {
                    Image(systemName: "plus.magnifyingglass")
                }
                Button {
                    rotation += .degrees(90)
                } label: {
                    Image(systemName: "rotate.right")
                }
                Button {
                    copy()
                } label: {
                    Image(systemName: "doc.on.doc")
                }
                .disabled(image == nil)
                Button {
                    save()
                } label: {
                    Image(systemName: "square.and.arrow.down")
                }
                .disabled(message?.mediaURL.localFileURL == nil)
                Button("Done") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
            }
            .buttonStyle(.borderless)
            .padding()

            Divider()

            ScrollView([.horizontal, .vertical]) {
                Group {
                    if let image {
                        Image(nsImage: image)
                            .resizable()
                            .scaledToFit()
                    } else {
                        ProgressView()
                    }
                }
                .scaleEffect(zoom)
                .rotationEffect(rotation)
                .frame(minWidth: 600, minHeight: 440)
            }
            .background(Color.black.opacity(0.88))
        }
        .frame(minWidth: 720, minHeight: 540)
        .task(id: message?.messageID) {
            image = nil
            guard let message else {
                return
            }
            if index < 8 || index + 8 >= messages.count,
               let expanded = try? await loadWindow(message.messageID)
            {
                merge(expanded.map(ImagePreviewItem.init), keeping: message.messageID)
            }
            guard let url = message.mediaURL.localFileURL else {
                return
            }
            image = try? await ImagePipeline.shared.image(for: url)
        }
    }

    private func select(_ nextIndex: Int) {
        guard messages.indices.contains(nextIndex) else {
            return
        }
        index = nextIndex
        zoom = 1
        rotation = .zero
    }

    private func copy() {
        guard let image else {
            return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.writeObjects([image])
    }

    private func save() {
        guard let message, let source = message.mediaURL.localFileURL else {
            return
        }
        Task {
            await FileInteraction.save(
                source: source,
                suggestedName: message.mediaName ?? source.lastPathComponent
            )
        }
    }

    private func merge(_ loaded: [ImagePreviewItem], keeping messageID: String) {
        var byID = Dictionary(uniqueKeysWithValues: messages.map { ($0.messageID, $0) })
        for item in loaded {
            byID[item.messageID] = item
        }
        messages = byID.values.sorted {
            if $0.createdAtMicros == $1.createdAtMicros {
                return $0.messageID < $1.messageID
            }
            return $0.createdAtMicros < $1.createdAtMicros
        }
        index = messages.firstIndex(where: { $0.messageID == messageID }) ?? index
    }
}

private struct ImagePreviewItem {
    let messageID: String
    let createdAtMicros: Int64
    let mediaURL: String
    let mediaName: String?
    let senderName: String

    nonisolated init(_ message: SwiftMessageItem) {
        messageID = message.messageId
        createdAtMicros = message.createdAtMicros
        mediaURL = message.mediaUrl ?? ""
        mediaName = message.mediaName
        senderName = message.senderName
    }

    nonisolated init(_ message: SwiftImageMessageItem) {
        messageID = message.messageId
        createdAtMicros = message.createdAtMicros
        mediaURL = message.mediaUrl
        mediaName = message.mediaName
        senderName = message.userFullName
    }
}

private struct VideoMessagePreview: View {
    @Environment(\.dismiss) private var dismiss
    let message: SwiftMessageItem
    @State private var player: AVPlayer?

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(message.mediaName ?? "Video")
                    .font(.headline)
                Spacer()
                Button("Done") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
            }
            .padding()
            Divider()
            if let player {
                VideoPlayer(player: player)
                    .onAppear {
                        player.play()
                    }
            } else {
                ContentUnavailableView(
                    "Video unavailable",
                    systemImage: "play.slash",
                    description: Text("The local media file could not be resolved.")
                )
            }
        }
        .frame(minWidth: 720, minHeight: 500)
        .task {
            if let url = message.resolvedMediaURL {
                player = AVPlayer(url: url)
            }
        }
        .onDisappear {
            player?.pause()
        }
    }
}

private struct PostMessagePreview: View {
    @Environment(\.dismiss) private var dismiss
    let message: SwiftMessageItem

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(message.senderName)
                    .font(.headline)
                Spacer()
                Button("Done") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
            }
            .padding()
            Divider()
            ScrollView {
                Text(message.postAttributedText)
                    .textSelection(.enabled)
                    .frame(maxWidth: 640, alignment: .leading)
                    .padding(32)
            }
        }
        .frame(minWidth: 680, minHeight: 520)
    }
}

private struct TranscriptMessagePreview: View {
    @Environment(\.dismiss) private var dismiss
    let title: String
    let sentByCurrentUser: Bool
    let mentionNames: [String: String]
    let load: () async throws -> [SwiftMessageItem]
    let onAttachmentAction: (SwiftMessageItem) async -> Void
    let onAppAction: (String, String) -> Void

    @State private var state = TranscriptLoadState.loading
    @State private var highlightedMessageID: String?

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Transcript")
                    .font(.headline)
                Text(title)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Done") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
            }
            .padding()
            Divider()
            content
        }
        .frame(minWidth: 680, minHeight: 560)
        .task {
            await reload()
        }
    }

    @ViewBuilder
    private var content: some View {
        switch state {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(error):
            ContentUnavailableView(
                "Unable to open transcript",
                systemImage: "exclamationmark.triangle",
                description: Text(error)
            )
        case let .ready(messages):
            if messages.isEmpty {
                ContentUnavailableView("Empty transcript", systemImage: "text.bubble")
            } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 12) {
                            ForEach(messages, id: \.messageId) { item in
                                VStack(alignment: .leading, spacing: 5) {
                                    HStack(spacing: 7) {
                                        MixinRemoteImage(url: URL(string: item.senderAvatarUrl)) { image in
                                            image.resizable().scaledToFill()
                                        } placeholder: {
                                            Image(systemName: "person.crop.circle.fill")
                                                .resizable()
                                                .foregroundStyle(.secondary)
                                        }
                                        .frame(width: 22, height: 22)
                                        .clipShape(Circle())

                                        Text(item.senderName)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                        Spacer()
                                        Text(item.transcriptTime)
                                            .font(.caption2)
                                            .foregroundStyle(.tertiary)
                                    }
                                    TranscriptRowContent(
                                        message: item,
                                        messages: messages,
                                        sentByCurrentUser: sentByCurrentUser,
                                        mentionNames: mentionNames,
                                        onLocate: { messageID in
                                            withAnimation(.easeOut(duration: 0.25)) {
                                                proxy.scrollTo(messageID, anchor: .center)
                                            }
                                            highlightedMessageID = messageID
                                            Task {
                                                try? await Task.sleep(for: .milliseconds(700))
                                                if highlightedMessageID == messageID {
                                                    highlightedMessageID = nil
                                                }
                                            }
                                        },
                                        onAttachmentAction: {
                                            Task {
                                                await onAttachmentAction(item)
                                                await reload()
                                            }
                                        },
                                        onAppAction: onAppAction
                                    )
                                }
                                .padding(10)
                                .background(
                                    highlightedMessageID == item.messageId
                                        ? Color.accentColor.opacity(0.2)
                                        : Color(nsColor: .controlBackgroundColor),
                                    in: RoundedRectangle(cornerRadius: 10)
                                )
                                .id(item.messageId)
                            }
                        }
                        .padding(18)
                    }
                }
            }
        }
    }

    private func reload() async {
        state = .loading
        do {
            state = .ready(try await load())
        } catch {
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }
}

private enum TranscriptLoadState {
    case loading
    case ready([SwiftMessageItem])
    case failed(String)
}

private struct TranscriptRowContent: View {
    let message: SwiftMessageItem
    let messages: [SwiftMessageItem]
    let sentByCurrentUser: Bool
    let mentionNames: [String: String]
    let onLocate: (String) -> Void
    let onAttachmentAction: () -> Void
    let onAppAction: (String, String) -> Void

    @State private var preview: TranscriptRowPreview?

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            if let quoteMessageID = message.quoteMessageId,
               messages.contains(where: { $0.messageId == quoteMessageID })
            {
                Button {
                    onLocate(quoteMessageID)
                } label: {
                    MessageRichText(
                        content: message.quoteContent ?? "Quoted message",
                        baseFontSize: 12,
                        color: .secondary,
                        lineLimit: 2,
                        mentionNames: mentionNames
                    )
                    .padding(.leading, 7)
                    .overlay(alignment: .leading) {
                        Capsule()
                            .fill(Color.accentColor)
                            .frame(width: 3)
                    }
                }
                .buttonStyle(.plain)
                .help("Locate quoted message")
            }

            content
        }
            .sheet(item: $preview) { preview in
                switch preview {
                case .image:
                    ImageMessagePreview(
                        messages: messages.filter {
                            $0.category.hasSuffix("_IMAGE")
                        },
                        initialMessageID: message.messageId,
                        loadWindow: { _ in [] }
                    )
                case .video:
                    VideoMessagePreview(message: message)
                case .post:
                    PostMessagePreview(message: message)
                }
            }
    }

    @ViewBuilder
    private var content: some View {
        let category = message.category.uppercased()
        if category.hasSuffix("_IMAGE") {
            Button(action: mediaAction) {
                MessageMediaImage(
                    source: message.mediaStatus.isComplete ? message.mediaUrl : nil,
                    thumbnail: message.thumbImage,
                    contentMode: .fit
                )
                .frame(maxWidth: 300, maxHeight: 240)
            }
            .buttonStyle(.plain)
        } else if category.hasSuffix("_VIDEO") || category.hasSuffix("_LIVE") {
            Button(action: mediaAction) {
                ZStack {
                    MessageMediaImage(
                        source: message.thumbUrl,
                        thumbnail: message.thumbImage,
                        contentMode: .fill
                    )
                    .frame(width: 300, height: 190)
                    .clipped()

                    Image(systemName: "play.fill")
                        .foregroundStyle(.white)
                        .frame(width: 38, height: 38)
                        .background(.black.opacity(0.45), in: Circle())
                }
            }
            .buttonStyle(.plain)
        } else if category.hasSuffix("_AUDIO") {
            AudioMessageView(
                message: message,
                playlist: messages.filter {
                    $0.category.hasSuffix("_AUDIO")
                },
                mediaDirectory: nil,
                conversationName: nil,
                outgoing: sentByCurrentUser,
                onDownload: onAttachmentAction,
                onCancelTransfer: onAttachmentAction,
                onRetryTransfer: onAttachmentAction,
                onMarkRead: { _ in onAttachmentAction() }
            )
        } else if category.hasSuffix("_DATA") {
            Button(action: fileAction) {
                Label(
                    message.mediaName ?? message.category,
                    systemImage: message.mediaStatus.isComplete
                        ? "doc.fill"
                        : "arrow.down.circle"
                )
            }
            .buttonStyle(.plain)
        } else if category.hasSuffix("_POST") {
            Button {
                preview = .post
            } label: {
                VStack(alignment: .leading, spacing: 5) {
                    Label("Post", systemImage: "doc.richtext")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(message.postPreview)
                        .lineLimit(6)
                        .multilineTextAlignment(.leading)
                }
            }
            .buttonStyle(.plain)
        } else if category.hasSuffix("_STICKER"),
                  let url = [message.stickerAssetUrl, message.mediaUrl, message.thumbUrl]
                      .compactMap({ $0 })
                      .compactMap(URL.init(string:))
                      .first
        {
            StickerImage(
                urlString: url.absoluteString,
                assetType: message.stickerAssetType,
                stickerID: message.stickerId
            )
            .frame(maxWidth: 220, maxHeight: 200)
        } else if category.hasSuffix("_CONTACT") {
            Label(
                message.sharedUserFullName ?? "Contact",
                systemImage: "person.crop.circle"
            )
            if let identity = message.sharedUserIdentityNumber {
                Text(identity)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        } else if category.hasSuffix("_LOCATION") {
            SpecialMessageContentView(
                message: message,
                mentionNames: mentionNames,
                onStrangerAction: { _ in false }
            )
        } else if ["APP_CARD", "APP_BUTTON_GROUP"].contains(category) {
            AppMessageView(message: message, onAction: onAppAction)
        } else if category.hasSuffix("_TRANSCRIPT") {
            VStack(alignment: .leading, spacing: 5) {
                Label("Transcript", systemImage: "text.bubble")
                ForEach(message.transcriptPreviewLines.prefix(4), id: \.self) { line in
                    Text(line)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
        } else {
            MessageRichText(
                content: message.content.isEmpty ? message.category : message.content,
                mentionNames: mentionNames
            )
        }
    }

    private func mediaAction() {
        switch message.mediaStatus.uppercased() {
        case "DONE", "READ":
            preview = message.category.hasSuffix("_IMAGE") ? .image : .video
        case "CANCELED", "PENDING":
            onAttachmentAction()
        default:
            if message.category.hasSuffix("_LIVE"),
               let url = message.mediaUrl?.httpURL
            {
                NSWorkspace.shared.open(url)
            }
        }
    }

    private func fileAction() {
        if message.mediaStatus.isComplete {
            guard let url = message.localMediaURL else {
                return
            }
            if FileInteraction.shouldOpenDirectly(url) {
                MessageMediaInteraction.open(message)
            } else {
                MessageMediaInteraction.save(message)
            }
        } else {
            onAttachmentAction()
        }
    }
}

private enum TranscriptRowPreview: String, Identifiable {
    case image
    case video
    case post

    var id: String { rawValue }
}

enum FileInteraction {
    private static let directlyOpenableExtensions: Set<String> = [
        "jpg", "jpeg", "png", "gif", "bmp", "webp", "avif",
        "mp4", "m3u8", "ts", "mp3", "wav", "pdf",
        "doc", "docx", "xls", "xlsx", "ppt", "pptx",
        "txt", "rtf", "csv", "log",
    ]

    static func shouldOpenDirectly(_ url: URL) -> Bool {
        directlyOpenableExtensions.contains(url.pathExtension.lowercased())
    }

    @MainActor
    static func save(source: URL, suggestedName: String) async {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = suggestedName
        guard await panel.begin() == .OK, let destination = panel.url else {
            return
        }
        do {
            if FileManager.default.fileExists(atPath: destination.path) {
                try FileManager.default.removeItem(at: destination)
            }
            try FileManager.default.copyItem(at: source, to: destination)
        } catch {
            NSAlert(error: error).runModal()
        }
    }
}

extension SwiftMessageItem {
    var localMediaURL: URL? {
        mediaUrl?.localFileURL
    }

    var resolvedMediaURL: URL? {
        guard let mediaUrl else {
            return nil
        }
        return mediaUrl.httpURL ?? mediaUrl.localFileURL
    }

    var postPreview: String {
        let lines = content.split(whereSeparator: \.isNewline).prefix(10)
        return String(lines.joined(separator: "\n").prefix(1_024))
    }

    var postAttributedText: AttributedString {
        (try? AttributedString(markdown: content)) ?? AttributedString(content)
    }

    var transcriptPreviewLines: [String] {
        guard let data = content.data(using: .utf8),
              let objects = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]]
        else {
            return content
                .split(whereSeparator: \.isNewline)
                .map(String.init)
        }
        return objects.compactMap { object in
            let name = (object["name"] as? String)
                ?? (object["sender_name"] as? String)
                ?? (object["senderName"] as? String)
                ?? "Message"
            let preview = (object["content"] as? String)
                ?? (object["media_name"] as? String)
                ?? (object["mediaName"] as? String)
                ?? (object["category"] as? String)
                ?? ""
            return "\(name): \(preview)"
        }
    }

    var transcriptTime: String {
        Date(timeIntervalSince1970: Double(createdAtMicros) / 1_000_000)
            .formatted(date: .omitted, time: .shortened)
    }
}

private extension String {
    var httpURL: URL? {
        guard let url = URL(string: self),
              ["http", "https"].contains(url.scheme?.lowercased() ?? "")
        else {
            return nil
        }
        return url
    }

    var localFileURL: URL? {
        if let url = URL(string: self), url.isFileURL {
            return url
        }
        guard !isEmpty else {
            return nil
        }
        return URL(fileURLWithPath: self)
    }

    var embeddedImage: NSImage? {
        let payload: Substring
        if hasPrefix("data:"), let comma = firstIndex(of: ",") {
            payload = self[index(after: comma)...]
        } else {
            payload = self[...]
        }
        guard let data = Data(base64Encoded: String(payload)) else {
            return nil
        }
        return NSImage(data: data)
    }

}

extension String {
    var isComplete: Bool {
        ["DONE", "READ"].contains(uppercased())
    }
}
