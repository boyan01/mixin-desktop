import CoreServices
import Foundation

@MainActor
final class StorageDirectoryMonitor {
    private let directory: String
    private let onChange: @MainActor () -> Void
    private var stream: FSEventStreamRef?

    init(directory: String, onChange: @escaping @MainActor () -> Void) {
        self.directory = directory
        self.onChange = onChange
    }

    func start() {
        guard stream == nil else {
            return
        }
        var context = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )
        let callback: FSEventStreamCallback = { _, info, _, _, _, _ in
            guard let info else {
                return
            }
            let monitor = Unmanaged<StorageDirectoryMonitor>
                .fromOpaque(info)
                .takeUnretainedValue()
            Task { @MainActor in
                monitor.onChange()
            }
        }
        guard let stream = FSEventStreamCreate(
            nil,
            callback,
            &context,
            [directory] as CFArray,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            0.4,
            FSEventStreamCreateFlags(kFSEventStreamCreateFlagFileEvents)
        ) else {
            return
        }
        self.stream = stream
        FSEventStreamSetDispatchQueue(stream, .main)
        FSEventStreamStart(stream)
    }

    func stop() {
        guard let stream else {
            return
        }
        FSEventStreamStop(stream)
        FSEventStreamInvalidate(stream)
        FSEventStreamRelease(stream)
        self.stream = nil
    }
}
