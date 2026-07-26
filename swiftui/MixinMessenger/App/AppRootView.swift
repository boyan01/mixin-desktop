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
    var body: some View {
        VStack(spacing: 16) {
            ProgressView()
                .controlSize(.large)
            Text("Starting Mixin…")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct RecoveryView: View {
    @Environment(AppModel.self) private var appModel
    let recovery: AppRecovery
    @State private var showsDiagnostic = false

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 42))
                .foregroundStyle(.orange)
            Text(title)
                .font(.title2.weight(.semibold))
            Text(recovery.message)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            HStack(spacing: 12) {
                if case .database = recovery.kind {
                    Button("Recreate Database", role: .destructive) {
                        Task {
                            await appModel.recreateAccountDatabase()
                        }
                    }
                }
                if case .savedLogin = recovery.kind {
                    Button("Use Another Account") {
                        Task {
                            await appModel.abortFailedLogin()
                        }
                    }
                }
                Button("Retry") {
                    Task {
                        await appModel.start()
                    }
                }
                .keyboardShortcut(.defaultAction)
            }

            DisclosureGroup("Details", isExpanded: $showsDiagnostic) {
                ScrollView {
                    Text(recovery.diagnostic)
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(10)
                }
                .frame(maxHeight: 140)
            }
            .frame(maxWidth: 520)
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var title: String {
        switch recovery.kind {
        case let .database(failure):
            "Database Error \(failure.resultCode)"
        case .savedLogin:
            "Unable to Restore Account"
        case .startup:
            "Unable to Start Mixin"
        }
    }
}
