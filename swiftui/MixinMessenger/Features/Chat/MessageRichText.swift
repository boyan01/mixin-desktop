import AppKit
import SwiftUI

/// Selectable message text with the same inline link ordering used by
/// Flutter's SelectableMessageText: URL, email, then emoji presentation.
struct MessageRichText: View {
    let content: String
    var baseFontSize: CGFloat = 16
    var color: Color = .primary
    var lineLimit: Int?
    var mentionNames: [String: String] = [:]
    var highlight: String = ""
    var onOpenURL: ((URL) -> Void)?

    var body: some View {
        let presentation = MessageRichTextPresentation(
            content: content,
            baseFontSize: baseFontSize,
            mentionNames: mentionNames,
            highlight: highlight
        )
        Text(presentation.attributedText)
            .font(.system(size: presentation.fontSize))
            .foregroundStyle(color)
            .lineLimit(lineLimit)
            .textSelection(.enabled)
            .environment(
                \.openURL,
                OpenURLAction { url in
                    if let onOpenURL {
                        onOpenURL(url)
                    } else {
                        NSWorkspace.shared.open(url)
                    }
                    return .handled
                }
            )
    }
}

struct MessageRichTextPresentation {
    let content: String
    let baseFontSize: CGFloat
    var mentionNames: [String: String] = [:]
    var highlight = ""

    var fontSize: CGFloat {
        switch emojiOnlyCount {
        case 1:
            max(baseFontSize, 44)
        case 2:
            max(baseFontSize, 38)
        case 3:
            max(baseFontSize, 32)
        default:
            baseFontSize
        }
    }

    var attributedText: AttributedString {
        let rendered = renderedContent
        let value = NSMutableAttributedString(
            string: rendered.text,
            attributes: [
                .font: NSFont.systemFont(ofSize: fontSize),
            ]
        )
        applyDetectedLinks(
            to: value,
            content: rendered.text,
            mentionLinks: rendered.mentions
        )
        applyHighlight(to: value, content: rendered.text)
        applyEmojiFont(to: value, content: rendered.text)
        return AttributedString(value)
    }

    var emojiOnlyCount: Int? {
        let visible = renderedContent.text.filter { !$0.isWhitespace }
        guard !visible.isEmpty else {
            return nil
        }
        let characters = Array(visible)
        guard characters.allSatisfy(\.isPresentedAsEmoji) else {
            return nil
        }
        return characters.count
    }

    private var renderedContent: RenderedMessageText {
        guard !mentionNames.isEmpty,
              let expression = try? NSRegularExpression(pattern: #"@(\d{4,})"#)
        else {
            return RenderedMessageText(text: content, mentions: [])
        }
        let source = content as NSString
        let matches = expression.matches(
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

    private func applyDetectedLinks(
        to value: NSMutableAttributedString,
        content: String,
        mentionLinks: [RenderedMention]
    ) {
        let fullRange = NSRange(location: 0, length: value.length)
        var linkedRanges: [NSRange] = []
        for mention in mentionLinks {
            if let url = URL(string: "mixin://users/\(mention.identity)") {
                applyLink(url, range: mention.range, to: value)
                linkedRanges.append(mention.range)
            }
        }
        if let detector = try? NSDataDetector(
            types: NSTextCheckingResult.CheckingType.link.rawValue
        ) {
            detector.enumerateMatches(
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
                applyLink(url, range: match.range, to: value)
                linkedRanges.append(match.range)
            }
        }

        guard let mail = try? NSRegularExpression(
            pattern: #"\b[\w.%+-]+@[\w.-]+\.[A-Za-z]{2,}\b"#
        ) else {
            return
        }
        for match in mail.matches(in: content, range: fullRange)
        where !linkedRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 }) {
            let address = (content as NSString).substring(with: match.range)
            var components = URLComponents()
            components.scheme = "mailto"
            components.path = address
            if let url = components.url {
                applyLink(url, range: match.range, to: value)
                linkedRanges.append(match.range)
            }
        }

        guard let identities = try? NSRegularExpression(
            pattern: #"@(\d{4,})|(?<!\d)(7000\d{6})(?!\d)"#
        ) else {
            return
        }
        for match in identities.matches(in: content, range: fullRange)
        where !linkedRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 }) {
            let raw = (content as NSString).substring(with: match.range)
            let identity = raw.hasPrefix("@") ? String(raw.dropFirst()) : raw
            if let url = URL(string: "mixin://users/\(identity)") {
                applyLink(url, range: match.range, to: value)
            }
        }
    }

    private func applyLink(
        _ url: URL,
        range: NSRange,
        to value: NSMutableAttributedString
    ) {
        value.addAttributes(
            [
                .link: url,
                .foregroundColor: NSColor.controlAccentColor,
                .underlineStyle: NSUnderlineStyle.single.rawValue,
            ],
            range: range
        )
    }

    private func applyHighlight(
        to value: NSMutableAttributedString,
        content: String
    ) {
        let keyword = highlight.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !keyword.isEmpty,
              let expression = try? NSRegularExpression(
                  pattern: NSRegularExpression.escapedPattern(for: keyword),
                  options: [.caseInsensitive]
              )
        else {
            return
        }
        let range = NSRange(location: 0, length: (content as NSString).length)
        for match in expression.matches(in: content, range: range) {
            value.addAttribute(
                .backgroundColor,
                value: NSColor.selectedTextBackgroundColor,
                range: match.range
            )
        }
    }

    private func applyEmojiFont(
        to value: NSMutableAttributedString,
        content: String
    ) {
        var utf16Offset = 0
        for character in content {
            let length = String(character).utf16.count
            if character.isPresentedAsEmoji {
                value.addAttribute(
                    .font,
                    value: NSFont(name: "Apple Color Emoji", size: fontSize)
                        ?? NSFont.systemFont(ofSize: fontSize),
                    range: NSRange(location: utf16Offset, length: length)
                )
            }
            utf16Offset += length
        }
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
