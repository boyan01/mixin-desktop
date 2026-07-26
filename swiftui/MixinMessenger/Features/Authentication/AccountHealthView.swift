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
    let session: AccountSession
    @State private var fullName = ""
    @State private var isSaving = false
    @State private var errorMessage: String?
    @FocusState private var isFocused: Bool

    var body: some View {
        ZStack {
            Color(nsColor: .windowBackgroundColor)
                .ignoresSafeArea()
            VStack(alignment: .leading, spacing: 18) {
                Text("Set Your Name")
                    .font(.title2.weight(.semibold))
                Text("A name is required before you can continue.")
                    .foregroundStyle(.secondary)
                TextField("Name", text: $fullName)
                    .textFieldStyle(.roundedBorder)
                    .focused($isFocused)
                    .onChange(of: fullName) {
                        if fullName.count > 40 {
                            fullName = String(fullName.prefix(40))
                        }
                    }
                    .onSubmit {
                        save()
                    }
                HStack {
                    Text("\(fullName.count)/40")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button("Continue") {
                        save()
                    }
                    .keyboardShortcut(.defaultAction)
                    .disabled(trimmedName.isEmpty || isSaving)
                }
            }
            .padding(28)
            .frame(width: 400)
            .background(.regularMaterial)
            .clipShape(RoundedRectangle(cornerRadius: 14))
            .shadow(radius: 18, y: 8)
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
    let session: AccountSession
    @State private var isRefreshing = false

    var body: some View {
        VStack(spacing: 24) {
            Text("Synchronizing time…")
                .font(.title3.weight(.medium))
            if isRefreshing {
                ProgressView()
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
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct RequiredUpdateView: View {
    var body: some View {
        VStack(spacing: 14) {
            Text("Update Mixin")
                .font(.title2.weight(.semibold))
            Text("This version is no longer supported. Update Mixin to continue.")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button("Download Update") {
                if let url = URL(string: "https://mixin.one/messenger") {
                    NSWorkspace.shared.open(url)
                }
            }
            .keyboardShortcut(.defaultAction)
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
