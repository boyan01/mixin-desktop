import AppKit
import SwiftUI
import SwiftUIIntrospect

enum AppScrollViewStyle {
  static func apply(to scrollView: NSScrollView) {
    scrollView.scrollerStyle = .overlay
    scrollView.autohidesScrollers = true
    scrollView.hasVerticalScroller = true
    scrollView.hasHorizontalScroller = false
    if !(scrollView.verticalScroller is AppOverlayScroller) {
      scrollView.verticalScroller = AppOverlayScroller()
    }
  }
}

extension View {
  func appListScrollStyle() -> some View {
    introspect(.list, on: .macOS(.v15, .v26)) { tableView in
      guard let scrollView = tableView.enclosingScrollView else {
        return
      }
      AppScrollViewStyle.apply(to: scrollView)
    }
  }
}
