import SwiftUI

struct SearchContactSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.dismiss) private var dismiss
    @Environment(\.mixinTheme) private var theme
    @State private var query = ""
    @State private var userID: String?
    @State private var loading = false
    @State private var errorMessage: String?
    @FocusState private var focused: Bool

    var body: some View {
        Group {
            if let userID {
                NavigationStack {
                    MessageUserProfileView(userID: userID)
                        .toolbar {
                            ToolbarItem(placement: .cancellationAction) {
                                Button("Close") {
                                    dismiss()
                                }
                            }
                        }
                }
                .frame(minWidth: 420, minHeight: 560)
            } else {
                searchForm
                    .frame(width: 400)
                    .padding(30)
            }
        }
        .animation(.easeOut(duration: 0.2), value: userID)
        .onAppear {
            focused = true
        }
        .alert(
            "User Not Found",
            isPresented: Binding(
                get: { errorMessage != nil },
                set: { if !$0 { errorMessage = nil } }
            )
        ) {
            Button("OK") {
                errorMessage = nil
            }
        } message: {
            Text(errorMessage ?? "")
        }
    }

    private var searchForm: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Add Contact")
                .font(.system(size: 16))
                .foregroundStyle(theme.text)

            Spacer()
                .frame(height: 24)

            TextField("Mixin ID or phone number", text: $query)
                .textFieldStyle(.roundedBorder)
                .font(.system(size: 14))
                .focused($focused)
                .onSubmit(search)
                .onChange(of: query) {
                    let filtered = query.filter {
                        $0.isNumber || $0 == "+"
                    }
                    query = String(filtered.prefix(128))
                }

            if !session.profile.identityNumber.isEmpty {
                Text("My Mixin ID: \(session.profile.identityNumber)")
                    .font(.system(size: 12))
                    .foregroundStyle(theme.secondaryText)
                    .padding(.top, 8)
            }

            Spacer()
                .frame(height: 30)

            HStack(spacing: 4) {
                Spacer()
                Button("Cancel") {
                    dismiss()
                }
                .buttonStyle(.plain)
                .foregroundStyle(theme.accent)
                .padding(.vertical, 8)
                .padding(.horizontal, 16)

                Button("Search", action: search)
                    .buttonStyle(.borderedProminent)
                    .tint(theme.accent)
                    .disabled(!searchable || loading)
            }
        }
        .overlay {
            if loading {
                ProgressView()
                    .tint(theme.accent)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(theme.popUp.opacity(0.82))
            }
        }
    }

    private var searchable: Bool {
        query.trimmingCharacters(in: .whitespacesAndNewlines).count > 3
    }

    private func search() {
        guard searchable, !loading else {
            return
        }
        loading = true
        Task {
            defer {
                loading = false
            }
            do {
                let user = try await session.handle.searchUser(query: query)
                userID = user.userId
            } catch {
                AppLogger.error(
                    "Search contact failed: query=\(query)",
                    error: error
                )
                errorMessage = MixinErrorPresenter.message(for: error)
            }
        }
    }
}
