import AppKit
import CoreImage.CIFilterBuiltins
import Observation
import SwiftUI

enum ProtocolPresentation: Identifiable {
    case snapshot(SwiftSnapshotDetailItem)
    case payment(SwiftCodeResult, URL)
    case group(SwiftCodeResult, String)

    var id: String {
        switch self {
        case let .snapshot(snapshot):
            "snapshot:\(snapshot.snapshotId)"
        case let .payment(result, url):
            "payment:\(result.kind):\(url.absoluteString)"
        case let .group(result, code):
            "group:\(result.conversationId ?? code)"
        }
    }
}

struct ProtocolPresentationSheet: View {
    let presentation: ProtocolPresentation

    var body: some View {
        switch presentation {
        case let .snapshot(snapshot):
            SnapshotDetailDialog(snapshot: snapshot)
        case let .payment(result, url):
            MultisigPaymentDialog(result: result, url: url)
        case let .group(result, code):
            GroupCodeDialog(result: result, code: code)
        }
    }
}

private struct GroupCodeDialog: View {
    @Environment(AccountSession.self) private var session
    @Environment(HomeNavigationModel.self) private var navigation
    @Environment(\.dismiss) private var dismiss
    @State private var joining = false
    @State private var error: String?

    let result: SwiftCodeResult
    let code: String

    var body: some View {
        VStack(spacing: 18) {
            ParticipantAvatarStack(avatars: result.participantAvatars)
                .frame(height: 58)
            Text(result.conversationName?.specialNonEmpty ?? "Group Conversation")
                .font(.title3.weight(.semibold))
            Text("\(result.participantCount) participants")
                .foregroundStyle(.secondary)
            if let error {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }
            HStack {
                Button("Cancel") {
                    dismiss()
                }
                Button(result.alreadyMember ? "Open Group" : "Join Group") {
                    openOrJoin()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(joining)
            }
        }
        .padding(28)
        .frame(width: 360)
    }

    private func openOrJoin() {
        joining = true
        error = nil
        Task {
            do {
                let conversationID: String
                if result.alreadyMember, let existing = result.conversationId {
                    conversationID = existing
                } else {
                    conversationID = try await session.handle.joinGroup(code: code)
                }
                navigation.selectConversation(
                    conversationID,
                    name: result.conversationName
                )
                dismiss()
            } catch {
                self.error = MixinErrorPresenter.message(for: error)
                joining = false
            }
        }
    }
}

private struct ParticipantAvatarStack: View {
    let avatars: [SwiftGroupAvatar]

    var body: some View {
        HStack(spacing: -12) {
            ForEach(Array(avatars.prefix(4).enumerated()), id: \.element.userId) { _, avatar in
                MixinRemoteImage(url: URL(string: avatar.avatarUrl)) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Circle()
                        .fill(.quaternary)
                        .overlay(Text(String(avatar.name.prefix(1))))
                }
                .frame(width: 52, height: 52)
                .clipShape(Circle())
                .overlay(Circle().stroke(.background, lineWidth: 3))
            }
        }
    }
}

struct SpecialSnapshotMessageCard: View {
    let message: SwiftMessageItem

    @State private var presented = false

    var body: some View {
        Button {
            presented = true
        } label: {
            card
        }
        .buttonStyle(.plain)
        .disabled(!message.canOpenSnapshotDetail)
        .help(message.canOpenSnapshotDetail
            ? "Show transaction details"
            : "Transaction details are unavailable")
        .sheet(isPresented: $presented) {
            SnapshotDetailDialog(message: message)
        }
    }

    @ViewBuilder
    private var card: some View {
        if message.category == "SYSTEM_SAFE_INSCRIPTION" {
            HStack(spacing: 10) {
                InscriptionContentView(
                    contentType: message.inscriptionContentType,
                    contentURL: message.inscriptionContentUrl,
                    iconURL: message.inscriptionIconUrl,
                    large: false
                )
                .frame(width: 64, height: 64)
                .clipShape(RoundedRectangle(cornerRadius: 8))

                VStack(alignment: .leading, spacing: 4) {
                    Text(message.inscriptionName?.specialNonEmpty ?? "Collectible")
                        .font(.headline)
                        .lineLimit(1)
                    Text(message.inscriptionHash?.specialNonEmpty ?? "Details unavailable")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
            }
            .frame(maxWidth: 280, alignment: .leading)
        } else {
            HStack(spacing: 10) {
                AssetIcon(
                    assetURL: message.snapshotAssetIconUrl,
                    chainURL: message.snapshotChainIconUrl,
                    size: 42
                )
                VStack(alignment: .leading, spacing: 4) {
                    Text(message.snapshotDisplayAmount)
                        .font(.title3.weight(.semibold))
                    Text(message.snapshotMemo?.decodedHexText.specialNonEmpty ?? "Transaction")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            .frame(minWidth: 190, alignment: .leading)
        }
    }
}

struct SnapshotDetailDialog: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(AccountSession.self) private var session
    @State private var model: SnapshotDetailModel

    init(message: SwiftMessageItem) {
        _model = State(initialValue: SnapshotDetailModel(message: message))
    }

    init(snapshot: SwiftSnapshotDetailItem) {
        _model = State(initialValue: SnapshotDetailModel(snapshot: snapshot))
    }

    var body: some View {
        Group {
            switch model.state {
            case let .content(snapshot, refreshing):
                SnapshotDetailContent(snapshot: snapshot, refreshing: refreshing)
            case let .failed(snapshot, message):
                VStack(spacing: 16) {
                    if let snapshot {
                        SnapshotDetailContent(snapshot: snapshot, refreshing: false)
                            .overlay(alignment: .bottom) {
                                loadFailure(message)
                            }
                    } else {
                        ContentUnavailableView(
                            "Unable to Load Transaction",
                            systemImage: "exclamationmark.triangle",
                            description: Text(message)
                        )
                        .overlay(alignment: .bottom) {
                            Button("Retry") {
                                Task { await model.load(account: session.handle) }
                            }
                            .buttonStyle(.borderedProminent)
                            .padding()
                        }
                    }
                }
            }
        }
        .overlay(alignment: .topTrailing) {
            Button {
                dismiss()
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .font(.title2)
                    .symbolRenderingMode(.hierarchical)
            }
            .buttonStyle(.plain)
            .keyboardShortcut(.cancelAction)
            .padding(14)
        }
        .frame(width: 440)
        .frame(minHeight: 480, maxHeight: 720)
        .task {
            await model.load(account: session.handle)
        }
    }

    private func loadFailure(_ message: String) -> some View {
        HStack(spacing: 12) {
            Label(message, systemImage: "exclamationmark.triangle")
                .font(.caption)
                .lineLimit(2)
            Spacer()
            Button("Retry") {
                Task { await model.load(account: session.handle) }
            }
        }
        .padding(12)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
        .padding()
    }
}

@MainActor
@Observable
final class SnapshotDetailModel {
    enum State {
        case content(SnapshotPresentation, refreshing: Bool)
        case failed(SnapshotPresentation?, String)
    }

    private let source: Source
    private(set) var state: State
    private var loaded = false

    private enum Source {
        case message(SwiftMessageItem)
        case snapshot(SwiftSnapshotDetailItem)
    }

    init(message: SwiftMessageItem) {
        let initial = SnapshotPresentation(message: message)
        source = .message(message)
        state = .content(initial, refreshing: initial.requiresRemoteDetail)
    }

    init(snapshot: SwiftSnapshotDetailItem) {
        source = .snapshot(snapshot)
        state = .content(SnapshotPresentation(snapshot: snapshot), refreshing: false)
    }

    func load(account: SwiftAccountHandle) async {
        guard !loaded else {
            return
        }
        if case let .snapshot(snapshot) = source {
            state = .content(
                SnapshotPresentation(
                    snapshot: snapshot,
                    fiatCurrency: account.profile().fiatCurrency
                ),
                refreshing: false
            )
            loaded = true
            return
        }
        guard case let .message(message) = source else {
            return
        }
        let initial = SnapshotPresentation(message: message)
        guard initial.requiresRemoteDetail,
              let snapshotID = message.snapshotId?.specialNonEmpty
                  ?? SnapshotMessageFallback(raw: message.content).id
        else {
            loaded = true
            if initial.inscription {
                state = .content(initial, refreshing: false)
            } else {
                state = .failed(initial, "The transaction identifier is missing.")
            }
            return
        }

        state = .content(initial, refreshing: true)
        do {
            let detail = if message.category == "SYSTEM_SAFE_SNAPSHOT" {
                try await account.safeSnapshotById(snapshotId: snapshotID)
            } else {
                try await account.snapshotById(snapshotId: snapshotID)
            }
            loaded = true
            state = .content(
                SnapshotPresentation(
                    snapshot: detail,
                    fiatCurrency: account.profile().fiatCurrency
                ),
                refreshing: false
            )
        } catch is CancellationError {
            return
        } catch {
            state = .failed(initial, MixinErrorPresenter.message(for: error))
        }
    }
}

struct SnapshotPresentation {
    let id: String
    let traceID: String
    let type: String
    let amount: String
    let symbol: String
    let assetName: String
    let assetIconURL: String
    let chainIconURL: String
    let opponentName: String
    let currentUserName: String
    let transactionHash: String
    let sender: String
    let receiver: String
    let memo: String
    let confirmations: Int64
    let assetConfirmations: Int64
    let assetTag: String
    let snapshotHash: String
    let openingBalance: String
    let closingBalance: String
    let createdAt: Date?
    let inscription: Bool
    let inscriptionHash: String
    let inscriptionCollectionHash: String
    let inscriptionSequence: Int64?
    let inscriptionContentType: String
    let inscriptionContentURL: String
    let inscriptionName: String
    let inscriptionIconURL: String
    let isSafe: Bool
    let priceUSD: String?
    let fiatRate: Double?
    let tickerPriceUSD: String?
    let fiatCurrency: String
    let depositHash: String
    let withdrawalHash: String
    let withdrawalReceiver: String

    var requiresRemoteDetail: Bool {
        !inscription
    }

    init(message: SwiftMessageItem) {
        let fallback = SnapshotMessageFallback(raw: message.content)
        id = message.snapshotId ?? fallback.id ?? ""
        traceID = ""
        type = message.snapshotType ?? ""
        amount = message.snapshotAmount ?? fallback.amount
        symbol = message.snapshotAssetSymbol ?? fallback.symbol
        assetName = message.snapshotAssetSymbol ?? fallback.symbol
        assetIconURL = message.snapshotAssetIconUrl ?? ""
        chainIconURL = message.snapshotChainIconUrl ?? ""
        opponentName = message.snapshotOpponentId ?? ""
        currentUserName = ""
        transactionHash = message.snapshotTransactionHash ?? ""
        sender = ""
        receiver = ""
        memo = (message.snapshotMemo ?? "").decodedHexText
        confirmations = 0
        assetConfirmations = 0
        assetTag = ""
        snapshotHash = ""
        openingBalance = ""
        closingBalance = ""
        createdAt = message.snapshotCreatedAt.flatMap(Self.parseDate)
        inscription = message.category == "SYSTEM_SAFE_INSCRIPTION"
        inscriptionHash = message.inscriptionHash ?? fallback.id ?? ""
        inscriptionCollectionHash = message.inscriptionCollectionHash ?? ""
        inscriptionSequence = message.inscriptionSequence
        inscriptionContentType = message.inscriptionContentType ?? fallback.contentType ?? ""
        inscriptionContentURL = message.inscriptionContentUrl ?? fallback.contentURL ?? ""
        inscriptionName = message.inscriptionName ?? fallback.name
        inscriptionIconURL = message.inscriptionIconUrl ?? ""
        isSafe = message.category == "SYSTEM_SAFE_SNAPSHOT"
        priceUSD = nil
        fiatRate = nil
        tickerPriceUSD = nil
        fiatCurrency = ""
        depositHash = ""
        withdrawalHash = ""
        withdrawalReceiver = ""
    }

    init(snapshot: SwiftSnapshotDetailItem, fiatCurrency: String = "") {
        id = snapshot.snapshotId
        traceID = snapshot.traceId ?? ""
        type = snapshot.snapshotType
        amount = snapshot.amount
        symbol = snapshot.symbol
        assetName = snapshot.assetName
        assetIconURL = snapshot.assetIconUrl
        chainIconURL = snapshot.chainIconUrl
        opponentName = snapshot.opponentName ?? ""
        currentUserName = snapshot.currentUserName
        transactionHash = snapshot.transactionHash ?? ""
        sender = snapshot.sender ?? ""
        receiver = snapshot.receiver ?? ""
        memo = snapshot.memo ?? ""
        confirmations = Int64(snapshot.confirmations ?? 0)
        assetConfirmations = snapshot.assetConfirmations
        assetTag = snapshot.assetTag ?? ""
        snapshotHash = snapshot.snapshotHash ?? ""
        openingBalance = snapshot.openingBalance ?? ""
        closingBalance = snapshot.closingBalance ?? ""
        createdAt = Date(timeIntervalSince1970: Double(snapshot.createdAtMillis) / 1_000)
        inscription = false
        inscriptionHash = ""
        inscriptionCollectionHash = ""
        inscriptionSequence = nil
        inscriptionContentType = ""
        inscriptionContentURL = ""
        inscriptionName = ""
        inscriptionIconURL = ""
        isSafe = snapshot.isSafe
        priceUSD = snapshot.priceUsd
        fiatRate = snapshot.fiatRate
        tickerPriceUSD = snapshot.tickerPriceUsd
        self.fiatCurrency = fiatCurrency
        depositHash = snapshot.depositHash ?? ""
        withdrawalHash = snapshot.withdrawalHash ?? ""
        withdrawalReceiver = snapshot.withdrawalReceiver ?? ""
    }

    nonisolated private static func parseDate(_ raw: String) -> Date? {
        ISO8601DateFormatter().date(from: raw)
    }
}

private struct SnapshotDetailContent: View {
    let snapshot: SnapshotPresentation
    let refreshing: Bool

    var body: some View {
        if snapshot.inscription {
            inscriptionContent
        } else {
            ScrollView {
                VStack(spacing: 0) {
                    transactionHeader
                    if let valuesDescription {
                        Text(valuesDescription)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 24)
                    }
                    Divider()
                        .padding(.top, 24)
                    transactionDetails
                }
            }
            .overlay(alignment: .topLeading) {
                if refreshing {
                    ProgressView()
                        .controlSize(.small)
                        .padding(16)
                        .help("Refreshing transaction details")
                }
            }
        }
    }

    private var transactionHeader: some View {
        VStack(spacing: 14) {
            AssetIcon(
                assetURL: snapshot.assetIconURL,
                chainURL: snapshot.chainIconURL,
                size: 60
            )
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                Text(snapshot.amount.formattedDecimal)
                    .font(.system(size: 32, weight: .bold, design: .rounded))
                    .foregroundStyle(amountColor)
                    .textSelection(.enabled)
                Text(snapshot.symbol)
                    .font(.callout)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 54)
        .padding(.bottom, 18)
    }

    private var amountColor: Color {
        if snapshot.type == "pending" {
            return .primary
        }
        return (Decimal(string: snapshot.amount) ?? 0) > 0 ? .green : .red
    }

    private var valuesDescription: String? {
        guard let price = snapshot.priceUSD.flatMap({ Decimal(string: $0) }),
              let rateValue = snapshot.fiatRate,
              let amount = Decimal(string: snapshot.amount),
              amount != 0
        else {
            return nil
        }
        let rate = Decimal(rateValue)
        let current = abs(amount) * price * rate
        let unit = current / abs(amount)
        let currency = snapshot.fiatCurrency.specialNonEmpty ?? "USD"
        let now = "Value now \(current.currency(currency)) (\(unit.currency(currency))/\(snapshot.symbol))"
        guard let ticker = snapshot.tickerPriceUSD.flatMap({ Decimal(string: $0) }) else {
            return now
        }
        if ticker == 0 {
            return "\(now)\nValue then N/A"
        }
        let past = abs(amount) * ticker * rate
        return "\(now)\nValue then \(past.currency(currency)) (\((past / abs(amount)).currency(currency))/\(snapshot.symbol))"
    }

    private var transactionDetails: some View {
        LazyVStack(alignment: .leading, spacing: 0) {
            ForEach(rows, id: \.title) { row in
                VStack(alignment: .leading, spacing: 7) {
                    Text(row.title)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                    Text(row.value)
                        .textSelection(.enabled)
                }
                .padding(.vertical, 12)
            }
        }
        .padding(.horizontal, 24)
        .padding(.bottom, 24)
    }

    private var rows: [SnapshotDetailRow] {
        snapshot.isSafe ? safeRows : accountRows
    }

    private var accountRows: [SnapshotDetailRow] {
        let positive = (Decimal(string: snapshot.amount) ?? 0) > 0
        var rows = [
            SnapshotDetailRow("Transaction ID", snapshot.id),
            SnapshotDetailRow("Snapshot Hash", snapshot.snapshotHash),
            SnapshotDetailRow("Asset", snapshot.assetName),
            SnapshotDetailRow("Transaction Type", snapshot.type.localizedSnapshotType),
        ]
        switch snapshot.type {
        case "deposit":
            rows.append(SnapshotDetailRow("From", snapshot.sender))
            rows.append(SnapshotDetailRow("Transaction Hash", snapshot.transactionHash))
        case "pending":
            rows.append(SnapshotDetailRow(
                "Status",
                "Pending confirmation \(snapshot.confirmations)/\(snapshot.assetConfirmations)"
            ))
            rows.append(SnapshotDetailRow("From", snapshot.sender))
            rows.append(SnapshotDetailRow("Transaction Hash", snapshot.transactionHash))
        case "transfer":
            rows.append(SnapshotDetailRow(
                "From",
                positive ? snapshot.opponentName : snapshot.currentUserName
            ))
            rows.append(SnapshotDetailRow(
                "Receiver",
                positive ? snapshot.currentUserName : snapshot.opponentName
            ))
        default:
            rows.append(SnapshotDetailRow("Transaction Hash", snapshot.transactionHash))
            rows.append(SnapshotDetailRow(
                snapshot.assetTag.isEmpty ? "Receiver" : "Address",
                snapshot.receiver
            ))
        }
        rows.append(contentsOf: commonRows)
        if snapshot.type == "transfer" {
            rows.append(SnapshotDetailRow("Trace", snapshot.traceID))
        }
        rows.append(SnapshotDetailRow("Inscription", snapshot.inscriptionHash))
        rows.append(SnapshotDetailRow("Collection", snapshot.inscriptionCollectionHash))
        if let sequence = snapshot.inscriptionSequence {
            rows.append(SnapshotDetailRow("Sequence", String(sequence)))
        }
        return rows.filter(\.hasValue)
    }

    private var safeRows: [SnapshotDetailRow] {
        let positive = (Decimal(string: snapshot.amount) ?? 0) > 0
        var rows: [SnapshotDetailRow] = []
        if snapshot.type == "pending" {
            rows.append(SnapshotDetailRow(
                "Status",
                "Pending confirmation \(snapshot.confirmations)/\(snapshot.assetConfirmations)"
            ))
            rows.append(SnapshotDetailRow("Deposit Hash", snapshot.depositHash))
        } else {
            rows.append(SnapshotDetailRow("Transaction ID", snapshot.id))
            rows.append(SnapshotDetailRow("Transaction Hash", snapshot.transactionHash))
        }
        switch snapshot.type {
        case "transfer":
            rows.append(SnapshotDetailRow(
                positive ? "From" : "To",
                snapshot.opponentName.specialNonEmpty ?? "N/A"
            ))
            rows.append(SnapshotDetailRow("Memo", snapshot.memo))
        case "deposit":
            rows.append(SnapshotDetailRow("Deposit Hash", snapshot.depositHash))
        case "withdrawal":
            rows.append(SnapshotDetailRow("To", snapshot.withdrawalReceiver))
            rows.append(SnapshotDetailRow(
                "Withdrawal Hash",
                snapshot.withdrawalHash.specialNonEmpty ?? "Waiting for transaction hash…"
            ))
        default:
            break
        }
        if let createdAt = snapshot.createdAt {
            rows.append(SnapshotDetailRow(
                "Time",
                createdAt.formatted(date: .long, time: .standard)
            ))
        }
        return rows.filter(\.hasValue)
    }

    private var commonRows: [SnapshotDetailRow] {
        var rows = [
            SnapshotDetailRow("Memo", snapshot.memo),
            SnapshotDetailRow(
                "Opening Balance",
                snapshot.openingBalance.isEmpty
                    ? ""
                    : "\(snapshot.openingBalance) \(snapshot.symbol)"
            ),
            SnapshotDetailRow(
                "Closing Balance",
                snapshot.closingBalance.isEmpty
                    ? ""
                    : "\(snapshot.closingBalance) \(snapshot.symbol)"
            ),
        ]
        if let createdAt = snapshot.createdAt {
            rows.append(SnapshotDetailRow(
                "Time",
                createdAt.formatted(date: .long, time: .standard)
            ))
        }
        return rows
    }

    private var inscriptionContent: some View {
        ZStack {
            Color.black.opacity(0.92)
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    InscriptionContentView(
                        contentType: snapshot.inscriptionContentType,
                        contentURL: snapshot.inscriptionContentURL,
                        iconURL: snapshot.inscriptionIconURL,
                        large: true
                    )
                    .aspectRatio(1, contentMode: .fit)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

                    InscriptionInfoRow(title: "Hash", value: snapshot.inscriptionHash)
                    InscriptionInfoRow(
                        title: "ID",
                        value: snapshot.inscriptionSequence.map { "#\($0)" } ?? "N/A"
                    )
                    InscriptionInfoRow(
                        title: "Collection",
                        value: snapshot.inscriptionName.specialNonEmpty ?? "N/A"
                    )
                }
                .padding(.horizontal, 30)
                .padding(.top, 54)
                .padding(.bottom, 30)
            }
        }
        .foregroundStyle(.white)
    }
}

private struct SnapshotDetailRow {
    let title: String
    let value: String

    init(_ title: String, _ value: String) {
        self.title = title
        self.value = value
    }

    var hasValue: Bool {
        !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

private struct InscriptionInfoRow: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .foregroundStyle(.white.opacity(0.45))
            if title == "Hash" {
                ColoredHashView(value: value)
            }
            Text(value)
                .textSelection(.enabled)
        }
    }
}

private struct ColoredHashView: View {
    let value: String

    var body: some View {
        HStack(spacing: 4) {
            ForEach(Array(value.utf8.prefix(16).enumerated()), id: \.offset) { _, byte in
                RoundedRectangle(cornerRadius: 2)
                    .fill(Color(
                        hue: Double(byte) / 255,
                        saturation: 0.72,
                        brightness: 0.92
                    ))
                    .frame(maxWidth: .infinity)
            }
        }
        .frame(height: 24)
    }
}

private struct InscriptionContentView: View {
    let contentType: String?
    let contentURL: String?
    let iconURL: String?
    let large: Bool

    @State private var textContent: String?
    @State private var textError = false

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: large ? 12 : 8)
                .fill(.white.opacity(0.08))
            if contentType?.hasPrefix("image") == true,
               let url = contentURL.flatMap(URL.init(string:))
            {
                MixinAsyncImage(url: url) { phase in
                    switch phase {
                    case let .success(image):
                        image.resizable().scaledToFill()
                    case .empty:
                        ProgressView()
                    default:
                        placeholder
                    }
                }
            } else if contentType?.hasPrefix("text") == true {
                VStack(spacing: large ? 16 : 6) {
                    MixinRemoteImage(url: iconURL.flatMap(URL.init(string:))) { image in
                        image.resizable().scaledToFit()
                    } placeholder: {
                        Image(systemName: "hexagon.fill")
                            .foregroundStyle(.orange)
                    }
                    .frame(width: large ? 92 : 24, height: large ? 92 : 24)
                    if let textContent {
                        Text(textContent)
                            .font(large ? .title2.bold() : .caption.bold())
                            .foregroundStyle(.orange)
                            .multilineTextAlignment(.center)
                            .lineLimit(large ? 5 : 2)
                    } else if textError {
                        Text("Unable to load")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    } else {
                        ProgressView()
                    }
                }
                .padding()
                .task(id: contentURL) {
                    await loadText()
                }
            } else {
                placeholder
            }
        }
        .clipped()
    }

    private var placeholder: some View {
        Image(systemName: "seal.fill")
            .font(large ? .system(size: 64) : .title2)
            .foregroundStyle(.secondary)
    }

    private func loadText() async {
        textContent = nil
        textError = false
        guard let url = contentURL.flatMap(URL.init(string:)) else {
            textError = true
            return
        }
        do {
            let (data, response) = try await URLSession.shared.data(from: url)
            guard let http = response as? HTTPURLResponse,
                  (200 ..< 300).contains(http.statusCode)
            else {
                textError = true
                return
            }
            textContent = String(decoding: data, as: UTF8.self)
                .trimmingCharacters(in: .whitespacesAndNewlines)
        } catch is CancellationError {
            return
        } catch {
            textError = true
        }
    }
}

private struct AssetIcon: View {
    let assetURL: String?
    let chainURL: String?
    let size: CGFloat

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            MixinRemoteImage(url: assetURL.flatMap(URL.init(string:))) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Circle()
                    .fill(.secondary.opacity(0.18))
                    .overlay {
                        Image(systemName: "bitcoinsign.circle")
                            .foregroundStyle(.secondary)
                    }
            }
            .frame(width: size, height: size)
            .clipShape(Circle())

            if let url = chainURL.flatMap(URL.init(string:)) {
                MixinRemoteImage(url: url) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Circle().fill(.background)
                }
                .frame(width: size * 0.28, height: size * 0.28)
                .clipShape(Circle())
                .overlay(Circle().stroke(.background, lineWidth: 2))
            }
        }
    }
}

struct MultisigPaymentDialog: View {
    @Environment(\.dismiss) private var dismiss
    let result: SwiftCodeResult
    let url: URL

    private var done: Bool {
        ["signed", "unlocked", "paid"].contains(result.state?.lowercased() ?? "")
    }

    private var users: [String: SwiftGroupAvatar] {
        Dictionary(uniqueKeysWithValues: result.participantAvatars.map { ($0.userId, $0) })
    }

    var body: some View {
        VStack(spacing: 22) {
            HStack {
                Text(title)
                    .font(.headline)
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.plain)
                .keyboardShortcut(.cancelAction)
            }

            HStack(spacing: 14) {
                ParticipantStack(ids: result.senders, users: users)
                Image(systemName: "arrow.right")
                    .foregroundStyle(.green)
                ParticipantStack(ids: result.receivers, users: users)
            }

            AssetIcon(
                assetURL: result.assetIconUrl,
                chainURL: result.chainIconUrl,
                size: 52
            )
            Text("\((result.amount ?? "").formattedDecimal) \(result.assetSymbol ?? "")")
                .font(.title3.weight(.semibold))
                .textSelection(.enabled)

            if done {
                VStack(spacing: 10) {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 72))
                        .foregroundStyle(.green)
                    Text("Done")
                        .foregroundStyle(.secondary)
                }
                .padding(.vertical, 26)
            } else {
                PaymentQRCode(value: url.absoluteString)
                    .frame(width: 190, height: 190)
                    .padding(8)
                    .background(.white, in: RoundedRectangle(cornerRadius: 10))
                Text(actionDescription)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                HStack {
                    Button("Copy Link") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(url.absoluteString, forType: .string)
                    }
                    Button(localActionTitle) {}
                        .disabled(true)
                        .help("This transaction must be approved by a compatible Mixin wallet.")
                }
            }
        }
        .padding(24)
        .frame(width: 380)
    }

    private var title: String {
        result.kind == "multisig_request" && result.action == "unlock"
            ? "Revoke Multisig Transaction"
            : "Multisig Transaction"
    }

    private var actionDescription: String {
        result.kind == "payment"
            ? "Scan this QR code with a Mixin wallet to complete the payment."
            : "Scan this QR code with a participating Mixin wallet to sign the transaction."
    }

    private var localActionTitle: String {
        result.kind == "payment" ? "Pay on This Mac" : "Sign on This Mac"
    }
}

private struct ParticipantStack: View {
    let ids: [String]
    let users: [String: SwiftGroupAvatar]

    var body: some View {
        HStack(spacing: -7) {
            ForEach(Array(ids.prefix(ids.count <= 3 ? 3 : 2)), id: \.self) { id in
                participant(id)
            }
            if ids.count > 3 {
                Text("+\(ids.count - 2)")
                    .font(.caption2)
                    .frame(width: 28, height: 28)
                    .background(.quaternary, in: Circle())
                    .overlay(Circle().stroke(.background, lineWidth: 2))
            }
        }
    }

    @ViewBuilder
    private func participant(_ id: String) -> some View {
        if let user = users[id] {
            MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Image(systemName: "person.crop.circle.fill")
                    .resizable()
                    .foregroundStyle(.secondary)
            }
            .frame(width: 28, height: 28)
            .clipShape(Circle())
            .overlay(Circle().stroke(.background, lineWidth: 2))
            .help(user.name)
        } else {
            Image(systemName: "person.crop.circle.fill")
                .resizable()
                .foregroundStyle(.secondary)
                .frame(width: 28, height: 28)
        }
    }
}

private struct PaymentQRCode: View {
    let value: String

    var body: some View {
        if let image = makeImage() {
            Image(nsImage: image)
                .interpolation(.none)
                .resizable()
                .scaledToFit()
                .accessibilityLabel("Payment or signature QR code")
        } else {
            ContentUnavailableView("Unable to render QR code", systemImage: "qrcode")
        }
    }

    private func makeImage() -> NSImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(value.utf8)
        filter.correctionLevel = "M"
        guard let output = filter.outputImage else {
            return nil
        }
        let scaled = output.transformed(by: CGAffineTransform(scaleX: 10, y: 10))
        let representation = NSCIImageRep(ciImage: scaled)
        let image = NSImage(size: representation.size)
        image.addRepresentation(representation)
        return image
    }
}

private struct SnapshotMessageFallback {
    let id: String?
    let amount: String
    let symbol: String
    let name: String
    let contentType: String?
    let contentURL: String?

    init(raw: String) {
        guard let data = raw.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            id = nil
            amount = ""
            symbol = ""
            name = ""
            contentType = nil
            contentURL = nil
            return
        }
        id = (json["snapshot_id"] as? String)?.specialNonEmpty
            ?? (json["inscription_hash"] as? String)?.specialNonEmpty
        amount = json["amount"].map(String.init(describing:)) ?? ""
        symbol = (json["symbol"] as? String)
            ?? (json["asset_symbol"] as? String)
            ?? ""
        name = (json["name"] as? String) ?? ""
        contentType = (json["content_type"] as? String)?.specialNonEmpty
        contentURL = (json["content_url"] as? String)?.specialNonEmpty
    }
}

private extension SwiftMessageItem {
    var canOpenSnapshotDetail: Bool {
        if category == "SYSTEM_SAFE_INSCRIPTION" {
            return inscriptionHash?.specialNonEmpty != nil
                || SnapshotMessageFallback(raw: content).id != nil
        }
        return snapshotId?.specialNonEmpty != nil
            || SnapshotMessageFallback(raw: content).id != nil
    }

    var snapshotDisplayAmount: String {
        let fallback = SnapshotMessageFallback(raw: content)
        let amount = (snapshotAmount ?? fallback.amount).formattedDecimal
        let symbol = snapshotAssetSymbol ?? fallback.symbol
        return [amount, symbol]
            .filter { !$0.isEmpty }
            .joined(separator: " ")
    }
}

private extension String {
    var specialNonEmpty: String? {
        let value = trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty ? nil : value
    }

    var formattedDecimal: String {
        guard let value = Decimal(string: self) else {
            return self
        }
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = 8
        formatter.minimumFractionDigits = 0
        return formatter.string(from: value as NSDecimalNumber) ?? self
    }

    var decodedHexText: String {
        guard !isEmpty, count.isMultiple(of: 2),
              allSatisfy(\.isHexDigit)
        else {
            return self
        }
        let bytes = stride(from: 0, to: count, by: 2).compactMap { offset -> UInt8? in
            let start = index(startIndex, offsetBy: offset)
            let end = index(start, offsetBy: 2)
            return UInt8(self[start ..< end], radix: 16)
        }
        return bytes.count * 2 == count ? String(decoding: bytes, as: UTF8.self) : self
    }

    var localizedSnapshotType: String {
        switch self {
        case "deposit": "Deposit"
        case "withdrawal": "Withdrawal"
        case "fee": "Fee"
        case "rebate": "Rebate"
        case "raw": "Raw"
        case "transfer": "Transfer"
        default: "N/A"
        }
    }
}

private extension Decimal {
    func currency(_ code: String) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = code
        return formatter.string(from: self as NSDecimalNumber) ?? description
    }
}
