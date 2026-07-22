import 'dart:convert';

import '../models/message_list_entry.dart';

class MessageActionPolicy {
  const MessageActionPolicy({
    required this.message,
    required this.currentUserId,
    required this.currentUserRole,
    required this.now,
    this.isTranscriptPage = false,
    this.isPinnedPage = false,
  });

  static const _completedStatuses = {'SENT', 'DELIVERED', 'READ'};

  final MessageListEntry message;
  final String currentUserId;
  final String? currentUserRole;
  final DateTime now;
  final bool isTranscriptPage;
  final bool isPinnedPage;

  bool get canReply =>
      !isTranscriptPage && !isPinnedPage && message.category.canReply;

  bool get canForward => !isTranscriptPage && _messageCanForward(message);

  bool get canCombineForward =>
      !isTranscriptPage && _messageCanCombineForward(message);

  bool get canSelect => !isTranscriptPage;

  bool get canPin =>
      !isTranscriptPage &&
      message.category.canReply &&
      _isCompleted &&
      currentUserRole != null;

  bool get canSave =>
      message.mediaStatus.toUpperCase() == 'DONE' &&
      message.mediaUrl?.isNotEmpty == true &&
      (message.category.isData ||
          message.category.isImage ||
          message.category.isVideo ||
          message.category.isAudio);

  bool get canRecall =>
      !isTranscriptPage &&
      _isCompleted &&
      message.senderId == currentUserId &&
      message.category.canRecall &&
      now.isBefore(message.createdAt.add(const Duration(minutes: 60)));

  bool get canDelete => !isTranscriptPage && !isPinnedPage;

  bool get canAddSticker => message.category.isSticker;

  bool get canAddImageAsSticker =>
      message.category.isImage &&
      const {'DONE', 'READ'}.contains(message.mediaStatus.toUpperCase()) &&
      message.mediaUrl?.isNotEmpty == true;

  bool get _isCompleted =>
      _completedStatuses.contains(message.status.toUpperCase());

  static bool _messageCanForward(MessageListEntry message) {
    if (!_completedStatuses.contains(message.status.toUpperCase())) {
      return false;
    }

    final category = message.category;
    if (category == 'APP_CARD') {
      return _jsonShareable(message.content, fallback: false);
    }

    final isFinishedAttachment =
        (category.isImage ||
            category.isVideo ||
            category.isAudio ||
            category.isData) &&
        const {'DONE', 'READ'}.contains(message.mediaStatus.toUpperCase()) &&
        message.mediaUrl?.isNotEmpty == true &&
        _jsonShareable(message.content, fallback: true);

    return category.isText ||
        isFinishedAttachment ||
        category.isSticker ||
        category.isContact ||
        (category.isLive && _jsonShareable(message.content, fallback: false)) ||
        category.isPost ||
        category.isLocation ||
        (category.isTranscript && (message.mediaSize ?? 0) <= 0);
  }

  static bool _messageCanCombineForward(MessageListEntry message) {
    if (!_completedStatuses.contains(message.status.toUpperCase()) ||
        message.category.isTranscript) {
      return false;
    }
    if (message.category == 'APP_CARD') {
      return _jsonShareable(message.content, fallback: false);
    }
    final category = message.category;
    final isAttachment =
        category.isImage ||
        category.isVideo ||
        category.isAudio ||
        category.isData;
    if (isAttachment) {
      final downloaded = const {
        'DONE',
        'READ',
      }.contains(message.mediaStatus.toUpperCase());
      if (!downloaded) return false;
      return !category.isAudio ||
          _jsonShareable(message.content, fallback: true);
    }
    return category.isText ||
        category.isSticker ||
        category.isContact ||
        category.isLive ||
        category.isPost ||
        category.isLocation;
  }

  static bool _jsonShareable(String content, {required bool fallback}) {
    try {
      final value = jsonDecode(content);
      if (value is Map<String, dynamic>) {
        final shareable = value['shareable'];
        return shareable is bool ? shareable : fallback;
      }
    } on FormatException {
      // Keep parity with flutter-app: invalid attachment metadata is handled
      // by the category-specific fallback.
    }
    return fallback;
  }
}

extension MessageActionCategory on String {
  bool get isText => endsWith('_TEXT');
  bool get isImage => endsWith('_IMAGE');
  bool get isVideo => endsWith('_VIDEO');
  bool get isAudio => endsWith('_AUDIO');
  bool get isSticker => endsWith('_STICKER');
  bool get isData => endsWith('_DATA');
  bool get isPost => endsWith('_POST');
  bool get isLive => endsWith('_LIVE');
  bool get isLocation => endsWith('_LOCATION');
  bool get isContact => endsWith('_CONTACT');
  bool get isTranscript => endsWith('_TRANSCRIPT');

  bool get canReply =>
      isText ||
      isImage ||
      isVideo ||
      isLive ||
      isData ||
      isPost ||
      isLocation ||
      isAudio ||
      isSticker ||
      isContact ||
      isTranscript ||
      this == 'APP_CARD' ||
      this == 'APP_BUTTON_GROUP';

  bool get canRecall =>
      canReply && this != 'APP_BUTTON_GROUP' && !startsWith('SYSTEM_');
}
