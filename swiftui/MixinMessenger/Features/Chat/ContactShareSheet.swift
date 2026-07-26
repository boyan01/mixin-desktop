import SwiftUI

struct ContactShareSheet: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.dismiss) private var dismiss
    @State private var users: [SwiftUserItem] = []
    @State private var query = ""
    @State private var loading = true
    @State private var sendingUserID: String?
    @State private var error: String?
    let onSend: (String) async -> Bool

    var body: some View {
        NavigationStack {
            Group {
                if loading {
                    ProgressView("Loading contacts…")
                } else if users.isEmpty, let error {
                    ContentUnavailableView(
                        "Unable to load contacts",
                        systemImage: "exclamationmark.triangle",
                        description: Text(error)
                    )
                } else {
                    List(filteredUsers, id: \.userId) { user in
                        Button {
                            send(user.userId)
                        } label: {
                            HStack(spacing: 12) {
                                MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                                    image.resizable().scaledToFill()
                                } placeholder: {
                                    Image(systemName: "person.crop.circle.fill")
                                        .resizable()
                                        .foregroundStyle(.secondary)
                                }
                                .frame(width: 42, height: 42)
                                .clipShape(Circle())

                                VStack(alignment: .leading, spacing: 3) {
                                    Text(user.fullName)
                                        .foregroundStyle(.primary)
                                    Text(user.identityNumber)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Spacer()
                                if sendingUserID == user.userId {
                                    ProgressView()
                                        .controlSize(.small)
                                }
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .disabled(sendingUserID != nil)
                    }
                    .overlay {
                        if filteredUsers.isEmpty {
                            ContentUnavailableView.search(text: query)
                        }
                    }
                }
            }
            .navigationTitle("Share Contact")
            .searchable(text: $query, prompt: "Search contacts")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                    .disabled(sendingUserID != nil)
                }
            }
            .task {
                await load()
            }
            .alert(
                "Unable to Share Contact",
                isPresented: Binding(
                    get: { error != nil && !users.isEmpty },
                    set: { if !$0 { error = nil } }
                )
            ) {
                Button("OK") {
                    error = nil
                }
            } message: {
                Text(error ?? "")
            }
        }
        .frame(minWidth: 460, minHeight: 560)
    }

    private var filteredUsers: [SwiftUserItem] {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        guard !query.isEmpty else {
            return users
        }
        return users.filter {
            $0.fullName.lowercased().contains(query)
                || $0.identityNumber.contains(query)
        }
    }

    private func load() async {
        loading = true
        error = nil
        do {
            users = try await session.handle.selectableUsers()
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
        loading = false
    }

    private func send(_ userID: String) {
        sendingUserID = userID
        error = nil
        Task {
            if await onSend(userID) {
                dismiss()
            } else {
                error = "The contact could not be shared."
                sendingUserID = nil
            }
        }
    }
}
