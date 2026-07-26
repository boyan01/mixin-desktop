import AppKit
import Observation
import SwiftUI
import WebKit

struct BotWebContext: Encodable, Sendable {
    let appVersion: String
    let immersive = false
    let appearance: String
    let platform = "Desktop"
    let locale: String
    let conversationID: String
    let currency: String

    enum CodingKeys: String, CodingKey {
        case appVersion = "app_version"
        case immersive
        case appearance
        case platform
        case locale
        case conversationID = "conversation_id"
        case currency
    }
}

@MainActor
enum BotWebViewWindow {
    private static var controllers: [ObjectIdentifier: BotWebViewWindowController] = [:]

    static func open(
        url: URL,
        title: String,
        conversationID: String,
        currency: String
    ) {
        let version = Bundle.main.object(
            forInfoDictionaryKey: "CFBundleShortVersionString"
        ) as? String ?? "0"
        let context = BotWebContext(
            appVersion: version,
            appearance: NSApp.effectiveAppearance.bestMatch(
                from: [.darkAqua, .aqua]
            ) == .darkAqua ? "dark" : "light",
            locale: Locale.current.identifier.replacingOccurrences(
                of: "_",
                with: "-"
            ),
            conversationID: conversationID,
            currency: currency
        )
        let controller = BotWebViewWindowController(
            url: url,
            title: title,
            context: context
        )
        let identifier = ObjectIdentifier(controller)
        controllers[identifier] = controller
        controller.onClose = {
            controllers.removeValue(forKey: identifier)
        }
        controller.showWindow(nil)
        controller.window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
}

@MainActor
private final class BotWebViewWindowController: NSWindowController,
    NSWindowDelegate
{
    var onClose: (() -> Void)?

    init(url: URL, title: String, context: BotWebContext) {
        let model = BotWebViewModel(url: url, context: context)
        let content = BotWebView(model: model)
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 380, height: 750),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = title
        window.minSize = NSSize(width: 320, height: 480)
        window.contentViewController = NSHostingController(rootView: content)
        window.center()
        super.init(window: window)
        window.delegate = self
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func windowWillClose(_: Notification) {
        onClose?()
    }
}

@MainActor
@Observable
private final class BotWebViewModel {
    let initialURL: URL
    let context: BotWebContext
    var title = ""
    var canGoBack = false
    var canGoForward = false
    var isLoading = false
    weak var webView: WKWebView?

    init(url: URL, context: BotWebContext) {
        initialURL = url
        self.context = context
    }

    func goBack() {
        webView?.goBack()
    }

    func goForward() {
        webView?.goForward()
    }

    func reload() {
        webView?.reload()
    }

    func openExternally() {
        NSWorkspace.shared.open(webView?.url ?? initialURL)
    }
}

private struct BotWebView: View {
    @State var model: BotWebViewModel

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Button(action: model.goBack) {
                    Image(systemName: "chevron.left")
                }
                .disabled(!model.canGoBack)
                Button(action: model.goForward) {
                    Image(systemName: "chevron.right")
                }
                .disabled(!model.canGoForward)
                Button(action: model.reload) {
                    Image(systemName: model.isLoading ? "xmark" : "arrow.clockwise")
                }
                Spacer()
                Text(model.title)
                    .font(.callout)
                    .lineLimit(1)
                Spacer()
                Button(action: model.openExternally) {
                    Image(systemName: "arrow.up.right.square")
                }
            }
            .buttonStyle(.borderless)
            .padding(.horizontal, 10)
            .frame(height: 38)

            Divider()
            BotWebViewRepresentable(model: model)
        }
    }
}

private struct BotWebViewRepresentable: NSViewRepresentable {
    let model: BotWebViewModel

    func makeCoordinator() -> Coordinator {
        Coordinator(model: model)
    }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        if let script = makeContextScript(model.context) {
            configuration.userContentController.addUserScript(
                WKUserScript(
                    source: script,
                    injectionTime: .atDocumentStart,
                    forMainFrameOnly: false
                )
            )
        }
        configuration.applicationNameForUserAgent = "Mixin/\(model.context.appVersion)"
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = context.coordinator
        model.webView = webView
        webView.load(URLRequest(url: model.initialURL))
        return webView
    }

    func updateNSView(_: WKWebView, context _: Context) {}

    private func makeContextScript(_ context: BotWebContext) -> String? {
        let encoder = JSONEncoder()
        guard
            let data = try? encoder.encode(context),
            let object = String(data: data, encoding: .utf8),
            let encodedData = try? encoder.encode(object),
            let encoded = String(data: encodedData, encoding: .utf8)
        else {
            return nil
        }
        return """
        window.MixinContext = {
          getContext: function() {
            return \(encoded);
          }
        };
        """
    }

    @MainActor
    final class Coordinator: NSObject, WKNavigationDelegate {
        private let model: BotWebViewModel

        init(model: BotWebViewModel) {
            self.model = model
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            guard let url = navigationAction.request.url else {
                decisionHandler(.cancel)
                return
            }
            if ["http", "https", "about"].contains(url.scheme?.lowercased() ?? "") {
                decisionHandler(.allow)
            } else {
                NSWorkspace.shared.open(url)
                decisionHandler(.cancel)
            }
        }

        func webView(_ webView: WKWebView, didStartProvisionalNavigation _: WKNavigation?) {
            update(webView, loading: true)
        }

        func webView(_ webView: WKWebView, didFinish _: WKNavigation?) {
            update(webView, loading: false)
        }

        func webView(
            _ webView: WKWebView,
            didFail _: WKNavigation?,
            withError _: Error
        ) {
            update(webView, loading: false)
        }

        func webView(
            _ webView: WKWebView,
            didFailProvisionalNavigation _: WKNavigation?,
            withError _: Error
        ) {
            update(webView, loading: false)
        }

        private func update(_ webView: WKWebView, loading: Bool) {
            model.title = webView.title ?? ""
            model.canGoBack = webView.canGoBack
            model.canGoForward = webView.canGoForward
            model.isLoading = loading
        }
    }
}
