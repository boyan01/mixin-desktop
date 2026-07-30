import AppKit
import SwiftUI

struct AppRootView: View {
    @Environment(AppModel.self) private var appModel

    var body: some View {
        Group {
            switch appModel.phase {
            case .launching:
                LaunchingView()
            case .signedOut:
                if let desktop = appModel.desktopHandle {
                    LoginView(desktop: desktop)
                } else {
                    LaunchingView()
                }
            case let .signedIn(session):
                DeviceTransferCoordinatorView(
                    controller: session.deviceTransfer
                ) {
                    AuthGuard(security: session.security) {
                        AccountHealthView(session: session) {
                            HomeView()
                        }
                    }
                }
                .environment(session)
                .focusedSceneValue(\.deviceTransfer, session.deviceTransfer)
            case let .recovery(recovery):
                RecoveryView(recovery: recovery)
            }
        }
        .alert(
            MixinLocalizations.unableToCompleteOperation,
            isPresented: Binding(
                get: { appModel.presentedError != nil },
                set: { if !$0 { appModel.dismissPresentedError() } }
            ),
            actions: {
                Button("OK") {
                    appModel.dismissPresentedError()
                }
            },
            message: {
                Text(appModel.presentedError ?? "")
            }
        )
    }
}

private struct LaunchingView: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            (colorScheme == .dark
                ? Color(red: 35 / 255, green: 39 / 255, blue: 43 / 255)
                : Color(red: 229 / 255, green: 229 / 255, blue: 229 / 255))
                .ignoresSafeArea()
            VStack(spacing: 0) {
                ProgressView()
                    .tint(theme.text)
                Spacer().frame(height: 24)
                Text("Initializing…")
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
}

private struct RecoveryView: View {
    @Environment(AppModel.self) private var appModel
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme
    let recovery: AppRecovery
    @State private var confirmsDatabaseRecreation = false

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            (colorScheme == .dark
                ? Color(red: 35 / 255, green: 39 / 255, blue: 43 / 255)
                : Color(red: 229 / 255, green: 229 / 255, blue: 229 / 255))
                .ignoresSafeArea()
            recoveryContent
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
    }

    @ViewBuilder
    private var recoveryContent: some View {
        switch recovery.kind {
        case .savedLogin:
            savedLoginFailure
        case .database, .startup:
            landingFailure
        }
    }

    private var savedLoginFailure: some View {
        VStack(spacing: 0) {
            Text("Unknown Error")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(theme.destructive)
                .multilineTextAlignment(.center)
            Spacer().frame(height: 16)
            AppScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    Text("Error: \(recovery.message)")
                    Text("StackTrace: \(recovery.diagnostic)")
                }
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.vertical, 20)
            }
            .background(theme.sidebarSelected, in: RoundedRectangle(cornerRadius: 8))
            Spacer().frame(height: 42)
            HStack(spacing: 0) {
                Spacer()
                Button("Copy") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(
                        "Error: \(recovery.message)\nStackTrace: \(recovery.diagnostic)",
                        forType: .string
                    )
                }
                .buttonStyle(RecoverySecondaryButtonStyle())
                Button("Retry") {
                    Task {
                        await appModel.abortFailedLogin()
                    }
                }
                .buttonStyle(RecoveryPrimaryButtonStyle())
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(.top, 56)
        .padding(.bottom, 30)
        .padding(.horizontal, 48)
    }

    private var landingFailure: some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 32)
            Spacer()
            Text(title)
                .font(.system(size: 20, weight: .bold))
                .foregroundStyle(theme.text)
                .padding(.horizontal, 16)
            Spacer().frame(height: 32)
            Text(message)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 30)
            Spacer()
            if canRecreateDatabase {
                Button("Continue", role: .destructive) {
                    confirmsDatabaseRecreation = true
                }
                .font(.system(size: 16))
                .foregroundStyle(theme.destructive)
                .padding(.bottom, 16)
            }
            Button(primaryActionTitle) {
                performPrimaryAction()
            }
            .buttonStyle(RecoveryPrimaryButtonStyle())
            .keyboardShortcut(.defaultAction)
            Spacer().frame(height: 32)
        }
        .alert("Recreate Database?", isPresented: $confirmsDatabaseRecreation) {
            Button("Cancel", role: .cancel) {}
            Button("Create", role: .destructive) {
                Task {
                    await appModel.recreateAccountDatabase()
                }
            }
        } message: {
            Text("The local message database will be deleted and recreated.")
        }
    }

    private var title: String {
        if case .database = recovery.kind {
            "Failed to Open Database"
        } else {
            "Unknown Error"
        }
    }

    private var message: String {
        if case let .database(failure) = recovery.kind {
            failure.explanation
        } else {
            recovery.message
        }
    }

    private var primaryActionTitle: String {
        if case .database = recovery.kind {
            "Exit"
        } else {
            "Retry"
        }
    }

    private func performPrimaryAction() {
        if case .database = recovery.kind {
            NSApplication.shared.terminate(nil)
        } else {
            Task {
                await appModel.start()
            }
        }
    }

    private var canRecreateDatabase: Bool {
        guard case let .database(failure) = recovery.kind else {
            return false
        }
        return [10, 11, 26].contains(failure.resultCode)
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
}

private struct RecoveryPrimaryButtonStyle: ButtonStyle {
    @Environment(\.mixinTheme) private var theme

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 16))
            .foregroundStyle(.white)
            .padding(.horizontal, 56)
            .padding(.vertical, 14)
            .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
            .opacity(configuration.isPressed ? 0.8 : 1)
    }
}

private struct RecoverySecondaryButtonStyle: ButtonStyle {
    @Environment(\.mixinTheme) private var theme

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 16))
            .foregroundStyle(theme.accent)
            .padding(.horizontal, 56)
            .padding(.vertical, 14)
            .opacity(configuration.isPressed ? 0.7 : 1)
    }
}
