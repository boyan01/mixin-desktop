import SwiftUI

struct AuthGuard<Content: View>: View {
    @Environment(\.scenePhase) private var scenePhase
    @Bindable var security: SecurityService
    @ViewBuilder let content: () -> Content

    var body: some View {
        Group {
            if security.isInitialized {
                guardedContent
            } else if let loadError = security.loadError {
                securityFailure(loadError)
            } else {
                ProgressView("Loading security settings…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .onChange(of: scenePhase) {
            switch scenePhase {
            case .active:
                security.appDidBecomeActive()
            case .background, .inactive:
                security.appDidLeaveActive()
            @unknown default:
                break
            }
        }
    }

    private var guardedContent: some View {
        ZStack {
            content()
                .blur(radius: security.isLocked ? 18 : 0)
                .allowsHitTesting(!security.isLocked)
                .accessibilityHidden(security.isLocked)

            if security.isLocked, security.hasPasscode {
                UnlockOverlay(security: security)
                    .transition(.opacity)
                    .zIndex(1)
            }
        }
        .animation(.easeInOut(duration: 0.16), value: security.isLocked)
    }

    private func securityFailure(_ message: String) -> some View {
        ContentUnavailableView {
            Label("Unable to Load Security Settings", systemImage: "lock.trianglebadge.exclamationmark")
        } description: {
            Text(message)
        } actions: {
            Button("Retry") {
                Task {
                    await security.retryLoad()
                }
            }
            .keyboardShortcut(.defaultAction)
        }
    }
}

private struct UnlockOverlay: View {
    @Bindable var security: SecurityService
    @State private var passcode = ""
    @State private var hasError = false
    @State private var biometricInProgress = false
    @FocusState private var passcodeFocused: Bool

    var body: some View {
        ZStack {
            Rectangle()
                .fill(.ultraThinMaterial)
                .ignoresSafeArea()
                .contentShape(Rectangle())
                .onTapGesture {
                    passcodeFocused = true
                }

            VStack(spacing: 0) {
                Image(systemName: "lock.fill")
                    .font(.system(size: 60, weight: .medium))
                    .foregroundStyle(.secondary)

                Text("Unlock with Passcode")
                    .font(.title3.weight(.semibold))
                    .padding(.top, 24)

                SixDigitPasscodeField(
                    value: $passcode,
                    isFocused: $passcodeFocused
                ) { candidate in
                    verify(candidate)
                }
                .padding(.top, 36)

                Text(hasError ? "Passcode incorrect" : " ")
                    .foregroundStyle(.red)
                    .padding(.top, 20)

                if security.biometricEnabled {
                    Button {
                        authenticateWithBiometrics()
                    } label: {
                        if biometricInProgress {
                            ProgressView()
                                .controlSize(.small)
                        } else {
                            Label("Use Touch ID", systemImage: "touchid")
                        }
                    }
                    .buttonStyle(.link)
                    .disabled(biometricInProgress)
                    .padding(.top, 12)
                }
            }
            .padding(40)
        }
        .onAppear {
            passcode = ""
            hasError = false
            passcodeFocused = true
        }
        .onChange(of: security.isLocked) {
            if security.isLocked {
                passcode = ""
                hasError = false
                passcodeFocused = true
            }
        }
        .onChange(of: passcodeFocused) {
            if !passcodeFocused {
                passcodeFocused = true
            }
        }
    }

    private func verify(_ candidate: String) {
        passcode = ""
        if security.unlock(passcode: candidate) {
            hasError = false
        } else {
            hasError = true
            passcodeFocused = true
        }
    }

    private func authenticateWithBiometrics() {
        biometricInProgress = true
        Task {
            _ = await security.unlockWithBiometrics()
            biometricInProgress = false
            if security.isLocked {
                passcodeFocused = true
            }
        }
    }
}
