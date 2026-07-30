import Observation
import SwiftUI

@MainActor
@Observable
final class SettingsPreferencesModel {
    private(set) var theme = "system"
    private(set) var showAvatar = true
    private(set) var showIdentityNumber = false
    private(set) var messagePreview = true
    private(set) var chatFontSizeDelta = 0.0
    private(set) var loadError: String?

    private var desktop: SwiftDesktopHandle?
    private var loaded = false
    private let noticeCenter: MixinNoticeCenter

    init(noticeCenter: MixinNoticeCenter) {
        self.noticeCenter = noticeCenter
    }

    var colorScheme: ColorScheme? {
        switch theme {
        case "light": .light
        case "dark": .dark
        default: nil
        }
    }

    func load(desktop: SwiftDesktopHandle) async {
        self.desktop = desktop
        guard !loaded else {
            return
        }

        do {
            theme = switch try await desktop.setting(key: "brightness") {
            case "1": "dark"
            case "2": "light"
            default: "system"
            }
            showAvatar = try await desktop.setting(key: "messageShowAvatar")
                .flatMap(Bool.init) ?? showAvatar
            showIdentityNumber = try await desktop.setting(key: "messageShowIdentityNumber")
                .flatMap(Bool.init) ?? showIdentityNumber
            messagePreview = try await desktop.setting(key: "messagePreview")
                .flatMap(Bool.init) ?? messagePreview
            chatFontSizeDelta = try await desktop.setting(key: "chatFontSizeDelta")
                .flatMap(Double.init) ?? chatFontSizeDelta
            loaded = true
            loadError = nil
        } catch {
            AppLogger.error("Load settings preferences failed", error: error)
            loadError = MixinErrorPresenter.message(for: error)
            noticeCenter.show(error: error)
        }
    }

    func setTheme(_ value: String) {
        theme = value
        let persistedValue = switch value {
        case "dark": "1"
        case "light": "2"
        default: "0"
        }
        persist(key: "brightness", value: persistedValue)
    }

    func setShowAvatar(_ value: Bool) {
        showAvatar = value
        persist(key: "messageShowAvatar", value: String(value))
    }

    func setShowIdentityNumber(_ value: Bool) {
        showIdentityNumber = value
        persist(key: "messageShowIdentityNumber", value: String(value))
    }

    func setMessagePreview(_ value: Bool) {
        messagePreview = value
        persist(key: "messagePreview", value: String(value))
    }

    func setChatFontSizeDelta(_ value: Double) {
        chatFontSizeDelta = min(max(value, -2), 4)
        persist(key: "chatFontSizeDelta", value: String(chatFontSizeDelta))
    }

    private func persist(key: String, value: String) {
        guard let desktop else {
            AppLogger.error(
                "Persist setting failed: desktop handle unavailable key=\(key)"
            )
            return
        }
        Task {
            do {
                try await desktop.setSetting(key: key, value: value)
                loadError = nil
            } catch {
                AppLogger.error(
                    "Persist setting failed: key=\(key)",
                    error: error
                )
                loadError = MixinErrorPresenter.message(for: error)
                noticeCenter.show(error: error)
            }
        }
    }
}
