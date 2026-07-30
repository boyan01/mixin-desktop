import AppKit
import SwiftUI

struct MentionCandidatePanel: View {
  @Environment(\.mixinTheme) private var theme

  let users: [ConversationParticipantItem]
  let keyword: String
  let selectedIndex: Int
  let onSelect: (Int?) -> Void

  var body: some View {
    ScrollViewReader { proxy in
      AppScrollView {
        LazyVStack(spacing: 0) {
          ForEach(Array(users.enumerated()), id: \.element.userId) {
            index,
            user in
            Button {
              onSelect(index)
            } label: {
              HStack(spacing: 6) {
                MentionCandidateAvatar(user: user)
                VStack(alignment: .leading, spacing: 2) {
                  MentionHighlightedText(
                    value: user.fullName,
                    keyword: keyword,
                    font: .systemFont(ofSize: 14)
                  )
                  MentionHighlightedText(
                    value: user.identityNumber,
                    keyword: keyword,
                    font: .systemFont(ofSize: 12),
                    secondary: true
                  )
                }
                Spacer()
              }
              .padding(8)
              .frame(height: 50)
              .contentShape(Rectangle())
              .background(
                index == selectedIndex
                  ? theme.listSelected
                  : Color.clear
              )
            }
            .buttonStyle(.plain)
            .id(index)
          }
        }
      }
      .onChange(of: selectedIndex) {
        withAnimation(.easeIn(duration: 0.15)) {
          proxy.scrollTo(selectedIndex, anchor: .center)
        }
      }
    }
    .background(theme.popUp)
    .clipped()
    .accessibilityLabel("Mention suggestions")
  }
}

private struct MentionCandidateAvatar: View {
  let user: ConversationParticipantItem

  var body: some View {
    MixinRemoteImage(url: URL(string: user.avatarUrl)) { image in
      image.resizable().scaledToFill()
    } placeholder: {
      ZStack {
        Circle().fill(Color.accentColor.opacity(0.16))
        Text(user.fullName.first.map(String.init) ?? "?")
          .font(.caption.weight(.semibold))
      }
    }
    .frame(width: 32, height: 32)
    .clipShape(Circle())
  }
}

private struct MentionHighlightedText: View {
  @Environment(\.mixinTheme) private var theme

  let value: String
  let keyword: String
  let font: NSFont
  var secondary = false

  var body: some View {
    Text(attributedValue)
      .lineLimit(1)
  }

  private var attributedValue: AttributedString {
    let result = NSMutableAttributedString(
      string: value,
      attributes: [
        .font: font,
        .foregroundColor: secondary
          ? NSColor(theme.secondaryText)
          : NSColor(theme.text),
      ]
    )
    let highlightedKeyword = keyword.trimmingCharacters(
      in: .whitespacesAndNewlines
    )
    if !highlightedKeyword.isEmpty {
      let range = (value as NSString).range(
        of: highlightedKeyword,
        options: [.caseInsensitive, .diacriticInsensitive]
      )
      if range.location != NSNotFound {
        result.addAttribute(
          .foregroundColor,
          value: NSColor(theme.accent),
          range: range
        )
      }
    }
    return AttributedString(result)
  }
}
