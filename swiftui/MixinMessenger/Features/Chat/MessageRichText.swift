import Foundation
import SwiftUI

/// Selectable message text with the same inline link ordering used by
/// Flutter's SelectableMessageText: URL, email, then emoji presentation.
struct MessageRichText: View {
    private let presentation: MessageRichTextPresentation
    private let color: Color
    private let lineLimit: Int?
    private let selectable: Bool
    private let onOpenURL: ((URL) -> Void)?

    init(
        content: String,
        baseFontSize: CGFloat = 16,
        color: Color = .primary,
        lineLimit: Int? = nil,
        mentionNames: [String: String] = [:],
        highlight: String = "",
        selectable: Bool = true,
        onOpenURL: ((URL) -> Void)? = nil
    ) {
        presentation = MessageRichTextPresentation(
            content: content,
            baseFontSize: baseFontSize,
            mentionNames: mentionNames,
            highlight: highlight
        )
        self.color = color
        self.lineLimit = lineLimit
        self.selectable = selectable
        self.onOpenURL = onOpenURL
    }

    @ViewBuilder
    var body: some View {
        if selectable {
            text
                .textSelection(.enabled)
        } else {
            text
        }
    }

    private var text: some View {
        Text(presentation.attributedText)
            .font(.system(size: presentation.fontSize))
            .foregroundStyle(color)
            .lineLimit(lineLimit)
            .environment(
                \.openURL,
                OpenURLAction { url in
                    if let onOpenURL {
                        onOpenURL(url)
                        return .handled
                    }
                    return .systemAction(url)
                }
            )
    }
}

struct MessageRichTextPresentation {
    let attributedText: AttributedString
    let fontSize: CGFloat

    init(
        content: String,
        baseFontSize: CGFloat,
        mentionNames: [String: String] = [:],
        highlight: String = ""
    ) {
        let rendered = Self.renderedContent(
            content: content,
            mentionNames: mentionNames
        )
        let fontSize =
            switch Self.emojiOnlyCount(in: rendered.text) {
            case 1:
                max(baseFontSize, 44)
            case 2:
                max(baseFontSize, 38)
            case 3:
                max(baseFontSize, 32)
            default:
                baseFontSize
            }
        var attributedText = AttributedString(rendered.text)
        Self.applyDetectedLinks(
            to: &attributedText,
            content: rendered.text,
            mentionLinks: rendered.mentions
        )
        Self.applyHighlight(
            to: &attributedText,
            content: rendered.text,
            highlight: highlight
        )
        self.fontSize = fontSize
        self.attributedText = attributedText
    }

    private static let mentionExpression = try? NSRegularExpression(
        pattern: #"@(\d{4,})"#
    )
    private static let emailExpression = try? NSRegularExpression(
        pattern: #"\b[\w.%+-]+@[\w.-]+\.[A-Za-z]{2,}\b"#
    )
    private static let identityExpression = try? NSRegularExpression(
        pattern: #"@(\d{4,})|(?<!\d)(7000\d{6})(?!\d)"#
    )
    private static let linkDetector = try? NSDataDetector(
        types: NSTextCheckingResult.CheckingType.link.rawValue
    )

    private static func emojiOnlyCount(in content: String) -> Int? {
        let visible = content.filter { !$0.isWhitespace }
        guard !visible.isEmpty else {
            return nil
        }
        let characters = Array(visible)
        guard characters.allSatisfy(\.isPresentedAsEmoji) else {
            return nil
        }
        return characters.count
    }

    private static func renderedContent(
        content: String,
        mentionNames: [String: String]
    ) -> RenderedMessageText {
        guard !mentionNames.isEmpty,
            let mentionExpression
        else {
            return RenderedMessageText(text: content, mentions: [])
        }
        let source = content as NSString
        let matches = mentionExpression.matches(
            in: content,
            range: NSRange(location: 0, length: source.length)
        )
        guard !matches.isEmpty else {
            return RenderedMessageText(text: content, mentions: [])
        }

        var output = ""
        var mentions: [RenderedMention] = []
        var sourceOffset = 0
        for match in matches {
            let identity = source.substring(with: match.range(at: 1))
            guard let name = mentionNames[identity] else {
                continue
            }
            if match.range.location > sourceOffset {
                output += source.substring(
                    with: NSRange(
                        location: sourceOffset,
                        length: match.range.location - sourceOffset
                    )
                )
            }
            let replacement = "@\(name)"
            let outputRange = NSRange(
                location: (output as NSString).length,
                length: (replacement as NSString).length
            )
            output += replacement
            mentions.append(RenderedMention(identity: identity, range: outputRange))
            sourceOffset = NSMaxRange(match.range)
        }
        if sourceOffset == 0 {
            return RenderedMessageText(text: content, mentions: [])
        }
        if sourceOffset < source.length {
            output += source.substring(from: sourceOffset)
        }
        return RenderedMessageText(text: output, mentions: mentions)
    }

    private static func applyDetectedLinks(
        to value: inout AttributedString,
        content: String,
        mentionLinks: [RenderedMention]
    ) {
        let fullRange = NSRange(location: 0, length: (content as NSString).length)
        var linkedRanges: [NSRange] = []
        for mention in mentionLinks {
            if let url = URL(string: "mixin://users/\(mention.identity)") {
                applyLink(url, range: mention.range, content: content, to: &value)
                linkedRanges.append(mention.range)
            }
        }
        if let linkDetector {
            linkDetector.enumerateMatches(
                in: content,
                options: [],
                range: fullRange
            ) { match, _, _ in
                guard let match,
                    let url = match.url,
                    match.range.length > 0,
                    !linkedRanges.contains(where: {
                        NSIntersectionRange($0, match.range).length > 0
                    })
                else {
                    return
                }
                applyLink(url, range: match.range, content: content, to: &value)
                linkedRanges.append(match.range)
            }
        }

        guard let emailExpression else {
            return
        }
        for match in emailExpression.matches(in: content, range: fullRange)
        where !linkedRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 }) {
            let address = (content as NSString).substring(with: match.range)
            var components = URLComponents()
            components.scheme = "mailto"
            components.path = address
            if let url = components.url {
                applyLink(url, range: match.range, content: content, to: &value)
                linkedRanges.append(match.range)
            }
        }

        guard let identityExpression else {
            return
        }
        for match in identityExpression.matches(in: content, range: fullRange)
        where !linkedRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 }) {
            let raw = (content as NSString).substring(with: match.range)
            let identity = raw.hasPrefix("@") ? String(raw.dropFirst()) : raw
            if let url = URL(string: "mixin://users/\(identity)") {
                applyLink(url, range: match.range, content: content, to: &value)
            }
        }
    }

    private static func applyLink(
        _ url: URL,
        range: NSRange,
        content: String,
        to value: inout AttributedString
    ) {
        guard let range = attributedRange(range, content: content, in: value) else {
            return
        }
        value[range].link = url
        value[range].foregroundColor = .accentColor
        value[range].underlineStyle = Text.LineStyle(
            pattern: .solid,
            color: .accentColor
        )
    }

    private static func applyHighlight(
        to value: inout AttributedString,
        content: String,
        highlight: String
    ) {
        let keyword = highlight.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !keyword.isEmpty else {
            return
        }

        var searchStart = content.startIndex
        while searchStart < content.endIndex,
            let match = content.range(
                of: keyword,
                options: [.caseInsensitive],
                range: searchStart..<content.endIndex
            )
        {
            guard
                let range = attributedRange(
                    NSRange(match, in: content),
                    content: content,
                    in: value
                )
            else {
                searchStart = match.upperBound
                continue
            }
            value[range].backgroundColor = Color.accentColor.opacity(0.24)
            searchStart = match.upperBound
        }
    }

    private static func attributedRange(
        _ range: NSRange,
        content: String,
        in value: AttributedString
    ) -> Range<AttributedString.Index>? {
        guard let stringRange = Range(range, in: content),
            let lowerBound = AttributedString.Index(
                stringRange.lowerBound,
                within: value
            ),
            let upperBound = AttributedString.Index(
                stringRange.upperBound,
                within: value
            )
        else {
            return nil
        }
        return lowerBound..<upperBound
    }
}

private struct RenderedMessageText {
    let text: String
    let mentions: [RenderedMention]
}

private struct RenderedMention {
    let identity: String
    let range: NSRange
}

private extension Character {
    var isPresentedAsEmoji: Bool {
        let scalars = unicodeScalars
        if scalars.contains(where: \.properties.isEmojiPresentation) {
            return true
        }
        let values = scalars.map(\.value)
        let requestsEmojiPresentation = values.contains(0xFE0F)
            || values.contains(0x20E3)
            || values.contains(0x200D)
        return requestsEmojiPresentation
            && scalars.contains(where: \.properties.isEmoji)
    }
}
