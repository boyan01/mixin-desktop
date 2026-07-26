import Foundation
import Observation

@MainActor
@Observable
final class LoginModel {
    enum State {
        case loading
        case ready(String)
        case provisioning(String)
        case failed(String)
    }

    private(set) var state: State = .loading
    private let desktop: SwiftDesktopHandle
    private var login: SwiftLoginHandle?
    private var requestVersion = 0

    init(desktop: SwiftDesktopHandle) {
        self.desktop = desktop
    }

    func run() async throws -> SwiftAccountHandle? {
        requestVersion += 1
        let version = requestVersion
        login?.cancel()
        login = nil
        state = .loading

        while version == requestVersion {
            do {
                let login = try await desktop.beginLogin()
                guard version == requestVersion else {
                    login.cancel()
                    return nil
                }
                self.login = login
                let authURL = login.authUrl()
                state = .ready(authURL)
                let account = try await login.wait()
                guard version == requestVersion else {
                    await account.shutdown()
                    return nil
                }
                self.login = nil
                return account
            } catch SwiftClientError.Cancelled {
                return nil
            } catch {
                guard version == requestVersion else {
                    return nil
                }
                let message = displayMessage(for: error)
                login = nil
                if message.contains("authorization timed out") {
                    state = .loading
                    continue
                }
                if message.contains("login_provisioning_error:") {
                    throw error
                }
                state = .failed(message)
                return nil
            }
        }
        return nil
    }

    func stop() {
        requestVersion += 1
        login?.cancel()
        login = nil
    }

    private func displayMessage(for error: Error) -> String {
        switch error {
        case let SwiftClientError.InvalidArgument(message):
            message
        case let SwiftClientError.Internal(message):
            message
        default:
            MixinErrorPresenter.message(for: error)
        }
    }
}
