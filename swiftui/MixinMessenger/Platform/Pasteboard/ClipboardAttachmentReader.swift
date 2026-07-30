import AppKit

enum ClipboardAttachmentReader {
  private struct ImageType {
    let pasteboardType: NSPasteboard.PasteboardType
    let extensionName: String
  }

  private static let imageTypes = [
    ImageType(pasteboardType: .png, extensionName: "png"),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("public.jpeg"),
      extensionName: "jpg"
    ),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("com.compuserve.gif"),
      extensionName: "gif"
    ),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("org.webmproject.webp"),
      extensionName: "webp"
    ),
    ImageType(
      pasteboardType: NSPasteboard.PasteboardType("com.microsoft.bmp"),
      extensionName: "bmp"
    ),
    ImageType(pasteboardType: .tiff, extensionName: "png"),
  ]

  static func files(from pasteboard: NSPasteboard) -> [URL] {
    let urls =
      (pasteboard.readObjects(
        forClasses: [NSURL.self],
        options: [.urlReadingFileURLsOnly: true]
      ) ?? [])
      .compactMap { object in
        (object as? NSURL).map { $0 as URL }
      }
      .filter(\.isFileURL)
      .filter { FileManager.default.fileExists(atPath: $0.path) }
    if !urls.isEmpty {
      return urls
    }

    return pasteboard.pasteboardItems?.compactMap { item in
      writeImage(item)
    } ?? []
  }

  private static func writeImage(_ item: NSPasteboardItem) -> URL? {
    for type in imageTypes {
      guard let data = item.data(forType: type.pasteboardType) else {
        continue
      }
      let output: Data
      if type.pasteboardType == .tiff {
        guard let bitmap = NSBitmapImageRep(data: data),
          let png = bitmap.representation(using: .png, properties: [:])
        else {
          AppLogger.error("Convert pasted TIFF image to PNG failed")
          continue
        }
        output = png
      } else {
        output = data
      }

      let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("mixin-paste-\(UUID().uuidString)")
        .appendingPathExtension(type.extensionName)
      do {
        try output.write(to: url, options: .atomic)
        return url
      } catch {
        AppLogger.error(
          "Write pasted image failed path=\(url.path)",
          error: error
        )
      }
    }
    return nil
  }
}
