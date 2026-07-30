import AppKit

final class MixinAppDelegate: NSObject, NSApplicationDelegate {
    private weak var previousKeyWindow: NSWindow?
    private weak var previousFirstResponder: NSResponder?
    private var eventMonitor: Any?

    func applicationDidFinishLaunching(_ notification: Notification) {
        eventMonitor = NSEvent.addLocalMonitorForEvents(
            matching: [
                .leftMouseDown,
                .leftMouseUp,
                .rightMouseDown,
                .rightMouseUp,
                .scrollWheel,
            ]
        ) { event in
            Self.logPointerEvent(event)
            return event
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let eventMonitor {
            NSEvent.removeMonitor(eventMonitor)
            self.eventMonitor = nil
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(
        _ sender: NSApplication
    ) -> Bool {
        false
    }

    func applicationShouldHandleReopen(
        _ sender: NSApplication,
        hasVisibleWindows flag: Bool
    ) -> Bool {
        if !flag {
            WindowActions.show()
        }
        return true
    }

    func applicationDidResignActive(_ notification: Notification) {
        guard let window = NSApp.keyWindow else {
            return
        }
        previousKeyWindow = window
        previousFirstResponder = window.firstResponder
        window.makeFirstResponder(nil)
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        defer {
            previousKeyWindow = nil
            previousFirstResponder = nil
        }
        guard let window = previousKeyWindow, window.isVisible else {
            return
        }
        window.makeKey()
        if let responder = previousFirstResponder {
            window.makeFirstResponder(responder)
        }
    }

    private static func logPointerEvent(_ event: NSEvent) {
        let window = event.window
        let hitView = window?.contentView?.hitTest(event.locationInWindow)
        let hitPath = viewPath(from: hitView)
        let firstResponder = window?.firstResponder.map {
            String(describing: type(of: $0))
        } ?? "nil"

        if event.type == .scrollWheel {
            AppLogger.debug(
                "PointerEvent type=scrollWheel location=\(event.locationInWindow) delta=(\(event.scrollingDeltaX), \(event.scrollingDeltaY)) phase=\(event.phase.rawValue) momentum_phase=\(event.momentumPhase.rawValue) hit=\(hitPath) first_responder=\(firstResponder)"
            )
        } else {
            AppLogger.debug(
                "PointerEvent type=\(String(describing: event.type)) button=\(event.buttonNumber) click_count=\(event.clickCount) location=\(event.locationInWindow) hit=\(hitPath) first_responder=\(firstResponder)"
            )
        }
    }

    private static func viewPath(from hitView: NSView?) -> String {
        var path: [String] = []
        var view = hitView
        while let current = view, path.count < 12 {
            path.append(
                "\(String(describing: type(of: current)))[\(current.frame)]"
            )
            view = current.superview
        }
        return path.isEmpty ? "nil" : path.joined(separator: " > ")
    }
}
