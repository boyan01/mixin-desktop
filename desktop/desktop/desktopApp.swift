import SwiftUI

@main
struct DesktopApp: App {
    var body: some Scene {
        WindowGroup("Mixin") {
            ContentView()
                .frame(minWidth: 620, minHeight: 480)
        }
        .defaultSize(width: 960, height: 640)
    }
}
