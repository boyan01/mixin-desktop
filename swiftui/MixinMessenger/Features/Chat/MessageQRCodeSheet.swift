import AppKit
import CoreImage.CIFilterBuiltins
import SwiftUI
import Vision

enum MessageQRPresentation: Identifiable {
    case generated(String)
    case detected([String])
    case detectionFailed

    var id: String {
        switch self {
        case let .generated(content):
            "generated:\(content)"
        case let .detected(contents):
            "detected:\(contents.joined(separator: "\u{1f}"))"
        case .detectionFailed:
            "detection-failed"
        }
    }
}

struct MessageQRCodeSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme

    let presentation: MessageQRPresentation

    var body: some View {
        if case let .generated(content) = presentation {
            generatedContent(content)
        } else {
            auxiliaryContent
        }
    }

    private var auxiliaryContent: some View {
        VStack(spacing: 18) {
            HStack {
                Text(title)
                    .font(.headline)
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                }
                .buttonStyle(.borderless)
                .accessibilityLabel("Close")
            }

            switch presentation {
            case .generated:
                EmptyView()
            case let .detected(contents):
                detectedContent(contents)
            case .detectionFailed:
                ContentUnavailableView(
                    "No QR Code Found",
                    systemImage: "qrcode.viewfinder",
                    description: Text(
                        "The downloaded image does not contain a readable QR code."
                    )
                )
                .frame(minHeight: 180)
            }
        }
        .padding(22)
        .frame(minWidth: 360, idealWidth: 420)
    }

    private var title: String {
        switch presentation {
        case .generated:
            "Message QR Code"
        case .detected:
            "QR Code Content"
        case .detectionFailed:
            "Scan QR Code"
        }
    }

    private func generatedContent(_ content: String) -> some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 36)
            if let image = QRCodeRenderer.image(for: content) {
                Image(nsImage: image)
                    .interpolation(.none)
                    .resizable()
                    .scaledToFit()
                    .padding(8)
                    .frame(width: 240, height: 240)
                    .background(.white)
                    .accessibilityLabel("QR code for message text")
            } else {
                ContentUnavailableView(
                    "Unable to Render QR Code",
                    systemImage: "qrcode"
                )
                .frame(height: 240)
            }
            Spacer().frame(height: 20)
            Button("Confirm") {
                dismiss()
            }
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(.white)
            .padding(.vertical, 8)
            .padding(.horizontal, 16)
            .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
            Spacer().frame(height: 20)
        }
        .padding(.horizontal, 20)
        .frame(minWidth: 320, minHeight: 210)
    }

    private func detectedContent(_ contents: [String]) -> some View {
        AppScrollView {
            LazyVStack(spacing: 12) {
                ForEach(Array(contents.enumerated()), id: \.offset) { _, content in
                    VStack(alignment: .leading, spacing: 10) {
                        Text(content)
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)

                        HStack {
                            Button("Copy", systemImage: "doc.on.doc") {
                                copy(content)
                            }
                            if let url = actionableURL(content) {
                                Button("Open", systemImage: "arrow.up.forward.app") {
                                    NSWorkspace.shared.open(url)
                                }
                            }
                        }
                        .buttonStyle(.borderless)
                    }
                    .padding(12)
                    .background(.quaternary.opacity(0.6), in: RoundedRectangle(cornerRadius: 10))
                }
            }
        }
        .frame(minHeight: 140, maxHeight: 360)
    }

    private func copy(_ content: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(content, forType: .string)
    }

    private func actionableURL(_ content: String) -> URL? {
        guard let url = URL(string: content),
              let scheme = url.scheme?.lowercased(),
              ["http", "https", "mixin"].contains(scheme)
        else {
            return nil
        }
        return url
    }
}

enum MessageQRScanner {
    static func scan(imageAt url: URL) async throws -> [String] {
        try await Task.detached(priority: .userInitiated) {
            let request = VNDetectBarcodesRequest()
            request.symbologies = [.qr]
            let handler = VNImageRequestHandler(url: url)
            try handler.perform([request])
            var seen = Set<String>()
            return (request.results ?? []).compactMap(\.payloadStringValue).filter {
                seen.insert($0).inserted
            }
        }.value
    }
}

private enum QRCodeRenderer {
    static func image(for content: String) -> NSImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(content.utf8)
        filter.correctionLevel = "Q"
        guard let output = filter.outputImage else {
            return nil
        }
        let scaled = output.transformed(by: CGAffineTransform(scaleX: 10, y: 10))
        let representation = NSCIImageRep(ciImage: scaled)
        let image = NSImage(size: representation.size)
        image.addRepresentation(representation)
        return image
    }
}
