import AppKit
import SwiftUI

private struct HomeNavigationFocusedKey: FocusedValueKey {
    typealias Value = HomeNavigationModel
}

private struct AccountSecurityFocusedKey: FocusedValueKey {
    typealias Value = SecurityService
}

private struct DeviceTransferFocusedKey: FocusedValueKey {
    typealias Value = DeviceTransferController
}

extension FocusedValues {
    var homeNavigation: HomeNavigationModel? {
        get { self[HomeNavigationFocusedKey.self] }
        set { self[HomeNavigationFocusedKey.self] = newValue }
    }

    var accountSecurity: SecurityService? {
        get { self[AccountSecurityFocusedKey.self] }
        set { self[AccountSecurityFocusedKey.self] = newValue }
    }

    var deviceTransfer: DeviceTransferController? {
        get { self[DeviceTransferFocusedKey.self] }
        set { self[DeviceTransferFocusedKey.self] = newValue }
    }
}

struct AppCommands: Commands {
    @FocusedValue(\.homeNavigation) private var navigation
    @FocusedValue(\.accountSecurity) private var security
    @FocusedValue(\.deviceTransfer) private var deviceTransfer

    var body: some Commands {
        CommandGroup(after: .appSettings) {
            Button("Preferences…") {
                navigation?.showSettings()
            }
            .keyboardShortcut(",", modifiers: .command)
            .disabled(navigation == nil)

            Divider()

            Button("Lock") {
                security?.lockNow()
            }
                .keyboardShortcut("l", modifiers: [.command, .shift])
                .disabled(security?.hasPasscode != true)

            Button("Quick Search") {
                navigation?.showCommandPalette()
            }
            .keyboardShortcut("k", modifiers: .command)
            .disabled(navigation == nil)

            Button("Show Mixin") {
                WindowActions.show()
            }
        }

        CommandGroup(after: .newItem) {
            Button("New Conversation") {
                navigation?.showCreation(.conversation)
            }
                .keyboardShortcut("n", modifiers: .command)
                .disabled(navigation == nil)
            Button("New Group") {
                navigation?.showCreation(.group)
            }
                .keyboardShortcut("n", modifiers: [.command, .shift])
                .disabled(navigation == nil)
            #if DEBUG
            Divider()
            Button("chat backup and restore") {
                deviceTransfer?.openSetup()
            }
            .disabled(deviceTransfer == nil)
            #endif

            Button("New Circle") {
                navigation?.showCreation(.circle)
            }
            .disabled(navigation == nil)

            Divider()
            Button("Close Window") {
                WindowActions.hide()
            }
            .keyboardShortcut("w", modifiers: .command)
        }

        CommandMenu("Conversation") {
            Button(navigation?.selectedConversation?.isMuted == true ? "Unmute" : "Mute") {
                navigation?.requestConversationCommand(.mute)
            }
            .disabled(navigation?.selectedConversation == nil)
            Button("Search Messages") {
                navigation?.focusMessageSearch()
            }
                .disabled(navigation?.selectedConversationID == nil)
            Button("Delete Chat") {
                navigation?.requestConversationCommand(.delete)
            }
            .disabled(navigation?.selectedConversation == nil)
            Button(navigation?.selectedConversation?.isPinned == true ? "Unpin" : "Pin") {
                navigation?.requestConversationCommand(.pin)
            }
            .disabled(navigation?.selectedConversation == nil)
            Button("Toggle Chat Info") {
                navigation?.toggleConversationInfo()
            }
            .disabled(navigation?.selectedConversation == nil)

        }

        CommandGroup(after: .windowSize) {
            Button("Zoom") {
                WindowActions.zoom()
            }
        }

        CommandGroup(after: .windowArrangement) {
            Button("Previous Conversation") {
                navigation?.selectAdjacentConversation(forward: false)
            }
            .keyboardShortcut(.upArrow, modifiers: .command)
            .disabled(navigation?.selectedConversationID == nil)

            Button("Next Conversation") {
                navigation?.selectAdjacentConversation(forward: true)
            }
            .keyboardShortcut(.downArrow, modifiers: .command)
            .disabled(navigation?.selectedConversationID == nil)

            Divider()

            Button("Show Mixin") {
                WindowActions.show()
            }
            .keyboardShortcut("o", modifiers: .command)
        }

        CommandGroup(replacing: .help) {
            Button("Mixin Help Center") {
                WindowActions.open("https://support.mixin.one/")
            }
            Button("Terms of Service") {
                WindowActions.open("https://mixin.one/pages/terms")
            }
            Button("Privacy Policy") {
                WindowActions.open("https://mixin.one/pages/privacy")
            }
        }
    }
}

enum WindowActions {
    static func hide() {
        NSApp.keyWindow?.orderOut(nil)
    }

    static func show() {
        let window = NSApp.windows.first {
            $0.title == "Mixin Messenger" && $0.canBecomeKey && !($0 is NSPanel)
        } ?? NSApp.windows.first {
            $0.canBecomeKey && !($0 is NSPanel)
        } ?? NSApp.windows.first
        window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    static func zoom() {
        NSApp.keyWindow?.zoom(nil)
    }

    static func open(_ source: String) {
        guard let url = URL(string: source) else {
            return
        }
        NSWorkspace.shared.open(url)
    }
}
