import Foundation
import Observation

@MainActor
@Observable
final class AppModel {
    private(set) var phase: AppPhase = .launching
    private(set) var presentedError: String?
    let noticeCenter: MixinNoticeCenter
    let preferences: SettingsPreferencesModel
    private var desktop: SwiftDesktopHandle?
    private var requestVersion = 0

    init() {
        let noticeCenter = MixinNoticeCenter()
        self.noticeCenter = noticeCenter
        preferences = SettingsPreferencesModel(noticeCenter: noticeCenter)
    }

    var desktopHandle: SwiftDesktopHandle? {
        desktop
    }

    func start() async {
        let version = nextRequestVersion()
        phase = .launching
        presentedError = nil

        do {
            let desktop: SwiftDesktopHandle
            if let existing = self.desktop {
                desktop = existing
            } else {
                desktop = try await openDesktop()
                try MixinImagePipeline.configure(desktop: desktop)
            }
            guard version == requestVersion else {
                return
            }
            self.desktop = desktop
            await preferences.load(desktop: desktop)

            let account = try await desktop.restoreAccount()
            guard version == requestVersion else {
                await account.shutdown()
                return
            }
            phase = .signedIn(makeSession(handle: account))
        } catch SwiftClientError.NotFound {
            guard version == requestVersion else {
                return
            }
            phase = .signedOut
        } catch {
            guard version == requestVersion else {
                return
            }
            AppLogger.error("App startup failed", error: error)
            phase = .recovery(recovery(for: error))
        }
    }

    func completeLogin(with account: SwiftAccountHandle) {
        _ = nextRequestVersion()
        presentedError = nil
        phase = .signedIn(makeSession(handle: account))
    }

    func failLogin(with error: Error) {
        _ = nextRequestVersion()
        phase = .recovery(recovery(for: error))
    }

    func abortFailedLogin() async {
        guard let desktop else {
            await start()
            return
        }
        do {
            try await desktop.abortSavedLogin()
            _ = nextRequestVersion()
            presentedError = nil
            phase = .signedOut
        } catch {
            AppLogger.error("Abort saved login failed", error: error)
            presentedError = displayMessage(for: error)
        }
    }

    func recreateAccountDatabase() async {
        guard let desktop else {
            await start()
            return
        }
        phase = .launching
        do {
            try await desktop.recreateAccountDatabase()
            await start()
        } catch {
            AppLogger.error("Recreate account database failed", error: error)
            phase = .recovery(recovery(for: error))
        }
    }

    func signOut() async {
        guard case let .signedIn(session) = phase else {
            return
        }
        _ = nextRequestVersion()
        do {
            try await session.signOut()
        } catch {
            AppLogger.error("Sign out failed", error: error)
            presentedError = displayMessage(for: error)
            await session.shutdown()
        }
        phase = .signedOut
    }

    func dismissPresentedError() {
        presentedError = nil
    }

    private func nextRequestVersion() -> Int {
        requestVersion += 1
        return requestVersion
    }

    private func makeSession(handle: SwiftAccountHandle) -> AccountSession {
        guard let desktop else {
            AppLogger.error("Create account session failed: desktop handle unavailable")
            preconditionFailure("A desktop handle must exist before creating an account session")
        }
        let session = AccountSession(handle: handle, desktop: desktop)
        session.start()
        return session
    }

    private func recovery(for error: Error) -> AppRecovery {
        let message = displayMessage(for: error)
        if let failure = DatabaseOpenFailure.parse(message) {
            return AppRecovery(
                kind: .database(failure),
                message: "The account database could not be opened.",
                diagnostic: message
            )
        }
        if desktop != nil {
            return AppRecovery(
                kind: .savedLogin,
                message: "The saved account could not be restored.",
                diagnostic: message
            )
        }
        return AppRecovery(
            kind: .startup,
            message: "Mixin could not start.",
            diagnostic: message
        )
    }

    private func displayMessage(for error: Error) -> String {
        MixinErrorPresenter.message(for: error)
    }
}
