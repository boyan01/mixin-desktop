import AppKit
import SwiftUI

private enum AccountSettingsRoute: Hashable {
    case changeNumber
    case deleteAccount
}

struct AccountSettingsView: View {
    @Environment(AccountSession.self) private var session

    var body: some View {
        NavigationStack {
            Form {
                Section("Account") {
                    LabeledContent("Phone Number") {
                        Text(session.profile.phone.isEmpty ? "Not set" : session.profile.phone)
                    }
                    NavigationLink("Change Number", value: AccountSettingsRoute.changeNumber)
                }

                Section {
                    NavigationLink(
                        "Delete My Account",
                        value: AccountSettingsRoute.deleteAccount
                    )
                    .foregroundStyle(.red)
                } footer: {
                    Text("Manage the phone number and lifecycle of this Mixin account.")
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Account")
            .navigationDestination(for: AccountSettingsRoute.self) { route in
                switch route {
                case .changeNumber:
                    ChangeNumberSettingsView()
                case .deleteAccount:
                    DeleteAccountSettingsView()
                }
            }
        }
    }
}

private struct ChangeNumberSettingsView: View {
    @Environment(AccountSession.self) private var session
    @State private var showConfirmation = false
    @State private var error: String?

    var body: some View {
        Form {
            Section("Current Number") {
                LabeledContent(
                    "Phone Number",
                    value: session.profile.phone.isEmpty ? "Not set" : session.profile.phone
                )
            }

            Section("Verification Required") {
                Text(
                    "Changing a phone number requires wallet PIN verification and an SMS code. "
                        + "This verification flow is currently completed in Mixin Messenger."
                )
                Button(session.profile.phone.isEmpty ? "Add Mobile Number" : "Change Phone Number") {
                    showConfirmation = true
                }
            }

            if let error {
                Section {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
        .navigationTitle("Change Number")
        .confirmationDialog(
            session.profile.phone.isEmpty ? "Add a mobile number?" : "Change your phone number?",
            isPresented: $showConfirmation
        ) {
            Button("Open Mixin Messenger") {
                openMessenger()
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Your wallet PIN and an SMS verification code will be required.")
        }
    }

    private func openMessenger() {
        guard let url = URL(string: "https://mixin.one/messenger") else {
            error = "The Mixin Messenger page could not be opened."
            return
        }
        if !NSWorkspace.shared.open(url) {
            error = "The Mixin Messenger page could not be opened."
        }
    }
}

private struct DeleteAccountSettingsView: View {
    @State private var showConfirmation = false
    @State private var opening = false
    @State private var error: String?

    var body: some View {
        Form {
            Section {
                Label("Delete your account info and profile photo", systemImage: "person.crop.circle.badge.minus")
                Label("Local messages and backups are not deleted automatically", systemImage: "externaldrive")
                Label("Transactions cannot be deleted", systemImage: "exclamationmark.triangle")
            } header: {
                VStack(spacing: 12) {
                    Image(systemName: "person.crop.circle.badge.xmark")
                        .font(.system(size: 64))
                        .foregroundStyle(.red)
                    Text("Delete My Account")
                        .font(.title2.bold())
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 24)
            }

            Section {
                Button("Delete My Account", role: .destructive) {
                    showConfirmation = true
                }
                .disabled(opening)
            } footer: {
                Text(
                    "Account deletion requires wallet PIN and, for phone accounts, SMS "
                        + "verification in Mixin Messenger."
                )
            }

            if let error {
                Section {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
        .navigationTitle("Delete My Account")
        .confirmationDialog(
            "Delete My Account",
            isPresented: $showConfirmation
        ) {
            Button("Continue in Mixin Messenger", role: .destructive) {
                openDeletionFlow()
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text(
                "Your account info and profile photo will be removed. "
                    + "Local messages, backups and transactions remain."
            )
        }
    }

    private func openDeletionFlow() {
        opening = true
        error = nil
        defer { opening = false }
        guard let url = URL(string: "https://mixin.one/messenger"),
              NSWorkspace.shared.open(url)
        else {
            error = "The Mixin Messenger page could not be opened."
            return
        }
    }
}
