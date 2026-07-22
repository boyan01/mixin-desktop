part of 'message_controller.dart';

typedef RecentMessagesQuery =
    Future<List<MessageListEntry>> Function(String conversationId, int limit);
typedef MessageOrderInfoQuery =
    Future<MessageOrderInfo?> Function(String messageId);
typedef AroundMessageIdsQuery =
    Future<List<String>> Function(
      MessageOrderInfo anchor,
      String conversationId,
      int limit,
    );
typedef AroundMessagesQuery =
    Future<List<MessageListEntry>> Function(
      MessageOrderInfo anchor,
      String conversationId,
      int limit,
    );
typedef MessagesByIdsQuery =
    Future<List<MessageListEntry>> Function(List<String> messageIds);
typedef CenteredMessagesQuery =
    Future<List<MessageListEntry>> Function(
      String conversationId,
      String targetMessageId,
      int before,
      int after,
    );

enum MessageWindowDirection { older, newer }

class MessageWindowLoader {
  const MessageWindowLoader({
    required this.recentMessages,
    required this.messageOrderInfo,
    required this.beforeMessages,
    required this.afterMessages,
    required this.beforeMessageIds,
    required this.afterMessageIds,
    required this.messagesByIds,
    required this.centeredMessages,
  });

  factory MessageWindowLoader.fromAccount(
    rust.AccountHandle account,
  ) {
    Future<List<MessageListEntry>> messagesByIds(List<String> ids) async {
      if (ids.isEmpty) return const [];
      final messages = await account.message().messageItemsByIds(
        messageIds: ids,
      );
      final byId = {
        for (final message in messages)
          message.messageId: MessageListEntry.fromRust(message),
      };
      return ids.map((id) => byId[id]).nonNulls.toList(growable: false);
    }

    Future<List<String>> messageIdsBefore(
      MessageOrderInfo anchor,
      String conversationId,
      int limit,
    ) => account.message().messageIdsBefore(
      conversationId: conversationId,
      anchorRowId: anchor.rowId,
      anchorCreatedAtMicros: anchor.createdAt.microsecondsSinceEpoch,
      limit: limit,
    );

    Future<List<String>> messageIdsAfter(
      MessageOrderInfo anchor,
      String conversationId,
      int limit,
    ) => account.message().messageIdsAfter(
      conversationId: conversationId,
      anchorRowId: anchor.rowId,
      anchorCreatedAtMicros: anchor.createdAt.microsecondsSinceEpoch,
      limit: limit,
    );

    return MessageWindowLoader(
      recentMessages: (conversationId, limit) async =>
          (await account.message().messages(
            conversationId: conversationId,
            limit: limit,
          )).map(MessageListEntry.fromRust).toList(growable: false),
      messageOrderInfo: (messageId) async {
        final info = await account.message().messageOrderInfo(
          messageId: messageId,
        );
        return info == null
            ? null
            : MessageOrderInfo(
                messageId: info.messageId,
                rowId: info.rowId,
                createdAt: DateTime.fromMicrosecondsSinceEpoch(
                  info.createdAtMicros,
                ),
              );
      },
      beforeMessages: (anchor, conversationId, limit) async {
        final ids = await messageIdsBefore(anchor, conversationId, limit);
        return messagesByIds(ids);
      },
      afterMessages: (anchor, conversationId, limit) async {
        final ids = await messageIdsAfter(anchor, conversationId, limit);
        return messagesByIds(ids);
      },
      beforeMessageIds: messageIdsBefore,
      afterMessageIds: messageIdsAfter,
      messagesByIds: messagesByIds,
      centeredMessages:
          (conversationId, targetMessageId, before, after) async =>
              (await account.message().messagesAround(
                conversationId: conversationId,
                targetMessageId: targetMessageId,
                before: before,
                after: after,
              )).map(MessageListEntry.fromRust).toList(growable: false),
    );
  }

  final RecentMessagesQuery recentMessages;
  final MessageOrderInfoQuery messageOrderInfo;
  final AroundMessagesQuery beforeMessages;
  final AroundMessagesQuery afterMessages;
  final AroundMessageIdsQuery beforeMessageIds;
  final AroundMessageIdsQuery afterMessageIds;
  final MessagesByIdsQuery messagesByIds;
  final CenteredMessagesQuery centeredMessages;

  Future<MessageState> loadBefore(
    MessageState state,
    String conversationId,
    int limit,
  ) async {
    final topMessageId = state.topMessage?.id;
    if (topMessageId == null) return state.copyWith(isOldest: true);

    final info = await messageOrderInfo(topMessageId);
    if (info == null) return state.copyWith(isOldest: true);

    final messages = await beforeMessages(info, conversationId, limit);
    return state.copyWith(
      top: [...messages.reversed, ...state.top],
      isOldest: messages.length < limit,
    );
  }

  Future<MessageState> loadAfter(
    MessageState state,
    String conversationId,
    int limit,
  ) async {
    final bottomMessageId = state.bottomMessage?.id;
    if (bottomMessageId == null) return state.copyWith(isLatest: true);

    final info = await messageOrderInfo(bottomMessageId);
    if (info == null) return state.copyWith(isLatest: true);

    final messages = await afterMessages(info, conversationId, limit);
    return state.copyWith(
      bottom: [...state.bottom, ...messages],
      isLatest: messages.length < limit ? true : null,
    );
  }

  Future<MessageWindowDirection?> directionFromSource({
    required String? sourceMessageId,
    required String targetMessageId,
  }) async {
    if (sourceMessageId == null || sourceMessageId == targetMessageId) {
      return null;
    }

    final results = await Future.wait([
      messageOrderInfo(sourceMessageId),
      messageOrderInfo(targetMessageId),
    ]);
    final sourceInfo = results[0];
    final targetInfo = results[1];
    if (sourceInfo == null || targetInfo == null) return null;

    final sourceAfterTarget = sourceInfo.createdAt == targetInfo.createdAt
        ? sourceInfo.rowId > targetInfo.rowId
        : sourceInfo.createdAt.isAfter(targetInfo.createdAt);
    return sourceAfterTarget
        ? MessageWindowDirection.older
        : MessageWindowDirection.newer;
  }

  Future<MessageState> load(
    String conversationId,
    int limit, {
    String? centerMessageId,
    MessageWindowAnchor? anchor,
    void Function(String message)? trace,
  }) async {
    final resolvedAnchor =
        anchor ??
        (centerMessageId == null
            ? const LatestMessageWindowAnchor()
            : AroundMessageWindowAnchor(
                messageId: centerMessageId,
                source: MessageWindowJumpSource.conversation,
              ));
    final resolvedCenterMessageId = resolvedAnchor.centerMessageId;

    Future<MessageState> recent() async {
      final list = await recentMessages(conversationId, limit);

      trace?.call('query recent count=${list.length} limit=$limit');
      return MessageState(
        top: list.reversed.toList(),
        isLatest: true,
        isOldest: list.length < limit,
      );
    }

    if (resolvedCenterMessageId == null) return recent();

    final halfLimit = limit ~/ 2;
    final messages = await centeredMessages(
      conversationId,
      resolvedCenterMessageId,
      halfLimit,
      halfLimit,
    );
    final centerIndex = messages.indexWhere(
      (message) => message.id == resolvedCenterMessageId,
    );
    if (centerIndex < 0) {
      trace?.call(
        'query center missing '
        'target=${shortMessageId(resolvedCenterMessageId)}',
      );
      return recent();
    }

    var topList = messages.take(centerIndex).toList(growable: false);
    MessageListEntry? center = messages[centerIndex];
    final bottomList = messages.skip(centerIndex + 1).toList(growable: false);

    final isLatest = bottomList.length < halfLimit;
    final isOldest = topList.length < halfLimit;

    if (bottomList.isEmpty) {
      topList = [...topList, center];
      center = null;
    }

    trace?.call(
      'query centered target=${shortMessageId(resolvedCenterMessageId)} '
      'top=${topList.length} center=${center != null} '
      'bottom=${bottomList.length} isLatest=$isLatest isOldest=$isOldest',
    );
    return MessageState(
      top: topList,
      center: center,
      bottom: bottomList,
      isLatest: isLatest,
      isOldest: isOldest,
      anchor: resolvedAnchor,
    );
  }
}
