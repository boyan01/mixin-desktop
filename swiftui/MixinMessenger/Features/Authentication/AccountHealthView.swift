import AppKit
import SwiftUI

struct AccountHealthView<Content: View>: View {
    let session: AccountSession
    let content: Content

    init(session: AccountSession, @ViewBuilder content: () -> Content) {
        self.session = session
        self.content = content()
    }

    var body: some View {
        ZStack {
            content
            if session.profile.fullName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                ProfileSetupView(session: session)
            }
            if session.health == "time_inaccurate" {
                LocalTimeErrorView(session: session)
            }
            if session.health == "update_required" {
                RequiredUpdateView()
            }
        }
    }
}

private struct ProfileSetupView: View {
    @Environment(\.mixinTheme) private var theme
    let session: AccountSession
    @State private var fullName = ""
    @State private var isSaving = false
    @State private var errorMessage: String?
    @FocusState private var isFocused: Bool

    var body: some View {
        ZStack {
            theme.background
                .ignoresSafeArea()
            VStack(alignment: .leading, spacing: 0) {
                Text("Edit Name")
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                Spacer().frame(height: 48)
                TextField("Name", text: $fullName)
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(theme.text)
                    .textFieldStyle(.plain)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 12)
                    .frame(minHeight: 48)
                    .background(theme.background, in: RoundedRectangle(cornerRadius: 5))
                    .focused($isFocused)
                    .onChange(of: fullName) {
                        if fullName.count > 40 {
                            fullName = String(fullName.prefix(40))
                        }
                    }
                    .onSubmit {
                        save()
                    }
                Spacer().frame(height: 30)
                HStack {
                    Spacer()
                    Button("Confirm") { save() }
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(.white)
                        .padding(.vertical, 8)
                        .padding(.horizontal, 16)
                        .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
                        .disabled(trimmedName.isEmpty || isSaving)
                        .opacity(trimmedName.isEmpty || isSaving ? 0.4 : 1)
                        .keyboardShortcut(.defaultAction)
                }
            }
            .padding(30)
            .frame(minWidth: 400, minHeight: 210)
        }
        .onAppear {
            isFocused = true
        }
        .alert(
            "Unable to Update Profile",
            isPresented: Binding(
                get: { errorMessage != nil },
                set: { if !$0 { errorMessage = nil } }
            ),
            actions: {
                Button("OK") {
                    errorMessage = nil
                }
            },
            message: {
                Text(errorMessage ?? "")
            }
        )
    }

    private var trimmedName: String {
        fullName.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func save() {
        guard !trimmedName.isEmpty, !isSaving else {
            return
        }
        isSaving = true
        Task {
            defer {
                isSaving = false
            }
            do {
                try await session.updateProfile(
                    fullName: trimmedName,
                    biography: session.profile.biography
                )
            } catch {
                errorMessage = MixinErrorPresenter.message(for: error)
            }
        }
    }
}

private struct LocalTimeErrorView: View {
    @Environment(\.mixinTheme) private var theme
    let session: AccountSession
    @State private var isRefreshing = false

    var body: some View {
        VStack(spacing: 0) {
            Text("Synchronizing time…")
                .font(.system(size: 16))
                .foregroundStyle(theme.text)
            Spacer().frame(height: 24)
            if isRefreshing {
                ProgressView()
                    .tint(theme.accent)
            } else {
                Button("Continue") {
                    isRefreshing = true
                    Task {
                        defer {
                            isRefreshing = false
                        }
                        try? await session.refreshAccountHealth()
                    }
                }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(HealthActionButtonStyle())
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme.background)
    }
}

private struct RequiredUpdateView: View {
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            VStack(spacing: 0) {
                Text("Update Mixin")
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                Spacer().frame(height: 10)
                Text(
                    "The current version (\(shortVersion)) is no longer available!\n"
                        + "Please click \"Update\" below to update to the latest version."
                )
                    .font(.system(size: 14))
                    .foregroundStyle(theme.text)
                    .multilineTextAlignment(.center)
                Spacer().frame(height: 32)
                Button("Upgrade") {
                    if let url = URL(string: "https://mixin.one/messenger") {
                        NSWorkspace.shared.open(url)
                    }
                }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(HealthActionButtonStyle())
            }
            Text(versionText)
                .font(.system(size: 12))
                .foregroundStyle(theme.secondaryText)
                .padding(16)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme.background)
    }

    private var versionText: String {
        let version = shortVersion
        let build = Bundle.main.object(
            forInfoDictionaryKey: "CFBundleVersion"
        ) as? String ?? ""
        return version.isEmpty ? "" : "\(version)+\(build)"
    }

    private var shortVersion: String {
        Bundle.main.object(
            forInfoDictionaryKey: "CFBundleShortVersionString"
        ) as? String ?? ""
    }
}

private struct HealthActionButtonStyle: ButtonStyle {
    @Environment(\.mixinTheme) private var theme

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(.white)
            .padding(.vertical, 8)
            .padding(.horizontal, 16)
            .background(theme.accent, in: RoundedRectangle(cornerRadius: 5))
            .opacity(configuration.isPressed ? 0.8 : 1)
    }
}
