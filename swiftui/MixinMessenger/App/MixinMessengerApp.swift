import SwiftUI

@main
struct MixinMessengerApp: App {
    @NSApplicationDelegateAdaptor(MixinAppDelegate.self) private var appDelegate
    @State private var appModel = AppModel()

    var body: some Scene {
        WindowGroup("Mixin Messenger") {
            AppRootView()
                .mixinTheme()
                .mixinNoticeOverlay()
                .environment(appModel)
                .environment(appModel.preferences)
                .environment(appModel.noticeCenter)
                .preferredColorScheme(appModel.preferences.colorScheme)
                .frame(minWidth: 360, minHeight: 480)
                .task {
                    await appModel.start()
                }
        }
        .defaultSize(width: 960, height: 640)
        .commands {
            AppCommands()
        }
    }
}
