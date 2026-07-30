import Observation
import SwiftUI

struct ConversationCircleManagerView: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.mixinTheme) private var theme
    @State private var model: ConversationCircleManagerModel
    @State private var creating = false
    @State private var circleName = ""

    init(conversation: ConversationListData) {
        _model = State(
            initialValue: ConversationCircleManagerModel(
                conversation: conversation
            )
        )
    }

    var body: some View {
        AppScrollView {
            VStack(spacing: 0) {
                circleRows(selected: true)
                if !model.selectedCircles.isEmpty,
                   !model.unselectedCircles.isEmpty
                {
                    Spacer()
                        .frame(height: 10)
                }
                circleRows(selected: false)
            }
        }
        .background(theme.background)
        .navigationTitle("Circles")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    circleName = ""
                    creating = true
                } label: {
                    Image("GenericAdd")
                        .resizable()
                        .renderingMode(.template)
                        .foregroundStyle(theme.icon)
                        .frame(width: 24, height: 24)
                }
                .buttonStyle(MixinActionButtonStyle())
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

    @ViewBuilder
    private func circleRows(selected: Bool) -> some View {
        ForEach(
            selected ? model.selectedCircles : model.unselectedCircles,
            id: \.circleId
        ) { circle in
            CircleManagerRow(
                circle: circle,
                selected: selected
            ) {
                Task {
                    await model.toggle(circle, account: session.handle)
                }
            }
            .disabled(model.updatingID != nil)
        }
    }
}

private struct CircleManagerRow: View {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.mixinTheme) private var theme

    let circle: CircleItem
    let selected: Bool
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 0) {
            Button(action: onToggle) {
                Image(selected ? "CircleRemove" : "CircleAdd")
                    .resizable()
                    .frame(width: 16, height: 16)
                    .frame(width: 48, height: 80)
            }
            .buttonStyle(.plain)

            Spacer()
                .frame(width: 4)

            Circle()
                .fill(
                    colorScheme == .dark
                        ? Color(red: 245 / 255, green: 247 / 255, blue: 250 / 255)
                        : Color(red: 246 / 255, green: 247 / 255, blue: 250 / 255)
                )
                .overlay {
                    Image("CircleGlyph")
                        .resizable()
                        .renderingMode(.template)
                        .foregroundStyle(circle.color)
                        .frame(width: 24, height: 24)
                }
                .frame(width: 50, height: 50)

            Spacer()
                .frame(width: 8)

            VStack(alignment: .leading, spacing: 6) {
                Text(circle.name)
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                Text("\(circle.conversationCount) conversations")
                    .font(.system(size: 14))
                    .foregroundStyle(theme.secondaryText)
            }
        }
        .frame(height: 80)
        .background(theme.primary)
    }
}

private extension CircleItem {
    var color: Color {
        let palette: [Color] = [
            circleColor(0x8E7BFF), circleColor(0x657CFB), circleColor(0xA739C2),
            circleColor(0xBD6DDA), circleColor(0xFD89F1), circleColor(0xFA7B95),
            circleColor(0xE94156), circleColor(0xFA9652), circleColor(0xF1D22B),
            circleColor(0xBAE361), circleColor(0x5EDD5E), circleColor(0x4BE6FF),
            circleColor(0x45B7FE), circleColor(0x00ECD0), circleColor(0xFFCCC0),
            circleColor(0xCEA06B),
        ]
        return palette[circleColorIndex(circleId, paletteCount: palette.count)]
    }
}

private func circleColor(_ value: UInt) -> Color {
    Color(
        red: Double((value >> 16) & 0xFF) / 255,
        green: Double((value >> 8) & 0xFF) / 255,
        blue: Double(value & 0xFF) / 255
    )
}

private func circleColorIndex(_ id: String, paletteCount: Int) -> Int {
    let components = id.trimmingCharacters(in: .whitespacesAndNewlines)
        .split(separator: "-")
    if components.count == 5,
       let first = UInt64(components[0], radix: 16),
       let second = UInt64(components[1], radix: 16),
       let third = UInt64(components[2], radix: 16),
       let fourth = UInt64(components[3], radix: 16),
       let fifth = UInt64(components[4], radix: 16)
    {
        let high = (first << 32) | (second << 16) | third
        let low = (fourth << 48) | fifth
        let hilo = high ^ low
        let upper = Int64(hilo >> 32)
        let signedLower = Int64(
            Int32(bitPattern: UInt32(truncatingIfNeeded: hilo))
        )
        let hash = upper ^ signedLower
        return Int(hash.magnitude % UInt64(paletteCount))
    }

    let hash = id.utf8.reduce(0) { ($0 &* 31) &+ Int($1) }
    return Int(hash.magnitude % UInt(paletteCount))
}

@MainActor
@Observable
final class ConversationCircleManagerModel {
    enum State {
        case loading
        case ready
        case failed(String)
    }

    let conversation: ConversationListData
    private(set) var state = State.loading
    private(set) var circles: [CircleItem] = []
    private(set) var selectedIDs: Set<String>
    private(set) var updatingID: String?
    var operationError: String?

    init(conversation: ConversationListData) {
        self.conversation = conversation
        selectedIDs = Set(conversation.circleIds)
    }

    var selectedCircles: [CircleItem] {
        circles.filter { selectedIDs.contains($0.circleId) }
    }

    var unselectedCircles: [CircleItem] {
        circles.filter { !selectedIDs.contains($0.circleId) }
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
        _ circle: CircleItem,
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
