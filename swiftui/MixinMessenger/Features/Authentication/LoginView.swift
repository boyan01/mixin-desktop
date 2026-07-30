import CoreImage.CIFilterBuiltins
import SwiftUI

struct LoginView: View {
    @Environment(AppModel.self) private var appModel
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    @State private var model: LoginModel

    init(desktop: SwiftDesktopHandle) {
        _model = State(initialValue: LoginModel(desktop: desktop))
    }

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            (colorScheme == .dark
                ? Color(red: 35 / 255, green: 39 / 255, blue: 43 / 255)
                : Color(red: 229 / 255, green: 229 / 255, blue: 229 / 255))
                .ignoresSafeArea()
            loginCard
                .frame(width: 520, height: 418)
                .background(theme.popUp)
                .clipShape(RoundedRectangle(cornerRadius: 13))
                .shadow(color: .black.opacity(0.18), radius: 18, y: 8)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            Text(versionText)
                .font(.system(size: 14))
                .foregroundStyle(theme.secondaryText)
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
            LoginLoadingContent(title: "Initializing…")
        case let .ready(authURL):
            VStack {
                Spacer()
                LoginQRCodeContent(authURL: authURL)
                Spacer()
            }
        case .provisioning:
            LoginLoadingContent(title: "Loading...")
        case let .failed(message):
            VStack {
                Spacer()
                LoginQRCodeContent(
                    authURL: nil,
                    error: message,
                    onRetry: {
                        Task {
                            await authenticate()
                        }
                    }
                )
                Spacer()
            }
        }
    }

    private var versionText: String {
        let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
        let build = Bundle.main.infoDictionary?["CFBundleVersion"] as? String
        return switch (version, build) {
        case let (version?, build?):
            "\(version)(\(build))"
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

private struct LoginQRCodeContent: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    let authURL: String?
    var error: String?
    var onRetry: (() -> Void)?

    var body: some View {
        VStack(spacing: 0) {
            ZStack {
                if let authURL {
                    QRCodeView(value: authURL)
                }
                if let error, let onRetry {
                    Button(action: onRetry) {
                        VStack(spacing: 14) {
                            Image("LoginRetry")
                                .resizable()
                                .frame(width: 40, height: 40)
                            Text("Click to reload the QR code")
                                .font(.system(size: 14))
                                .multilineTextAlignment(.center)
                        }
                        .foregroundStyle(.white.opacity(0.9))
                        .padding(.horizontal, 8)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .background(.black.opacity(0.86))
                    }
                    .buttonStyle(.plain)
                    .help(error)
                }
            }
            .frame(width: 160, height: 160)
            .clipShape(RoundedRectangle(cornerRadius: 11))

            Spacer().frame(height: 16)
            Text("Log in to Mixin Messenger with a QR code")
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
            Spacer().frame(height: 16)

            VStack(alignment: .leading, spacing: 4) {
                Text("1. Open Mixin Messenger on your phone.")
                Text("2. Scan the QR code on the screen and confirm your sign-in.")
            }
            .font(.system(size: 14))
            .foregroundStyle(
                colorScheme == .dark
                    ? Color.white.opacity(0.4)
                    : Color(red: 187 / 255, green: 190 / 255, blue: 195 / 255)
            )
            .frame(width: 375, alignment: .leading)
            .padding(.horizontal, 20)
        }
    }
}

private struct LoginLoadingContent: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    let title: String

    var body: some View {
        VStack(spacing: 0) {
            ProgressView()
                .tint(theme.text)
            Spacer().frame(height: 24)
            Text(title)
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(theme.text)
            Spacer().frame(height: 8)
            Text("End-to-end encrypted")
                .font(.system(size: 16))
                .foregroundStyle(
                    colorScheme == .dark
                        ? Color.white.opacity(0.4)
                        : Color(red: 188 / 255, green: 190 / 255, blue: 195 / 255)
                )
        }
        .frame(width: 375)
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
            .padding(8)
            .background(.white)
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
