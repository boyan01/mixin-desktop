import CoreImage.CIFilterBuiltins
import SwiftUI

struct LoginView: View {
    @Environment(AppModel.self) private var appModel
    @State private var model: LoginModel

    init(desktop: SwiftDesktopHandle) {
        _model = State(initialValue: LoginModel(desktop: desktop))
    }

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            Color(nsColor: .underPageBackgroundColor)
                .ignoresSafeArea()
            loginCard
                .frame(width: 520, height: 418)
                .background(.background)
                .clipShape(RoundedRectangle(cornerRadius: 13))
                .shadow(color: .black.opacity(0.18), radius: 18, y: 8)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            Text(versionText)
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(16)
        }
        .task {
            await authenticate()
        }
        .onDisappear {
            model.stop()
        }
    }

    @ViewBuilder
    private var loginCard: some View {
        switch model.state {
        case .loading:
            ProgressView("Preparing QR code…")
                .controlSize(.large)
        case let .ready(authURL):
            VStack(spacing: 20) {
                QRCodeView(value: authURL)
                    .frame(width: 220, height: 220)
                Text("Scan with Mixin Messenger")
                    .font(.title3.weight(.semibold))
                Text("Open Mixin on your phone and scan this code to sign in.")
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
            .padding(36)
        case let .provisioning(authURL):
            VStack(spacing: 20) {
                QRCodeView(value: authURL)
                    .frame(width: 220, height: 220)
                    .opacity(0.35)
                    .overlay {
                        ProgressView()
                            .controlSize(.large)
                    }
                Text("Finishing sign in…")
                    .font(.title3.weight(.semibold))
            }
        case let .failed(message):
            ContentUnavailableView {
                Label("QR Code Unavailable", systemImage: "qrcode")
            } description: {
                Text(message)
            } actions: {
                Button("Try Again") {
                    Task {
                        await authenticate()
                    }
                }
                .keyboardShortcut(.defaultAction)
            }
        }
    }

    private var versionText: String {
        let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
        let build = Bundle.main.infoDictionary?["CFBundleVersion"] as? String
        return switch (version, build) {
        case let (version?, build?):
            "\(version) (\(build))"
        case let (version?, nil):
            version
        case let (nil, build?):
            build
        case (nil, nil):
            ""
        }
    }

    private func authenticate() async {
        do {
            if let account = try await model.run() {
                appModel.completeLogin(with: account)
            }
        } catch {
            appModel.failLogin(with: error)
        }
    }
}

private struct QRCodeView: View {
    let value: String

    var body: some View {
        if let image = makeImage() {
            ZStack {
                Image(nsImage: image)
                    .interpolation(.none)
                    .resizable()
                    .scaledToFit()
                Image("LoginLogo")
                    .resizable()
                    .scaledToFit()
                    .frame(width: 36, height: 36)
            }
            .padding(10)
            .background(.white)
            .clipShape(RoundedRectangle(cornerRadius: 11))
            .accessibilityLabel("Login QR code")
        } else {
            ContentUnavailableView("Unable to render QR code", systemImage: "qrcode")
        }
    }

    private func makeImage() -> NSImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(value.utf8)
        filter.correctionLevel = "Q"
        guard let output = filter.outputImage else {
            return nil
        }
        let scaled = output.transformed(by: CGAffineTransform(scaleX: 10, y: 10))
        let representation = NSCIImageRep(ciImage: scaled)
        let image = NSImage(size: representation.size)
        image.addRepresentation(representation)
        return image
    }
}
