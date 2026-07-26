import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

enum ProfileImageProcessor {
    enum ProcessingError: LocalizedError {
        case unreadableImage
        case encodingFailed

        var errorDescription: String? {
            switch self {
            case .unreadableImage:
                "The selected file is not a supported image."
            case .encodingFailed:
                "The profile photo could not be prepared."
            }
        }
    }

    static func avatarBase64(from url: URL) async throws -> String {
        let data = try await Task.detached(priority: .userInitiated) {
            try makeAvatarData(from: url)
        }.value
        return data
            .base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .trimmingCharacters(in: CharacterSet(charactersIn: "="))
    }

    nonisolated private static func makeAvatarData(from url: URL) throws -> Data {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateThumbnailAtIndex(
                  source,
                  0,
                  [
                      kCGImageSourceCreateThumbnailFromImageAlways: true,
                      kCGImageSourceCreateThumbnailWithTransform: true,
                      kCGImageSourceThumbnailMaxPixelSize: 1_024,
                  ] as CFDictionary
              )
        else {
            throw ProcessingError.unreadableImage
        }

        let side = min(image.width, image.height)
        let crop = CGRect(
            x: (image.width - side) / 2,
            y: (image.height - side) / 2,
            width: side,
            height: side
        )
        guard let cropped = image.cropping(to: crop),
              let context = CGContext(
                  data: nil,
                  width: 512,
                  height: 512,
                  bitsPerComponent: 8,
                  bytesPerRow: 0,
                  space: CGColorSpaceCreateDeviceRGB(),
                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
              )
        else {
            throw ProcessingError.encodingFailed
        }

        context.interpolationQuality = .high
        context.draw(cropped, in: CGRect(x: 0, y: 0, width: 512, height: 512))
        guard let output = context.makeImage() else {
            throw ProcessingError.encodingFailed
        }

        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(
            data,
            UTType.jpeg.identifier as CFString,
            1,
            nil
        ) else {
            throw ProcessingError.encodingFailed
        }
        CGImageDestinationAddImage(
            destination,
            output,
            [kCGImageDestinationLossyCompressionQuality: 0.9] as CFDictionary
        )
        guard CGImageDestinationFinalize(destination) else {
            throw ProcessingError.encodingFailed
        }
        return data as Data
    }
}
