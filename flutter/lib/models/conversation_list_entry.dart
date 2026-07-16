class ConversationListEntry {
  const ConversationListEntry({
    required this.id,
    required this.ownerId,
    required this.name,
    required this.avatarUrl,
    required this.category,
    required this.draft,
    required this.status,
    this.lastReadMessageId,
    required this.content,
    required this.contentType,
    required this.messageStatus,
    required this.senderId,
    required this.senderName,
    required this.updatedAt,
    required this.unseenCount,
    required this.mentionCount,
    required this.isMuted,
    required this.isVerified,
    required this.isBot,
    required this.isPinned,
    required this.relationship,
    required this.identityNumber,
    required this.circleIds,
    required this.groupAvatars,
    this.participantCount = 0,
  });

  final String id;
  final String ownerId;
  final String name;
  final String avatarUrl;
  final String category;
  final String draft;
  final int status;
  final String? lastReadMessageId;
  final String content;
  final String? contentType;
  final String? messageStatus;
  final String? senderId;
  final String? senderName;
  final DateTime updatedAt;
  final int unseenCount;
  final int mentionCount;
  final bool isMuted;
  final bool isVerified;
  final bool isBot;
  final bool isPinned;
  final String relationship;
  final String identityNumber;
  final List<String> circleIds;
  final List<ConversationAvatarEntry> groupAvatars;
  final int participantCount;

  bool get isGroup => category == 'GROUP';
}

class ConversationAvatarEntry {
  const ConversationAvatarEntry({
    required this.userId,
    required this.name,
    required this.avatarUrl,
  });

  final String userId;
  final String name;
  final String avatarUrl;
}

enum ConversationCategoryFilter {
  chats,
  contacts,
  groups,
  bots,
  strangers,
  circle,
}
