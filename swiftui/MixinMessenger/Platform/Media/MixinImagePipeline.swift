import CryptoKit
import Foundation
import Nuke

enum MixinImagePipeline {
  private static let cacheDirectoryName = "cacheimage"
  private static let maximumResponseSize: UInt64 = 32 * 1024 * 1024

  @MainActor
  static func configure(desktop: SwiftDesktopHandle) throws {
    let dataCache = try DataCache(
      path: cacheDirectory(),
      filenameGenerator: md5Filename(for:)
    )
    dataCache.isSweepEnabled = false
    var configuration = ImagePipeline.Configuration(
      dataLoader: RustImageDataLoader(
        desktop: desktop,
        maximumResponseSize: maximumResponseSize
      )
    )
    configuration.dataCache = dataCache
    configuration.dataCachePolicy = .storeOriginalData
    ImagePipeline.shared = ImagePipeline(configuration: configuration)
  }

  private static func cacheDirectory() throws -> URL {
    guard
      let root = FileManager.default.urls(
        for: .cachesDirectory,
        in: .userDomainMask
      ).first
    else {
      throw CocoaError(.fileNoSuchFile)
    }
    return
      root
      .appendingPathComponent(cacheDirectoryName, isDirectory: true)
  }

  nonisolated private static func md5Filename(for key: String) -> String? {
    Insecure.MD5.hash(data: Data(key.utf8))
      .map { String(format: "%02x", $0) }
      .joined()
  }
}

private final class RustImageDataLoader: DataLoading, @unchecked Sendable {
  private let desktop: SwiftDesktopHandle
  private let maximumResponseSize: UInt64

  init(desktop: SwiftDesktopHandle, maximumResponseSize: UInt64) {
    self.desktop = desktop
    self.maximumResponseSize = maximumResponseSize
  }

  nonisolated func loadData(
    with request: URLRequest,
    didReceiveData: @escaping @Sendable (Data, URLResponse) -> Void,
    completion: @escaping @Sendable (Error?) -> Void
  ) -> any Cancellable {
    let task = Task {
      do {
        guard let url = request.url else {
          throw URLError(.badURL)
        }
        let response = try await desktop.httpRequest(
          method: request.httpMethod ?? "GET",
          url: url.absoluteString,
          headers: request.allHTTPHeaderFields ?? [:],
          body: request.httpBody,
          timeoutMillis: UInt64(request.timeoutInterval * 1_000),
          maxResponseBytes: maximumResponseSize
        )
        try Task.checkCancellation()
        guard (200..<300).contains(response.statusCode) else {
          throw DataLoader.Error.statusCodeUnacceptable(Int(response.statusCode))
        }
        guard
          let urlResponse = HTTPURLResponse(
            url: url,
            statusCode: Int(response.statusCode),
            httpVersion: nil,
            headerFields: response.headers
          )
        else {
          throw URLError(.badServerResponse)
        }
        didReceiveData(response.body, urlResponse)
        completion(nil)
      } catch {
        completion(error)
      }
    }
    return SwiftTaskCancellable(task: task)
  }
}

private final class SwiftTaskCancellable: Cancellable, @unchecked Sendable {
  private let task: Task<Void, Never>

  init(task: Task<Void, Never>) {
    self.task = task
  }

  nonisolated func cancel() {
    task.cancel()
  }
}
