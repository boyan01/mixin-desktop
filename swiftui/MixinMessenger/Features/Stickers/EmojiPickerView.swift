import Foundation
import SwiftUI

struct EmojiPickerView: View {
    let recent: [String]
    let onSelect: (String) -> Void
    @State private var selectedCategory: EmojiCategory = .smileys

    var body: some View {
        VStack(spacing: 0) {
            ScrollView(.horizontal) {
                HStack(spacing: 5) {
                    if !recent.isEmpty {
                        categoryButton(.recent)
                    }
                    ForEach(EmojiCategory.catalogCases) { category in
                        categoryButton(category)
                    }
                }
                .padding(.horizontal, 12)
                .frame(height: 47)
            }
            Divider()
            emojiGrid
        }
    }

    private var emojiGrid: some View {
        ScrollView {
            LazyVGrid(
                columns: Array(repeating: GridItem(.flexible(), spacing: 0), count: 9),
                spacing: 0
            ) {
                ForEach(emojis, id: \.self) { emoji in
                    Button {
                        onSelect(emoji)
                    } label: {
                        Text(emoji)
                            .font(.system(size: 25))
                            .frame(maxWidth: .infinity, minHeight: 39)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .background(Color.secondary.opacity(0.001))
                    .clipShape(RoundedRectangle(cornerRadius: 7))
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
        }
        .overlay {
            if emojis.isEmpty {
                ContentUnavailableView("No Recent Emoji", systemImage: "clock")
            }
        }
    }

    private var emojis: [String] {
        selectedCategory == .recent
            ? recent
            : EmojiCatalog.groups[selectedCategory, default: []]
    }

    private func categoryButton(_ category: EmojiCategory) -> some View {
        Button {
            selectedCategory = category
        } label: {
            Image(systemName: category.icon)
                .frame(width: 30, height: 30)
                .foregroundStyle(selectedCategory == category ? Color.accentColor : .secondary)
                .background(
                    selectedCategory == category
                        ? Color.accentColor.opacity(0.12)
                        : Color.clear
                )
                .clipShape(RoundedRectangle(cornerRadius: 7))
        }
        .buttonStyle(.plain)
        .help(category.title)
    }
}

private enum EmojiCategory: String, CaseIterable, Identifiable {
    case recent
    case smileys
    case animals
    case food
    case travel
    case activities
    case objects
    case symbols
    case flags

    static let catalogCases = allCases.filter { $0 != .recent }

    var id: Self { self }

    var title: String {
        switch self {
        case .recent: "Recent"
        case .smileys: "Smileys & People"
        case .animals: "Animals & Nature"
        case .food: "Food & Drink"
        case .travel: "Travel & Places"
        case .activities: "Activities"
        case .objects: "Objects"
        case .symbols: "Symbols"
        case .flags: "Flags"
        }
    }

    var icon: String {
        switch self {
        case .recent: "clock"
        case .smileys: "face.smiling"
        case .animals: "pawprint"
        case .food: "fork.knife"
        case .travel: "car"
        case .activities: "sportscourt"
        case .objects: "lightbulb"
        case .symbols: "heart"
        case .flags: "flag"
        }
    }
}

private enum EmojiCatalog {
    static let groups: [EmojiCategory: [String]] = build()

    private static func build() -> [EmojiCategory: [String]] {
        var result = Dictionary(
            uniqueKeysWithValues: EmojiCategory.catalogCases.map { ($0, [String]()) }
        )
        let ranges = [
            0x203C ... 0x3299,
            0x1F000 ... 0x1FAFF,
        ]
        for range in ranges {
            for value in range {
                guard let scalar = Unicode.Scalar(value),
                      scalar.properties.isEmoji,
                      !scalar.properties.isEmojiModifier,
                      !scalar.properties.isEmojiModifierBase,
                      !(0x1F1E6 ... 0x1F1FF).contains(value)
                else {
                    continue
                }
                let emoji = scalar.properties.isEmojiPresentation
                    ? String(scalar)
                    : String(scalar) + "\u{FE0F}"
                result[classify(value), default: []].append(emoji)
            }
        }
        result[.flags] = flags()
        return result
    }

    private static func classify(_ value: Int) -> EmojiCategory {
        if (0x1F32D ... 0x1F37F).contains(value)
            || (0x1F950 ... 0x1F96F).contains(value)
            || (0x1FAD0 ... 0x1FADF).contains(value)
        {
            return .food
        }
        if (0x1F3A0 ... 0x1F3FF).contains(value)
            || (0x1F93C ... 0x1F945).contains(value)
        {
            return .activities
        }
        if (0x1F680 ... 0x1F6FF).contains(value)
            || (0x1F3E0 ... 0x1F3F0).contains(value)
            || (0x1F5FA ... 0x1F5FF).contains(value)
        {
            return .travel
        }
        if (0x1F300 ... 0x1F43F).contains(value)
            || (0x1F980 ... 0x1F9AE).contains(value)
            || (0x1FAB0 ... 0x1FABF).contains(value)
        {
            return .animals
        }
        if (0x1F600 ... 0x1F64F).contains(value)
            || (0x1F900 ... 0x1F9FF).contains(value)
            || (0x1FA70 ... 0x1FAFF).contains(value)
        {
            return .smileys
        }
        if (0x1F440 ... 0x1F5F4).contains(value)
            || (0x1F4A0 ... 0x1F5FF).contains(value)
        {
            return .objects
        }
        return .symbols
    }

    private static func flags() -> [String] {
        var regionCodes = Set<String>()
        for identifier in Locale.availableIdentifiers {
            guard let identifier = Locale(identifier: identifier).region?.identifier else {
                continue
            }
            let code = identifier.uppercased()
            let scalars = Array(code.unicodeScalars)
            guard scalars.count == 2,
                  scalars.allSatisfy({ (65 ... 90).contains(Int($0.value)) })
            else {
                continue
            }
            regionCodes.insert(code)
        }

        var result: [String] = []
        for code in regionCodes.sorted() {
            var flag = ""
            for scalar in code.unicodeScalars {
                guard let regional = Unicode.Scalar(127_397 + Int(scalar.value)) else {
                    continue
                }
                flag.append(Character(String(regional)))
            }
            if !flag.isEmpty {
                result.append(flag)
            }
        }
        return result
    }
}
