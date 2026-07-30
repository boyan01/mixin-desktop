import AppKit
import SwiftUI

private enum AccountSettingsRoute: Hashable {
    case changeNumber
    case deleteAccount
}

struct AccountSettingsView: View {
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        NavigationStack {
            AppScrollView {
                VStack(spacing: 0) {
                    Spacer().frame(height: 40)
                    accountCellGroup {
                        NavigationLink(value: AccountSettingsRoute.changeNumber) {
                            accountCell("Change Number")
                        }
                        .buttonStyle(.plain)
                    }
                    accountCellGroup {
                        NavigationLink(value: AccountSettingsRoute.deleteAccount) {
                            accountCell("Delete My Account")
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .background(theme.background)
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

    private func accountCellGroup<Content: View>(
        @ViewBuilder content: () -> Content
    ) -> some View {
        content()
            .background(theme.settingCellBackground)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .padding(.horizontal, 10)
            .padding(.bottom, 10)
            .frame(maxWidth: 600)
    }

    private func accountCell(_ title: String) -> some View {
        HStack(spacing: 4) {
            Text(title)
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
            Spacer(minLength: 0)
            Image("SettingsArrow")
                .resizable()
                .renderingMode(.template)
                .foregroundStyle(theme.secondaryText)
                .frame(width: 30, height: 30)
        }
        .padding(.leading, 16)
        .padding(.trailing, 10)
        .padding(.vertical, 17)
        .contentShape(Rectangle())
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
    @Environment(\.mixinTheme) private var theme
    @State private var showConfirmation = false
    @State private var opening = false
    @State private var error: String?

    var body: some View {
        AppScrollView {
            VStack(spacing: 0) {
                Spacer().frame(height: 50)
                Image("DeleteAccount")
                    .resizable()
                    .frame(width: 70, height: 72)
                Spacer().frame(height: 20)
                VStack(alignment: .leading, spacing: 0) {
                    warning("Delete your account info and profile photo")
                    warning("Local messages and backups are not deleted automatically")
                    warning("Transactions cannot be deleted")
                }
                .frame(width: 380)
                Spacer().frame(height: 30)
                accountGroup {
                    Button {
                        showConfirmation = true
                    } label: {
                        cell(
                            "Delete My Account",
                            color: theme.destructive,
                            trailing: opening ? AnyView(
                                ProgressView()
                                    .controlSize(.small)
                                    .tint(theme.destructive)
                                    .frame(width: 18, height: 18)
                            ) : AnyView(arrow)
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(opening)
                }
                accountGroup {
                    NavigationLink(value: AccountSettingsRoute.changeNumber) {
                        cell("Change Number Instead", color: theme.text, trailing: AnyView(arrow))
                    }
                    .buttonStyle(.plain)
                }
                if let error {
                    Text(error)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.destructive)
                        .padding(.top, 10)
                }
            }
        }
        .background(theme.background)
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

    private func warning(_ text: String) -> some View {
        HStack(alignment: .top, spacing: 0) {
            Circle()
                .fill(theme.text)
                .frame(width: 4, height: 4)
                .padding(.top, 7)
                .padding(.trailing, 6)
            Text(text)
                .font(.system(size: 14))
                .foregroundStyle(theme.text)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 2)
    }

    private func accountGroup<Content: View>(
        @ViewBuilder content: () -> Content
    ) -> some View {
        content()
            .background(theme.settingCellBackground)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .padding(.horizontal, 10)
            .padding(.bottom, 10)
            .frame(maxWidth: 600)
    }

    private var arrow: some View {
        Image("SettingsArrow")
            .resizable()
            .renderingMode(.template)
            .foregroundStyle(theme.secondaryText)
            .frame(width: 30, height: 30)
    }

    private func cell(_ title: String, color: Color, trailing: AnyView) -> some View {
        HStack(spacing: 4) {
            Text(title)
                .font(.system(size: 16))
                .foregroundStyle(color)
            Spacer(minLength: 4)
            trailing
        }
        .padding(.leading, 16)
        .padding(.trailing, 10)
        .padding(.vertical, 17)
        .contentShape(Rectangle())
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
