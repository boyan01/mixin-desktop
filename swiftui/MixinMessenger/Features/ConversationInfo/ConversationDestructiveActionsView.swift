import Observation
import SwiftUI

enum ConversationDestructivePendingAction: String, Identifiable {
    case clear
    case exit
    case delete

    var id: String { rawValue }

    var title: String {
        switch self {
        case .clear:
            "Clear Chat"
        case .exit:
            "Exit Group"
        case .delete:
            "Delete Group"
        }
    }

}

struct ConversationDestructiveActionsView<RelationshipActions: View>: View {

    @Environment(AccountSession.self) private var session
    @Environment(\.mixinTheme) private var theme
    @State private var model = ConversationDestructiveActionsModel()
    @State private var pendingAction: ConversationDestructivePendingAction?

    let conversationID: String
    let isGroup: Bool
    let isExited: Bool
    let relationshipActions: RelationshipActions
    let onConversationDeleted: () -> Void

    init(
        conversationID: String,
        isGroup: Bool,
        isExited: Bool,
        @ViewBuilder relationshipActions: () -> RelationshipActions,
        onConversationDeleted: @escaping () -> Void
    ) {
        self.conversationID = conversationID
        self.isGroup = isGroup
        self.isExited = isExited
        self.relationshipActions = relationshipActions()
        self.onConversationDeleted = onConversationDeleted
    }

    var body: some View {
        VStack(spacing: 0) {
            relationshipActions

            Button("Clear Chat", role: .destructive) {
                pendingAction = .clear
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.leading, 16)
            .padding(.trailing, 10)
            .padding(.vertical, 17)
            .buttonStyle(.plain)

            if isGroup {
                Button(
                    isExited ? "Delete Group" : "Exit Group",
                    role: .destructive
                ) {
                    pendingAction = isExited ? .delete : .exit
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.leading, 16)
                .padding(.trailing, 10)
                .padding(.vertical, 17)
                .buttonStyle(.plain)
            }
        }
        .font(.system(size: 16))
        .foregroundStyle(theme.destructive)
        .background(theme.listSelected)
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .padding(.horizontal, 10)
        .padding(.bottom, 10)
        .frame(maxWidth: 600)
        .disabled(model.isRunning)
        .overlay {
            if model.isRunning {
                ProgressView()
                    .controlSize(.small)
                    .frame(maxWidth: .infinity, alignment: .trailing)
                    .padding(.trailing, 8)
            }
        }
        .confirmationDialog(
            pendingAction?.title ?? "",
            isPresented: Binding(
                get: { pendingAction != nil },
                set: { if !$0 { pendingAction = nil } }
            ),
            titleVisibility: .visible
        ) {
            if let pendingAction {
                Button("Confirm") {
                    Task {
                        let removed = await model.perform(
                            pendingAction,
                            account: session.handle,
                            conversationID: conversationID
                        )
                        if removed {
                            onConversationDeleted()
                        }
                    }
                }
            }
            Button("Cancel", role: .cancel) {}
        }
        .alert(
            "Unable to update conversation",
            isPresented: Binding(
                get: { model.errorMessage != nil },
                set: { if !$0 { model.dismissError() } }
            )
        ) {
            Button("OK") {
                model.dismissError()
            }
        } message: {
            Text(model.errorMessage ?? "")
        }
    }
}

@MainActor
@Observable
final class ConversationDestructiveActionsModel {
    private(set) var isRunning = false
    private(set) var errorMessage: String?

    func perform(
        _ action: ConversationDestructivePendingAction,
        account: SwiftAccountHandle,
        conversationID: String
    ) async -> Bool {
        guard !isRunning else {
            return false
        }
        isRunning = true
        errorMessage = nil
        defer { isRunning = false }

        do {
            switch action {
            case .clear:
                try await account.clearConversation(
                    conversationId: conversationID
                )
                return false
            case .exit:
                try await account.exitGroup(conversationId: conversationID)
                return true
            case .delete:
                try await account.deleteConversation(
                    conversationId: conversationID
                )
                return true
            }
        } catch {
            errorMessage = MixinErrorPresenter.message(for: error)
            return false
        }
    }

    func dismissError() {
        errorMessage = nil
    }
}
