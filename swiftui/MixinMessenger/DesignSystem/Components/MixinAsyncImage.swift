import Nuke
import NukeUI
import SwiftUI

enum MixinAsyncImagePhase {
  case empty
  case success(Image)
  case failure(Error)
}

struct MixinAsyncImage<Content: View>: View {
  private let request: ImageRequest?
  private let transaction: Transaction
  private let content: (MixinAsyncImagePhase) -> Content

  init(
    url: URL?,
    maxPixelSize: Float? = nil,
    transaction: Transaction = Transaction(animation: nil),
    @ViewBuilder content: @escaping (MixinAsyncImagePhase) -> Content
  ) {
    request = Self.makeRequest(url: url, maxPixelSize: maxPixelSize)
    self.transaction = transaction
    self.content = content
  }

  var body: some View {
    LazyImage(request: request, transaction: transaction) { state in
      if let image = state.image {
        content(.success(image))
      } else if let error = state.error {
        content(.failure(error))
      } else {
        content(.empty)
      }
    }
    .pipeline(ImagePipeline.shared)
  }

  private static func makeRequest(url: URL?, maxPixelSize: Float?) -> ImageRequest? {
    guard let url else {
      return nil
    }
    var request = ImageRequest(url: url)
    if let maxPixelSize {
      request.thumbnail = .init(maxPixelSize: maxPixelSize)
    }
    return request
  }
}

struct MixinRemoteImage<Content: View, Placeholder: View>: View {
  private let url: URL?
  private let content: (Image) -> Content
  private let placeholder: () -> Placeholder

  init(
    url: URL?,
    @ViewBuilder content: @escaping (Image) -> Content,
    @ViewBuilder placeholder: @escaping () -> Placeholder
  ) {
    self.url = url
    self.content = content
    self.placeholder = placeholder
  }

  var body: some View {
    LazyImage(url: url) { state in
      if let image = state.image {
        content(image)
      } else {
        placeholder()
      }
    }
    .pipeline(ImagePipeline.shared)
  }
}
