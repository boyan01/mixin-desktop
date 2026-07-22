import '../src/rust/desktop_api.dart' show UserProfileItem;
import 'conversation_list_entry.dart';

class CommandPaletteItem {
  const CommandPaletteItem({
    required this.id,
    required this.name,
    required this.avatarUrl,
    required this.type,
    required this.isVerified,
    required this.isBot,
    required this.matchScore,
    this.conversation,
  });

  factory CommandPaletteItem.conversation(
    ConversationListEntry conversation,
    String keyword,
  ) => CommandPaletteItem(
    id: conversation.id,
    name: conversation.name,
    avatarUrl: conversation.avatarUrl,
    type: conversation.category,
    isVerified: conversation.isVerified,
    isBot: conversation.isBot,
    matchScore: _matchScore(
      keyword,
      conversation.name,
      conversation.identityNumber,
    ),
    conversation: conversation,
  );

  factory CommandPaletteItem.user(UserProfileItem user, String keyword) =>
      CommandPaletteItem(
        id: user.userId,
        name: user.fullName,
        avatarUrl: user.avatarUrl,
        type: user.isBot ? 'BOT' : 'USER',
        isVerified: user.isVerified,
        isBot: user.isBot,
        matchScore: _matchScore(keyword, user.fullName, user.identityNumber),
      );

  final String id;
  final String name;
  final String avatarUrl;
  final String type;
  final bool isVerified;
  final bool isBot;
  final int matchScore;
  final ConversationListEntry? conversation;
}

int _matchScore(String keyword, String name, String identityNumber) {
  final query = keyword.trim().toLowerCase();
  if (query.isEmpty) return 0;
  final normalizedName = name.toLowerCase();
  final normalizedIdentity = identityNumber.toLowerCase();
  if (normalizedName == query || normalizedIdentity == query) return 100;
  if (normalizedName.startsWith(query) ||
      normalizedIdentity.startsWith(query)) {
    return 80;
  }
  if (normalizedName.contains(query) || normalizedIdentity.contains(query)) {
    return 60;
  }
  return 0;
}
