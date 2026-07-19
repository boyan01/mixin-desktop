import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixin_desktop_ui/controllers/message_controller.dart';
import 'package:mixin_desktop_ui/utils/chat_jump_trace.dart';
import 'package:mixin_desktop_ui/widgets/clamping_custom_scroll_view/clamping_custom_scroll_view.dart';
import 'package:mixin_desktop_ui/widgets/message_day_time.dart';
import 'package:mixin_desktop_ui/widgets/message_rows.dart';

import 'chat_scroll_coordinator.dart';
import 'chat_timeline_window.dart';

typedef ChatMessageBuilder =
    Widget Function(MessageRowModel row, GlobalKey? dayTimeKey);

@visibleForTesting
void syncMessageGlobalKeys(
  Map<String, GlobalKey> keysByMessageId,
  Set<String> messageIds, {
  GlobalKey Function(String messageId) createKey = _messageGlobalKey,
}) {
  keysByMessageId.removeWhere(
    (messageId, _) => !messageIds.contains(messageId),
  );
  for (final messageId in messageIds) {
    keysByMessageId.putIfAbsent(messageId, () => createKey(messageId));
  }
}

GlobalKey _messageGlobalKey(String messageId) => MessageGlobalKey(messageId);

GlobalKey _messageDayTimeKey(String messageId) =>
    GlobalKey(debugLabel: 'message day time $messageId');

class ChatHistoryViewport extends HookWidget {
  const ChatHistoryViewport({
    required this.messageController,
    required this.presentationListenable,
    required this.scrollCoordinator,
    required this.messageBuilder,
    required this.unreadBar,
    super.key,
  });

  final MessageController messageController;
  final Listenable presentationListenable;
  final ChatScrollCoordinator scrollCoordinator;
  final ChatMessageBuilder messageBuilder;
  final Widget unreadBar;

  @override
  Widget build(BuildContext context) {
    final state = useValueListenable(messageController);
    useListenable(presentationListenable);
    final timelineWindow = ChatTimelineWindow(state);

    final centerKey = ValueKey((state.conversationId, state.refreshKey));
    final messages = timelineWindow.messages;
    final rows = timelineWindow.rows;
    final messageKeysRef = useRef<Map<String, GlobalKey>>({});
    final dayTimeKeysRef = useRef<Map<String, GlobalKey>>({});
    final previousConversationIdRef = useRef<String?>(null);
    final previousRefreshKeyRef = useRef<Object?>(null);
    final viewportKey = useMemoized(
      () => GlobalKey(debugLabel: 'chat scroll viewport'),
      [scrollCoordinator],
    );

    final messageIds = messages.map((message) => message.id).toSet();
    final messageIdsKey = messages.map((message) => message.id).join('|');
    final resetScrollWindow = timelineWindow.resetScrollWindow(
      previousConversationId: previousConversationIdRef.value,
      previousRefreshKey: previousRefreshKeyRef.value,
    );

    useEffect(() {
      scrollCoordinator.viewportKey = viewportKey;
      return () => scrollCoordinator.detachViewportKey(viewportKey);
    }, [scrollCoordinator, viewportKey]);

    if (!resetScrollWindow) {
      scrollCoordinator.captureViewportState(messages, messageKeysRef.value);
    }

    useMemoized(() {
      syncMessageGlobalKeys(messageKeysRef.value, messageIds);
      syncMessageGlobalKeys(
        dayTimeKeysRef.value,
        messageIds,
        createKey: _messageDayTimeKey,
      );
    }, [messageIdsKey]);

    final dayTimeEntries = useMemoized(
      () => _dayTimeEntries(rows, messageKeysRef.value, dayTimeKeysRef.value),
      [messageIdsKey],
    );

    Widget buildMessage(MessageRowModel row) {
      final messageId = row.message.id;
      return ChatRenderedMessage(
        coordinator: scrollCoordinator,
        messageId: messageId,
        child: KeyedSubtree(
          key: messageKeysRef.value[messageId],
          child: messageBuilder(
            row,
            row.dateTime == null ? null : dayTimeKeysRef.value[messageId],
          ),
        ),
      );
    }

    useEffect(() {
      traceChatJump(
        'viewport restore-input '
        'conv=${shortMessageId(state.conversationId)} '
        'reset=$resetScrollWindow '
        'unreadAnchor=${timelineWindow.anchorUnreadSeparator} '
        'anchor=${formatDouble(timelineWindow.scrollAnchor)} '
        'lastRead=${shortMessageId(state.lastReadMessageId)} '
        'center=${shortMessageId(state.center?.id)} '
        'top=${state.top.length} bottom=${state.bottom.length} '
        'messages=${messages.length} latest=${state.isLatest}',
      );
      scrollCoordinator.scheduleRestore(
        messages: messages,
        keysByMessageId: messageKeysRef.value,
        reset: resetScrollWindow,
        isLatest: state.isLatest,
        hasCenteredAnchor: timelineWindow.anchorUnreadSeparator,
        animateLatestReset: timelineWindow.animateLatestReset(
          previousConversationId: previousConversationIdRef.value,
          previousRefreshKey: previousRefreshKeyRef.value,
        ),
        centerMessageId: timelineWindow.restoreCenterMessageId,
        traceTargetMessageId: timelineWindow.traceTargetMessageId,
      );
      previousConversationIdRef.value = state.conversationId;
      previousRefreshKeyRef.value = state.refreshKey;
      return null;
    }, [state.conversationId, state.refreshKey, messageIdsKey, state.isLatest]);

    return MessageDayTimeViewportWidget(
      key: centerKey,
      entries: dayTimeEntries,
      reTraversalKey: messageIdsKey,
      child: NotificationListener<ScrollNotification>(
        onNotification: (notification) =>
            scrollCoordinator.handleScrollNotification(
              notification,
              messages: messages,
              keysByMessageId: messageKeysRef.value,
              loadBefore: messageController.before,
              loadAfter: messageController.after,
            ),
        child: ClampingCustomScrollView(
          key: viewportKey,
          center: centerKey,
          controller: scrollCoordinator.scrollController,
          anchor: timelineWindow.scrollAnchor,
          physics: const ClampingScrollPhysics(),
          scrollCacheExtent: const ScrollCacheExtent.viewport(
            ChatScrollCoordinator.loadedJumpViewportCount,
          ),
          suppressAutoBottomTracking:
              scrollCoordinator.shouldSuppressAutoBottomTracking,
          slivers: [
            SliverList(
              delegate: SliverChildBuilderDelegate((context, index) {
                final actualIndex = rows.top.length - index - 1;
                return buildMessage(rows.top[actualIndex]);
              }, childCount: rows.top.length),
            ),
            SliverToBoxAdapter(
              key: centerKey,
              child: Builder(
                builder: (context) {
                  if (timelineWindow.anchorUnreadSeparator) return unreadBar;
                  final row = rows.center;
                  return row == null ? const SizedBox() : buildMessage(row);
                },
              ),
            ),
            SliverList(
              delegate: SliverChildBuilderDelegate(
                (context, index) => buildMessage(rows.bottom[index]),
                childCount: rows.bottom.length,
              ),
            ),
            const SliverToBoxAdapter(child: SizedBox(height: 10)),
          ],
        ),
      ),
    );
  }
}

List<MessageDayTimeViewportEntry> _dayTimeEntries(
  MessageRows rows,
  Map<String, GlobalKey> messageKeys,
  Map<String, GlobalKey> dayTimeKeys,
) {
  Iterable<MessageDayTimeViewportEntry> mapRows(
    Iterable<MessageRowModel> rows,
  ) => rows.map((row) {
    final message = row.message;
    return MessageDayTimeViewportEntry(
      dateTime: message.createdAt,
      messageKey: messageKeys[message.id]!,
      dayTimeKey: row.dateTime == null ? null : dayTimeKeys[message.id],
    );
  });

  return [
    ...mapRows(rows.top),
    if (rows.center != null) ...mapRows([rows.center!]),
    ...mapRows(rows.bottom),
  ];
}
