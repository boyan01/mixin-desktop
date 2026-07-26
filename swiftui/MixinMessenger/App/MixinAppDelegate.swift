import AppKit

final class MixinAppDelegate: NSObject, NSApplicationDelegate {
    private weak var previousKeyWindow: NSWindow?
    private weak var previousFirstResponder: NSResponder?

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
}
