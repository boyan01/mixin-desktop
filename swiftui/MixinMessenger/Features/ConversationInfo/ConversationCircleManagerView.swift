import Observation
import SwiftUI

struct ConversationCircleManagerView: View {
    @Environment(AccountSession.self) private var session
    @State private var model: ConversationCircleManagerModel
    @State private var creating = false
    @State private var circleName = ""

    init(conversation: SwiftConversationListItem) {
        _model = State(
            initialValue: ConversationCircleManagerModel(
                conversation: conversation
            )
        )
    }

    var body: some View {
        Group {
            switch model.state {
            case .loading:
                ProgressView()
            case let .failed(message):
                ContentUnavailableView(
                    "Unable to Load Circles",
                    systemImage: "circle.grid.2x2",
                    description: Text(message)
                )
            case .ready where model.circles.isEmpty:
                ContentUnavailableView(
                    "No Circles",
                    systemImage: "circle.grid.2x2",
                    description: Text("Create a circle to organize this conversation.")
                )
            case .ready:
                List(model.sortedCircles, id: \.circleId) { circle in
                    Button {
                        Task {
                            await model.toggle(
                                circle,
                                account: session.handle
                            )
                        }
                    } label: {
                        HStack(spacing: 12) {
                            Image(systemName: model.selectedIDs.contains(circle.circleId)
                                ? "minus.circle.fill"
                                : "plus.circle.fill")
                                .foregroundStyle(model.selectedIDs.contains(circle.circleId)
                                    ? .red
                                    : Color.accentColor)
                            Image(systemName: "circle.grid.2x2.fill")
                                .font(.title2)
                                .foregroundStyle(.secondary)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(circle.name)
                                    .foregroundStyle(.primary)
                                Text("\(circle.conversationCount) conversations")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer()
                            if model.updatingID == circle.circleId {
                                ProgressView()
                                    .controlSize(.small)
                            }
                        }
                        .padding(.vertical, 5)
                    }
                    .buttonStyle(.plain)
                    .disabled(model.updatingID != nil)
                }
            }
        }
        .navigationTitle("Circles")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Add Circle", systemImage: "plus") {
                    circleName = ""
                    creating = true
                }
            }
        }
        .task {
            await model.load(account: session.handle)
        }
        .sheet(isPresented: $creating) {
            NavigationStack {
                Form {
                    TextField("Circle name", text: $circleName)
                        .onSubmit(create)
                }
                .formStyle(.grouped)
                .navigationTitle("New Circle")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel") {
                            creating = false
                        }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Create", action: create)
                            .disabled(
                                circleName
                                    .trimmingCharacters(in: .whitespacesAndNewlines)
                                    .isEmpty
                            )
                    }
                }
            }
            .frame(width: 400, height: 180)
        }
        .alert(
            "Unable to Update Circles",
            isPresented: Binding(
                get: { model.operationError != nil },
                set: { if !$0 { model.operationError = nil } }
            )
        ) {
            Button("OK") {
                model.operationError = nil
            }
        } message: {
            Text(model.operationError ?? "")
        }
    }

    private func create() {
        let name = circleName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else {
            return
        }
        Task {
            if await model.create(name: name, account: session.handle) {
                creating = false
            }
        }
    }
}

@MainActor
@Observable
final class ConversationCircleManagerModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    let conversation: SwiftConversationListItem
    private(set) var state = State.loading
    private(set) var circles: [SwiftCircleItem] = []
    private(set) var selectedIDs: Set<String>
    private(set) var updatingID: String?
    var operationError: String?

    init(conversation: SwiftConversationListItem) {
        self.conversation = conversation
        selectedIDs = Set(conversation.circleIds)
    }

    var sortedCircles: [SwiftCircleItem] {
        circles.sorted {
            let lhsSelected = selectedIDs.contains($0.circleId)
            let rhsSelected = selectedIDs.contains($1.circleId)
            if lhsSelected != rhsSelected {
                return lhsSelected
            }
            return $0.name.localizedCaseInsensitiveCompare($1.name)
                == .orderedAscending
        }
    }

    func load(account: SwiftAccountHandle) async {
        state = .loading
        do {
            circles = try await account.circles()
            state = .ready
        } catch {
            state = .failed(MixinErrorPresenter.message(for: error))
        }
    }

    func toggle(
        _ circle: SwiftCircleItem,
        account: SwiftAccountHandle
    ) async {
        guard updatingID == nil else {
            return
        }
        let add = !selectedIDs.contains(circle.circleId)
        if add {
            selectedIDs.insert(circle.circleId)
        } else {
            selectedIDs.remove(circle.circleId)
        }
        updatingID = circle.circleId
        defer { updatingID = nil }
        do {
            try await account.editCircleConversation(
                circleId: circle.circleId,
                conversationId: conversation.conversationId,
                ownerId: conversation.ownerId,
                category: conversation.category,
                add: add
            )
        } catch {
            if add {
                selectedIDs.remove(circle.circleId)
            } else {
                selectedIDs.insert(circle.circleId)
            }
            operationError = MixinErrorPresenter.message(for: error)
        }
    }

    func create(name: String, account: SwiftAccountHandle) async -> Bool {
        guard updatingID == nil else {
            return false
        }
        updatingID = "new"
        defer { updatingID = nil }
        do {
            let circle = try await account.createCircle(name: name)
            try await account.editCircleConversation(
                circleId: circle.circleId,
                conversationId: conversation.conversationId,
                ownerId: conversation.ownerId,
                category: conversation.category,
                add: true
            )
            selectedIDs.insert(circle.circleId)
            circles.append(circle)
            state = .ready
            return true
        } catch {
            operationError = MixinErrorPresenter.message(for: error)
            return false
        }
    }
}
