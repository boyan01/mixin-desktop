import AppKit
import AVFoundation
import AVKit
import ImageIO
import SwiftUI
import UniformTypeIdentifiers

struct AttachmentPreviewSheet: View {
    enum Mode: String, CaseIterable, Identifiable {
        case media = "Media"
        case files = "Files"
        case zip = "ZIP"

        var id: String { rawValue }
    }

    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var urls: [URL]
    @State private var mode: Mode
    @State private var caption = ""
    @State private var sending = false
    @State private var progress = 0
    @State private var error: String?
    @State private var zipPassword = ""
    @State private var zipPasswordHidden = true
    @State private var editTarget: EditableAttachment?
    let onSend: (AttachmentSendRequest) async -> Bool
    let onComplete: () -> Void

    init(
        urls: [URL],
        onSend: @escaping (AttachmentSendRequest) async -> Bool,
        onComplete: @escaping () -> Void
    ) {
        _urls = State(initialValue: urls)
        _mode = State(initialValue: urls.contains(where: \.isVisualMedia) ? .media : .files)
        self.onSend = onSend
        self.onComplete = onComplete
    }

    var body: some View {
        ZStack(alignment: .topTrailing) {
            VStack(spacing: 0) {
                Spacer().frame(height: 20)
                HStack(spacing: 0) {
                    previewTab(
                        mode: .media,
                        asset: "FilePreviewImages",
                        help: "Send quickly",
                        visible: urls.contains(where: \.isVisualMedia)
                    )
                    previewTab(
                        mode: .files,
                        asset: "FilePreviewFiles",
                        help: "Send without compression",
                        visible: true
                    )
                    previewTab(
                        mode: .zip,
                        asset: "FilePreviewZip",
                        help: "Send archived",
                        visible: urls.count > 1
                    )
                }
                .padding(.leading, 15)
                .frame(maxWidth: .infinity, alignment: .leading)
                Spacer().frame(height: 4)

                if mode == .zip {
                    zipPage
                } else {
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(urls, id: \.path) { url in
                                AttachmentPreviewRow(
                                    url: url,
                                    mode: mode,
                                    onEdit: url.contentType?.conforms(to: .image) == true
                                        ? { editTarget = EditableAttachment(url: url) }
                                        : nil
                                ) {
                                    urls.removeAll { $0 == url }
                                    normalizeMode()
                                    if urls.isEmpty {
                                        dismiss()
                                    }
                                }
                            }
                        }
                    }
                }

                Spacer().frame(height: 16)
                if supportsCaption {
                    TextField("Caption", text: $caption, axis: .vertical)
                        .textFieldStyle(.plain)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                        .lineLimit(1 ... 3)
                        .padding(.leading, 8)
                        .padding(.vertical, 8)
                        .frame(minHeight: 40)
                        .background(captionBackground, in: RoundedRectangle(cornerRadius: 4))
                        .padding(.horizontal, 30)
                }

                if let error {
                    Text(error)
                        .font(.system(size: 12))
                        .foregroundStyle(theme.destructive)
                        .padding(.top, 8)
                }
                Spacer().frame(height: 16)

                Button("SEND") {
                    send(silent: false)
                }
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(.white)
                .padding(.horizontal, 32)
                .padding(.vertical, 18)
                .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
                .disabled(urls.isEmpty || sending)
                .contextMenu {
                    Button("Send Silently") {
                        send(silent: true)
                    }
                }

                Spacer().frame(height: 24)
                Text("Press Return to send")
                    .font(.system(size: 12))
                    .foregroundStyle(.gray)
                Spacer().frame(height: 24)
            }
            Button {
                dismiss()
            } label: {
                Image(systemName: "xmark")
            }
            .buttonStyle(MixinActionButtonStyle())
            .padding(22)
        }
        .frame(width: 480, height: 600)
        .background(theme.popUp)
        .dropDestination(for: URL.self) { dropped, _ in
            addFiles(dropped)
            return !dropped.isEmpty
        }
        .sheet(item: $editTarget) { target in
            AttachmentImageEditor(url: target.url) { editedURL in
                guard let index = urls.firstIndex(of: target.url) else {
                    return
                }
                urls[index] = editedURL
            }
        }
    }

    private var supportsCaption: Bool {
        urls.count == 1
            && urls[0].contentType?.conforms(to: .image) == true
    }

    private var availableModes: [Mode] {
        urls.count > 1 ? Mode.allCases : [.media, .files]
    }

    private var captionBackground: Color {
        switch NSApp.effectiveAppearance.name {
        case .darkAqua, .vibrantDark:
            Color.white.opacity(0.08)
        default:
            Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255)
        }
    }

    @ViewBuilder
    private func previewTab(
        mode tab: Mode,
        asset: String,
        help: String,
        visible: Bool
    ) -> some View {
        if visible {
            Button {
                mode = tab
            } label: {
                Image(asset)
                    .resizable()
                    .renderingMode(.template)
                    .foregroundStyle(mode == tab ? theme.accent : theme.icon)
                    .frame(width: 24, height: 24)
                    .frame(width: 54, height: 54)
            }
            .buttonStyle(.plain)
            .help(help)
        }
    }

    private var zipPage: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                Spacer().frame(width: 30)
                Text("ZIP")
                    .font(.system(size: 16))
                    .foregroundStyle(theme.secondaryText)
                    .frame(width: 50, height: 50)
                    .background(theme.statusBackground, in: Circle())
                Spacer().frame(width: 16)
                VStack(alignment: .leading, spacing: 4) {
                    Text("Archive.zip")
                        .font(.system(size: 16))
                        .foregroundStyle(theme.text)
                    Text("Archived folder")
                        .font(.system(size: 14))
                        .foregroundStyle(theme.secondaryText)
                }
                Spacer()
                Spacer().frame(width: 30)
            }
            Spacer()
            HStack(spacing: 8) {
                Image(systemName: "lock")
                    .font(.system(size: 18))
                    .foregroundStyle(
                        zipPassword.isEmpty ? theme.secondaryText : theme.text
                    )
                if zipPasswordHidden {
                    SecureField("Encrypt ZIP file with password", text: $zipPassword)
                        .textFieldStyle(.plain)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                } else {
                    TextField("Encrypt ZIP file with password", text: $zipPassword)
                        .textFieldStyle(.plain)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                }
                Button {
                    zipPasswordHidden.toggle()
                } label: {
                    Image(systemName: zipPasswordHidden ? "eye.slash" : "eye")
                        .font(.system(size: 20))
                        .foregroundStyle(
                            zipPassword.isEmpty ? theme.secondaryText : theme.text
                        )
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 16)
            .frame(width: 300, height: 36)
            .background(captionBackground, in: Capsule())
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func addFiles(_ selected: [URL]) {
        let existing = Set(urls.map(\.standardizedFileURL))
        urls.append(contentsOf: selected.filter {
            !existing.contains($0.standardizedFileURL)
        })
        normalizeMode()
    }

    private func send(silent: Bool) {
        sending = true
        error = nil
        progress = 0
        Task {
            let outgoing: [URL]
            if mode == .zip {
                do {
                    outgoing = [try AttachmentArchive.create(
                        urls: urls,
                        password: zipPassword.trimmingCharacters(in: .whitespacesAndNewlines)
                    )]
                } catch {
                    self.error = MixinErrorPresenter.message(for: error)
                    sending = false
                    return
                }
            } else {
                outgoing = urls
            }
            for url in outgoing {
                let preparedURL = mode == .media
                    ? (AttachmentImageCompressor.compressIfNeeded(url) ?? url)
                    : url
                let request = AttachmentSendRequest(
                    url: preparedURL,
                    mode: mode,
                    caption: supportsCaption
                        ? caption.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty
                        : nil,
                    silent: silent
                )
                guard await onSend(request) else {
                    error = "The attachment could not be sent."
                    sending = false
                    return
                }
                progress += 1
            }
            sending = false
            onComplete()
            dismiss()
        }
    }

    private func normalizeMode() {
        if mode == .zip, urls.count < 2 {
            mode = .files
        } else if mode == .media, !urls.contains(where: \.isVisualMedia) {
            mode = .files
        }
    }
}

private struct EditableAttachment: Identifiable {
    let id = UUID()
    let url: URL
}

struct AttachmentSendRequest {
    let url: URL
    let mode: AttachmentPreviewSheet.Mode
    let caption: String?
    let silent: Bool

    var kind: String {
        guard mode == .media, let contentType = url.contentType else {
            return "DATA"
        }
        if contentType.conforms(to: .image) {
            return "IMAGE"
        }
        if contentType.conforms(to: .movie) {
            return "VIDEO"
        }
        return "DATA"
    }

    var mimeType: String {
        url.contentType?.preferredMIMEType ?? "application/octet-stream"
    }

    var dimensions: (width: Int32, height: Int32)? {
        guard kind == "IMAGE" else {
            return nil
        }
        return imageDimensions
    }

    func videoMetadata() async -> (
        width: Int32,
        height: Int32,
        durationMillis: Int64
    )? {
        guard kind == "VIDEO" else {
            return nil
        }
        do {
            let asset = AVURLAsset(url: url)
            let duration = try await asset.load(.duration)
            guard let track = try await asset.loadTracks(withMediaType: .video).first else {
                return nil
            }
            let size = try await track.load(.naturalSize)
                .applying(try await track.load(.preferredTransform))
            let durationMillis = Int64((duration.seconds * 1_000).rounded())
            let width = Int32(abs(size.width).rounded())
            let height = Int32(abs(size.height).rounded())
            guard width > 0, height > 0, durationMillis > 0 else {
                return nil
            }
            return (width, height, durationMillis)
        } catch {
            return nil
        }
    }

    var thumbnail: String? {
        kind == "VIDEO" ? "L1GIo.]day]K-;jsfQjsRjfQj[fQ" : nil
    }

    private var imageDimensions: (width: Int32, height: Int32)? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil)
                as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? NSNumber,
              let height = properties[kCGImagePropertyPixelHeight] as? NSNumber
        else {
            return nil
        }
        return (width.int32Value, height.int32Value)
    }
}

private struct AttachmentPreviewRow: View {
    @Environment(\.mixinTheme) private var theme
    let url: URL
    let mode: AttachmentPreviewSheet.Mode
    let onEdit: (() -> Void)?
    let onRemove: () -> Void

    var body: some View {
        if mode == .files || !url.isVisualMedia {
            normalFileRow
        } else if url.contentType?.conforms(to: .image) == true,
                  let image = NSImage(contentsOf: url)
        {
            imageRow(image)
        } else if url.contentType?.conforms(to: .movie) == true {
            videoRow
        } else {
            mediaRow
        }
    }

    private var normalFileRow: some View {
        HStack(spacing: 0) {
            Spacer().frame(width: 30)
            Text(fileExtension)
                .font(.system(size: 16))
                .foregroundStyle(theme.secondaryText)
                .frame(width: 50, height: 50)
                .background(theme.statusBackground, in: Circle())
            Spacer().frame(width: 16)
            VStack(alignment: .leading, spacing: 0) {
                Text(url.lastPathComponent)
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                    .lineLimit(1)
                    .lineSpacing(8)
                Text(formattedFileSize)
                    .font(.system(size: 14))
                    .foregroundStyle(theme.secondaryText)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Button(action: onRemove) {
                Image("Delete")
                    .resizable()
                    .renderingMode(.template)
                    .foregroundStyle(theme.secondaryText)
                    .frame(width: 24, height: 24)
                    .frame(width: 44, height: 44)
            }
            .buttonStyle(.plain)
            Spacer().frame(width: 10)
        }
    }

    private var mediaRow: some View {
        HStack(spacing: 12) {
            preview
                .frame(width: 64, height: 54)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
                .clipShape(RoundedRectangle(cornerRadius: 8))

            VStack(alignment: .leading, spacing: 4) {
                Text(url.lastPathComponent)
                    .lineLimit(1)
                Text(fileDescription)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if let onEdit {
                Button(action: onEdit) {
                    Image(systemName: "pencil")
                }
                .buttonStyle(.borderless)
                .help("Edit Image")
            }
            Button(role: .destructive, action: onRemove) {
                Image(systemName: "trash")
            }
            .buttonStyle(.borderless)
        }
        .padding(.vertical, 5)
    }

    private func imageRow(_ image: NSImage) -> some View {
        ZStack(alignment: .bottomTrailing) {
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
                .frame(width: 420)
            LinearGradient(
                colors: [.clear, .black.opacity(0.28)],
                startPoint: .top,
                endPoint: .bottom
            )
            .frame(height: 50)
            HStack(spacing: 0) {
                Spacer()
                if let onEdit {
                    Button(action: onEdit) {
                        Image("EditImage")
                            .resizable()
                            .renderingMode(.template)
                            .foregroundStyle(.white)
                            .frame(width: 24, height: 24)
                            .frame(width: 44, height: 50)
                    }
                    .buttonStyle(.plain)
                    .help("Edit Image")
                }
                Button(action: onRemove) {
                    Image("Delete")
                        .resizable()
                        .frame(width: 24, height: 24)
                        .frame(width: 44, height: 50)
                }
                .buttonStyle(.plain)
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .padding(.horizontal, 30)
        .padding(.vertical, 15)
    }

    private var videoRow: some View {
        ZStack(alignment: .bottomTrailing) {
            AttachmentPreviewVideo(url: url)
                .frame(width: 420, height: 200)
            LinearGradient(
                colors: [.clear, .black.opacity(0.28)],
                startPoint: .top,
                endPoint: .bottom
            )
            .frame(height: 50)
            HStack {
                Spacer()
                Button(action: onRemove) {
                    Image("Delete")
                        .resizable()
                        .frame(width: 24, height: 24)
                        .frame(width: 44, height: 50)
                }
                .buttonStyle(.plain)
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .padding(.horizontal, 30)
        .padding(.vertical, 15)
    }

    @ViewBuilder
    private var preview: some View {
        if mode == .media,
           url.contentType?.conforms(to: .image) == true,
           let image = NSImage(contentsOf: url)
        {
            Image(nsImage: image)
                .resizable()
                .scaledToFill()
        } else if mode == .media,
                  url.contentType?.conforms(to: .movie) == true
        {
            VideoPlayer(player: AVPlayer(url: url))
        } else {
            Image(systemName: url.contentType?.conforms(to: .movie) == true
                ? "film"
                : "doc.fill")
                .font(.title2)
                .foregroundStyle(.secondary)
        }
    }

    private var fileDescription: String {
        let values = try? url.resourceValues(forKeys: [.fileSizeKey])
        let size = Int64(values?.fileSize ?? 0)
        return [
            mode == .media && url.isVisualMedia ? "Media" : "File",
            ByteCountFormatter.string(fromByteCount: size, countStyle: .file),
        ].joined(separator: " · ")
    }

    private var fileExtension: String {
        let value = url.pathExtension.uppercased()
        return value.isEmpty ? "FILE" : value
    }

    private var formattedFileSize: String {
        let values = try? url.resourceValues(forKeys: [.fileSizeKey])
        let bytes = Int64(values?.fileSize ?? 0)
        if bytes < 1_024 {
            return "\(bytes) B"
        }
        if bytes < 1_024 * 1_024 {
            return String(format: "%.1f KB", Double(bytes) / 1_024)
        }
        if bytes < 1_024 * 1_024 * 1_024 {
            return String(format: "%.1f MB", Double(bytes) / (1_024 * 1_024))
        }
        return String(format: "%.1f GB", Double(bytes) / (1_024 * 1_024 * 1_024))
    }

}

private struct AttachmentPreviewVideo: View {
    @State private var playback: AttachmentPreviewPlayback

    init(url: URL) {
        _playback = State(initialValue: AttachmentPreviewPlayback(url: url))
    }

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.black
            VideoPlayer(player: playback.player)
            TimelineView(.periodic(from: .now, by: 0.5)) { _ in
                if let remaining = playback.remainingText {
                    Text(remaining)
                        .font(.system(size: 12))
                        .foregroundStyle(.white)
                        .padding(4)
                        .background(
                            .black.opacity(0.3),
                            in: RoundedRectangle(cornerRadius: 5)
                        )
                        .padding(6)
                }
            }
        }
        .onAppear {
            playback.play()
        }
        .onDisappear {
            playback.pause()
        }
    }
}

@MainActor
private final class AttachmentPreviewPlayback {
    let player: AVQueuePlayer
    private let looper: AVPlayerLooper

    init(url: URL) {
        let item = AVPlayerItem(url: url)
        let player = AVQueuePlayer()
        self.player = player
        looper = AVPlayerLooper(player: player, templateItem: item)
        player.isMuted = true
    }

    var remainingText: String? {
        guard let duration = player.currentItem?.duration.seconds,
              duration.isFinite,
              duration > 0
        else {
            return nil
        }
        let position = player.currentTime().seconds
        let seconds = max(Int((duration - position).rounded(.down)), 0)
        return "\(seconds / 60):\(String(format: "%02d", seconds % 60))"
    }

    func play() {
        player.play()
    }

    func pause() {
        player.pause()
    }
}

private enum AttachmentArchive {
    static func create(urls: [URL], password: String) throws -> URL {
        let fileManager = FileManager.default
        let workspace = fileManager.temporaryDirectory
            .appendingPathComponent("mixin_archive_\(UUID().uuidString)", isDirectory: true)
        let staging = workspace.appendingPathComponent("files", isDirectory: true)
        try fileManager.createDirectory(at: staging, withIntermediateDirectories: true)

        var usedNames = Set<String>()
        for url in urls {
            let name = uniqueName(for: url.lastPathComponent, usedNames: &usedNames)
            try fileManager.copyItem(at: url, to: staging.appendingPathComponent(name))
        }

        let archive = workspace.appendingPathComponent("Archive.zip")
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/zip")
        process.currentDirectoryURL = staging
        var arguments = ["-q", "-r"]
        if !password.isEmpty {
            arguments += ["-P", password]
        }
        arguments += [archive.path, "."]
        process.arguments = arguments

        let errorPipe = Pipe()
        process.standardError = errorPipe
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            let data = errorPipe.fileHandleForReading.readDataToEndOfFile()
            let detail = String(data: data, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
            throw CocoaError(
                .fileWriteUnknown,
                userInfo: [NSLocalizedDescriptionKey: detail?.nilIfEmpty ?? "Could not create the ZIP archive."]
            )
        }
        return archive
    }

    private static func uniqueName(for proposed: String, usedNames: inout Set<String>) -> String {
        if usedNames.insert(proposed).inserted {
            return proposed
        }
        let url = URL(fileURLWithPath: proposed)
        let base = url.deletingPathExtension().lastPathComponent
        let suffix = url.pathExtension
        var index = 2
        while true {
            let candidate = suffix.isEmpty ? "\(base) \(index)" : "\(base) \(index).\(suffix)"
            if usedNames.insert(candidate).inserted {
                return candidate
            }
            index += 1
        }
    }
}

private enum AttachmentImageCompressor {
    static func compressIfNeeded(_ url: URL) -> URL? {
        guard url.contentType?.conforms(to: .image) == true,
              url.contentType != .gif,
              let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
        else {
            return nil
        }

        let maximum = 1_920
        let scale = min(1, CGFloat(maximum) / CGFloat(max(image.width, image.height)))
        let width = max(1, Int((CGFloat(image.width) * scale).rounded()))
        let height = max(1, Int((CGFloat(image.height) * scale).rounded()))
        guard let context = CGContext(
            data: nil,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ) else {
            return nil
        }
        context.interpolationQuality = .high
        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        guard let rendered = context.makeImage() else {
            return nil
        }

        let hasAlpha = image.alphaInfo != .none
            && image.alphaInfo != .noneSkipFirst
            && image.alphaInfo != .noneSkipLast
        let type: UTType = hasAlpha ? .png : .jpeg
        let target = FileManager.default.temporaryDirectory
            .appendingPathComponent("mixin_image_\(UUID().uuidString)")
            .appendingPathExtension(type.preferredFilenameExtension ?? "jpg")
        guard let destination = CGImageDestinationCreateWithURL(
            target as CFURL,
            type.identifier as CFString,
            1,
            nil
        ) else {
            return nil
        }
        let options = hasAlpha
            ? nil
            : [kCGImageDestinationLossyCompressionQuality: 0.85] as CFDictionary
        CGImageDestinationAddImage(destination, rendered, options)
        return CGImageDestinationFinalize(destination) ? target : nil
    }
}

private struct AttachmentImageEditor: View {
    private enum Crop: String, CaseIterable, Identifiable {
        case original = "Original"
        case square = "Square"
        case portrait = "4:5"
        case landscape = "16:9"

        var id: String { rawValue }
        var ratio: CGFloat? {
            switch self {
            case .original: nil
            case .square: 1
            case .portrait: 4 / 5
            case .landscape: 16 / 9
            }
        }
    }

    fileprivate struct Stroke {
        var points: [CGPoint]
        let color: Color
    }

    @Environment(\.dismiss) private var dismiss
    @State private var rotation = 0
    @State private var flipped = false
    @State private var crop: Crop = .original
    @State private var drawing = false
    @State private var color: Color = .red
    @State private var strokes: [Stroke] = []
    @State private var activeStrokeIndex: Int?
    @State private var saving = false

    let url: URL
    let onSave: (URL) -> Void

    private var sourceImage: NSImage {
        NSImage(contentsOf: url) ?? NSImage()
    }

    var body: some View {
        VStack(spacing: 14) {
            GeometryReader { geometry in
                let size = fittedSize(in: geometry.size)
                EditedImageCanvas(
                    image: sourceImage,
                    rotation: rotation,
                    flipped: flipped,
                    strokes: strokes
                )
                .frame(width: size.width, height: size.height)
                .clipped()
                .contentShape(Rectangle())
                .gesture(drawGesture, including: drawing ? .all : .none)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }

            HStack {
                Button("Reset") {
                    rotation = 0
                    flipped = false
                    crop = .original
                    strokes.removeAll()
                }
                Spacer()
                Picker("Crop", selection: $crop) {
                    ForEach(Crop.allCases) { value in
                        Text(value.rawValue).tag(value)
                    }
                }
                .frame(width: 150)
                Button {
                    rotation = (rotation + 1) % 4
                } label: {
                    Label("Rotate", systemImage: "rotate.right")
                }
                Button {
                    flipped.toggle()
                } label: {
                    Label("Flip", systemImage: "arrow.left.and.right.righttriangle.left.righttriangle.right")
                }
                Toggle(isOn: $drawing) {
                    Label("Draw", systemImage: "pencil.tip")
                }
                .toggleStyle(.button)
                if drawing {
                    ColorPicker("Color", selection: $color, supportsOpacity: false)
                        .labelsHidden()
                    Button {
                        _ = strokes.popLast()
                    } label: {
                        Image(systemName: "arrow.uturn.backward")
                    }
                    .disabled(strokes.isEmpty)
                }
                Spacer()
                Button("Cancel") {
                    dismiss()
                }
                Button("Done") {
                    save()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(saving)
            }
        }
        .padding()
        .frame(minWidth: 760, minHeight: 620)
    }

    private var drawGesture: some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                if let activeStrokeIndex {
                    strokes[activeStrokeIndex].points.append(value.location)
                } else {
                    strokes.append(Stroke(points: [value.location], color: color))
                    activeStrokeIndex = strokes.indices.last
                }
            }
            .onEnded { _ in
                activeStrokeIndex = nil
            }
    }

    private func fittedSize(in available: CGSize) -> CGSize {
        let source = sourceImage.size
        let rotatedRatio = rotation.isMultiple(of: 2)
            ? source.width / max(source.height, 1)
            : source.height / max(source.width, 1)
        let ratio = crop.ratio ?? rotatedRatio
        let width = min(available.width, available.height * ratio)
        return CGSize(width: width, height: width / max(ratio, 0.01))
    }

    @MainActor
    private func save() {
        saving = true
        let source = sourceImage.size
        let rotatedRatio = rotation.isMultiple(of: 2)
            ? source.width / max(source.height, 1)
            : source.height / max(source.width, 1)
        let ratio = crop.ratio ?? rotatedRatio
        let width = min(CGFloat(1_920), max(source.width, source.height) * min(ratio, 1))
        let size = CGSize(width: max(width, 1), height: max(width / ratio, 1))
        let renderer = ImageRenderer(content:
            EditedImageCanvas(
                image: sourceImage,
                rotation: rotation,
                flipped: flipped,
                strokes: strokes
            )
            .frame(width: size.width, height: size.height)
            .clipped()
        )
        renderer.scale = 1
        guard let image = renderer.nsImage,
              let tiff = image.tiffRepresentation,
              let representation = NSBitmapImageRep(data: tiff),
              let data = representation.representation(using: .png, properties: [:])
        else {
            saving = false
            return
        }
        let target = FileManager.default.temporaryDirectory
            .appendingPathComponent("mixin_edited_\(UUID().uuidString).png")
        do {
            try data.write(to: target, options: .atomic)
            onSave(target)
            dismiss()
        } catch {
            saving = false
        }
    }
}

private struct EditedImageCanvas: View {
    let image: NSImage
    let rotation: Int
    let flipped: Bool
    let strokes: [AttachmentImageEditor.Stroke]

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                Color.clear
                Image(nsImage: image)
                    .resizable()
                    .scaledToFill()
                    .rotationEffect(.degrees(Double(rotation * 90)))
                    .scaleEffect(x: flipped ? -1 : 1, y: 1)
                Canvas { context, _ in
                    for stroke in strokes where stroke.points.count > 1 {
                        var path = Path()
                        path.move(to: stroke.points[0])
                        for point in stroke.points.dropFirst() {
                            path.addLine(to: point)
                        }
                        context.stroke(
                            path,
                            with: .color(stroke.color),
                            style: StrokeStyle(lineWidth: 5, lineCap: .round, lineJoin: .round)
                        )
                    }
                }
            }
            .frame(width: geometry.size.width, height: geometry.size.height)
        }
    }
}

enum AttachmentFilePicker {
    @MainActor
    static func select() async -> [URL] {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.resolvesAliases = true
        let response = await panel.begin()
        return response == .OK ? panel.urls : []
    }
}

private extension URL {
    var contentType: UTType? {
        guard let extensionType = UTType(filenameExtension: pathExtension) else {
            return nil
        }
        return extensionType
    }

    var isVisualMedia: Bool {
        contentType?.conforms(to: .image) == true
            || contentType?.conforms(to: .movie) == true
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
