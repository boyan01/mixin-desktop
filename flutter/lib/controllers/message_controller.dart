import 'dart:async';

import 'package:equatable/equatable.dart';
import 'package:flutter/widgets.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/utils/chat_jump_trace.dart';

part 'message_window_loader.dart';

enum MessageWindowJumpSource {
  conversation,
  message,
  quote,
  pin,
  mention,
  search,
  restore,
}

sealed class MessageWindowAnchor extends Equatable {
  const MessageWindowAnchor();

  String? get centerMessageId => null;

  String get debugName;
}

final class LatestMessageWindowAnchor extends MessageWindowAnchor {
  const LatestMessageWindowAnchor();

  @override
  String get debugName => 'latest';

  @override
  List<Object?> get props => const [];
}

final class UnreadMessageWindowAnchor extends MessageWindowAnchor {
  const UnreadMessageWindowAnchor({required this.lastReadMessageId});

  final String lastReadMessageId;

  @override
  String get centerMessageId => lastReadMessageId;

  @override
  String get debugName => 'unread';

  @override
  List<Object?> get props => [lastReadMessageId];
}

final class AroundMessageWindowAnchor extends MessageWindowAnchor {
  const AroundMessageWindowAnchor({
    required this.messageId,
    required this.source,
  });

  final String messageId;
  final MessageWindowJumpSource source;

  @override
  String get centerMessageId => messageId;

  @override
  String get debugName => 'message:$source';

  @override
  List<Object?> get props => [messageId, source];
}

final class RestoreMessageWindowAnchor extends MessageWindowAnchor {
  const RestoreMessageWindowAnchor({
    required this.messageId,
    required this.offset,
  });

  final String messageId;
  final double offset;

  @override
  String get centerMessageId => messageId;

  @override
  String get debugName => 'restore';

  @override
  List<Object?> get props => [messageId, offset];
}

@visibleForTesting
String? resolveMessageWindowLastReadMessageId({
  required String? requestedLastReadMessageId,
  required int? unseenMessageCount,
  required String? conversationLastReadMessageId,
}) {
  if (requestedLastReadMessageId != null) return requestedLastReadMessageId;
  return (unseenMessageCount ?? 0) > 0 ? conversationLastReadMessageId : null;
}

class MessageOrderInfo {
  const MessageOrderInfo({
    required this.messageId,
    required this.rowId,
    required this.createdAt,
  });

  final String messageId;
  final int rowId;
  final DateTime createdAt;
}

class MessageState extends Equatable {
  MessageState({
    this.top = const [],
    this.center,
    this.bottom = const [],
    this.conversationId,
    this.isLatest = false,
    this.isOldest = false,
    this.lastReadMessageId,
    this.refreshKey,
    this.anchor = const LatestMessageWindowAnchor(),
  }) {
    // check top, center, bottom didn't has same messageId
    assert(() {
      final ids = <String>{};
      for (final item in list) {
        if (ids.contains(item.id)) {
          e('MessageState has same messageId: ${item.id}');
        }
        ids.add(item.id);
      }
      return true;
    }());
  }

  final String? conversationId;
  final List<MessageListEntry> top;
  final MessageListEntry? center;
  final List<MessageListEntry> bottom;
  final bool isLatest;
  final bool isOldest;
  final String? lastReadMessageId;
  final Object? refreshKey;
  final MessageWindowAnchor anchor;

  @override
  List<Object?> get props => [
    conversationId,
    top,
    center,
    bottom,
    isLatest,
    isOldest,
    lastReadMessageId,
    refreshKey,
    anchor,
  ];

  MessageListEntry? get bottomMessage =>
      bottom.lastOrNull ?? center ?? top.lastOrNull;

  MessageListEntry? get topMessage =>
      top.firstOrNull ?? center ?? bottom.firstOrNull;

  bool get isEmpty => top.isEmpty && center == null && bottom.isEmpty;

  List<MessageListEntry> get list => [...top, ?center, ...bottom];

  MessageState copyWith({
    String? conversationId,
    List<MessageListEntry>? top,
    MessageListEntry? center,
    List<MessageListEntry>? bottom,
    bool? isLatest,
    bool? isOldest,
    String? lastReadMessageId,
    Object? refreshKey,
    MessageWindowAnchor? anchor,
  }) => MessageState(
    conversationId: conversationId ?? this.conversationId,
    top: top ?? this.top,
    center: center ?? this.center,
    bottom: bottom ?? this.bottom,
    isLatest: isLatest ?? this.isLatest,
    isOldest: isOldest ?? this.isOldest,
    lastReadMessageId: lastReadMessageId ?? this.lastReadMessageId,
    refreshKey: refreshKey ?? this.refreshKey,
    anchor: anchor ?? this.anchor,
  );

  MessageState removeMessage(String messageId) {
    if (center?.id == messageId) {
      return MessageState(
        conversationId: conversationId,
        top: top,
        bottom: bottom,
        isLatest: isLatest,
        isOldest: isOldest,
        lastReadMessageId: lastReadMessageId,
        refreshKey: refreshKey,
        anchor: anchor,
      );
    }

    bool include(MessageListEntry message) => message.id == messageId;
    bool exclusive(MessageListEntry message) => message.id != messageId;

    if (top.any(include)) {
      return copyWith(top: top.where(exclusive).toList());
    }

    if (bottom.any(include)) {
      return copyWith(bottom: bottom.where(exclusive).toList());
    }

    return this;
  }
}

class MessageController extends ValueNotifier<MessageState>
    with WidgetsBindingObserver {
  MessageController({
    required this.account,
    required this.conversation,
    required this.limit,
    this.initialMessageId,
  }) : super(MessageState()) {
    WidgetsBinding.instance.addObserver(this);
    _init(
      centerMessageId: initialMessageId,
      lastReadMessageId: conversation.lastReadMessageId,
    );

    _conversationChanges = account.conversationChanges().listen(
      (event) {
        if (event.reloadAll ||
            event.conversationIds.contains(conversation.id)) {
          _reloadChangedMessages(includeRecent: true);
        }
      },
      onError: (Object exception, StackTrace stackTrace) {
        e(
          'Message change stream failed: conversation_id=${conversation.id}',
          exception,
          stackTrace,
        );
      },
    );
    _messageChanges = account.messageChanges().listen(
      (_) => _reloadChangedMessages(),
      onError: (Object exception, StackTrace stackTrace) {
        e(
          'Message revision stream failed: conversation_id=${conversation.id}',
          exception,
          stackTrace,
        );
      },
    );
  }

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final String? initialMessageId;
  StreamSubscription<rust.ConversationChangeEvent>? _conversationChanges;
  StreamSubscription<BigInt>? _messageChanges;
  int limit;
  var _generation = 0;
  var _loadAfterInFlight = false;
  var _loadBeforeInFlight = false;
  var _reloadInFlight = false;
  var _reloadPending = false;
  var _reloadRecentPending = false;
  var _disposed = false;
  late final MessageWindowLoader _messageWindowLoader =
      MessageWindowLoader.fromAccount(account, conversation.id);

  MessageState get state => value;

  @override
  void dispose() {
    _disposed = true;
    WidgetsBinding.instance.removeObserver(this);
    unawaited(_conversationChanges?.cancel());
    unawaited(_messageChanges?.cancel());
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed) return;
    _reloadChangedMessages(includeRecent: true);
  }

  bool _isCurrent(int generation, String conversationId) =>
      !_disposed &&
      generation == _generation &&
      conversation.id == conversationId;

  void _emit(MessageState nextState) {
    if (_disposed) return;
    value = nextState;
  }

  void _init({
    String? centerMessageId,
    String? lastReadMessageId,
    bool forceLatest = false,
    MessageWindowAnchor? anchor,
  }) {
    final generation = ++_generation;
    unawaited(
      _runInit(
        generation,
        centerMessageId: centerMessageId,
        lastReadMessageId: lastReadMessageId,
        forceLatest: forceLatest,
        anchor: anchor,
      ).catchError((Object error, StackTrace stackTrace) {
        e('message init failed: $error $stackTrace');
      }),
    );
  }

  Future<void> _runInit(
    int generation, {
    String? centerMessageId,
    String? lastReadMessageId,
    bool forceLatest = false,
    MessageWindowAnchor? anchor,
  }) async {
    final finalLimit = limit;
    final conversationId = conversation.id;
    final windowAnchor =
        anchor ??
        _resolveWindowAnchor(
          requestedCenterMessageId: centerMessageId,
          forceLatest: forceLatest,
        );
    final resolvedLastReadMessageId = resolveMessageWindowLastReadMessageId(
      requestedLastReadMessageId: lastReadMessageId,
      unseenMessageCount: conversation.unseenCount,
      conversationLastReadMessageId: conversation.lastReadMessageId,
    );

    traceChatJump(
      'message init '
      'conv=${shortMessageId(conversationId)} '
      'requestedCenter=${shortMessageId(centerMessageId)} '
      'anchor=${windowAnchor.debugName} '
      'anchorCenter=${shortMessageId(windowAnchor.centerMessageId)} '
      'inputLastRead=${shortMessageId(lastReadMessageId)} '
      'stateLastRead=${shortMessageId(state.lastReadMessageId)} '
      'convLastRead=${shortMessageId(conversation.lastReadMessageId)} '
      'unseen=${conversation.unseenCount} '
      'forceLatest=$forceLatest limit=$finalLimit',
    );

    final messageState = await _resetMessageList(
      conversationId,
      finalLimit,
      anchor: windowAnchor,
    );
    if (!_isCurrent(generation, conversationId)) return;

    final nextState = _pretreatment(
      messageState.copyWith(
        refreshKey: Object(),
        lastReadMessageId: resolvedLastReadMessageId,
      ),
    );
    traceChatJump(
      'message init loaded '
      'conv=${shortMessageId(conversationId)} '
      'lastRead=${shortMessageId(nextState.lastReadMessageId)} '
      '${_formatWindow(nextState)}',
    );
    _emit(nextState);
  }

  MessageWindowAnchor _resolveWindowAnchor({
    required String? requestedCenterMessageId,
    required bool forceLatest,
  }) {
    if (forceLatest) return const LatestMessageWindowAnchor();
    if (requestedCenterMessageId != null) {
      return AroundMessageWindowAnchor(
        messageId: requestedCenterMessageId,
        source: MessageWindowJumpSource.conversation,
      );
    }
    final lastReadMessageId = conversation.lastReadMessageId;
    if (conversation.unseenCount > 0 && lastReadMessageId != null) {
      return UnreadMessageWindowAnchor(lastReadMessageId: lastReadMessageId);
    }
    return const LatestMessageWindowAnchor();
  }

  void _reloadChangedMessages({bool includeRecent = false}) {
    _reloadRecentPending = _reloadRecentPending || includeRecent;
    if (_reloadInFlight) {
      _reloadPending = true;
      return;
    }
    unawaited(_runReloadChangedMessages());
  }

  Future<void> _runReloadChangedMessages() async {
    _reloadInFlight = true;
    do {
      _reloadPending = false;
      final includeRecent = _reloadRecentPending;
      _reloadRecentPending = false;
      try {
        if (state.isEmpty) {
          reload();
          continue;
        }
        final currentState = state;
        final loadedIds = currentState.list
            .map((message) => message.id)
            .where((id) => id.isNotEmpty)
            .toList(growable: false);
        final loadedFuture = _messageWindowLoader.messagesByIds(loadedIds);
        final recentFuture = includeRecent
            ? _messageWindowLoader.recentMessages(conversation.id, limit)
            : Future<List<MessageListEntry>>.value(const []);
        final results = await Future.wait([loadedFuture, recentFuture]);
        if (_disposed || !identical(currentState, state)) continue;

        final refreshedById = {
          for (final message in results[0]) message.id: message,
        };
        List<MessageListEntry> refreshPart(List<MessageListEntry> part) => part
            .map(
              (message) =>
                  message.id.isEmpty ? message : refreshedById[message.id],
            )
            .nonNulls
            .toList(growable: false);

        final refreshedState = MessageState(
          conversationId: currentState.conversationId,
          top: refreshPart(currentState.top),
          center: currentState.center == null
              ? null
              : refreshedById[currentState.center!.id],
          bottom: refreshPart(currentState.bottom),
          isLatest: currentState.isLatest,
          isOldest: currentState.isOldest,
          lastReadMessageId: currentState.lastReadMessageId,
          refreshKey: currentState.refreshKey,
          anchor: currentState.anchor,
        );
        final nextState = _insertOrReplace(
          conversation.id,
          results[1],
          currentState: refreshedState,
        );
        if (nextState != null) _emit(_pretreatment(nextState));
      } catch (exception, stackTrace) {
        e(
          'Refresh message window failed: conversation_id=${conversation.id}',
          exception,
          stackTrace,
        );
      }
    } while (_reloadPending && !_disposed);
    _reloadInFlight = false;
  }

  void after() {
    if (_loadAfterInFlight || state.isLatest) return;
    final conversationId = conversation.id;
    if (state.conversationId != conversationId) {
      return;
    }

    _loadAfterInFlight = true;
    final generation = _generation;
    unawaited(
      _loadAfter(generation, conversationId)
          .catchError((Object error, StackTrace stackTrace) {
            e('message load after failed: $error $stackTrace');
          })
          .whenComplete(() => _loadAfterInFlight = false),
    );
  }

  void before() {
    if (_loadBeforeInFlight || state.isOldest) return;
    final conversationId = conversation.id;
    if (state.conversationId != conversationId) {
      return;
    }

    _loadBeforeInFlight = true;
    final generation = _generation;
    unawaited(
      _loadBefore(generation, conversationId)
          .catchError((Object error, StackTrace stackTrace) {
            e('message load before failed: $error $stackTrace');
          })
          .whenComplete(() => _loadBeforeInFlight = false),
    );
  }

  Future<void> _loadAfter(int generation, String conversationId) async {
    final messageState = await _after(conversationId);
    if (!_isCurrent(generation, conversationId)) return;
    _emit(_pretreatment(messageState));
  }

  Future<void> _loadBefore(int generation, String conversationId) async {
    final messageState = await _before(conversationId);
    if (!_isCurrent(generation, conversationId)) return;
    _emit(_pretreatment(messageState));
  }

  Future<MessageState> _before(String conversationId) =>
      _messageWindowLoader.loadBefore(state, conversationId, limit);

  Future<MessageState> _after(String conversationId) =>
      _messageWindowLoader.loadAfter(state, conversationId, limit);

  Future<MessageState> _resetMessageList(
    String conversationId,
    int limit, {
    required MessageWindowAnchor anchor,
  }) async {
    traceChatJump(
      'reset list '
      'conv=${shortMessageId(conversationId)} '
      'anchor=${anchor.debugName} '
      'resolved=${shortMessageId(anchor.centerMessageId)} '
      'convLastRead=${shortMessageId(conversation.lastReadMessageId)} '
      'unseen=${conversation.unseenCount}',
    );

    final state = await _messagesByConversationId(
      conversationId,
      limit,
      anchor: anchor,
    );

    return state.copyWith(
      conversationId: conversationId,
      center: state.center,
      bottom: state.bottom,
      top: state.top,
    );
  }

  Future<MessageState> _messagesByConversationId(
    String conversationId,
    int limit, {
    required MessageWindowAnchor anchor,
  }) => _messageWindowLoader.load(
    conversationId,
    limit,
    anchor: anchor,
    trace: traceChatJump,
  );

  Future<MessageWindowDirection?> restoreDirectionFromSource({
    required String? sourceMessageId,
    required String targetMessageId,
  }) => _messageWindowLoader.directionFromSource(
    sourceMessageId: sourceMessageId,
    targetMessageId: targetMessageId,
  );

  MessageState? _insertOrReplace(
    String conversationId,
    List<MessageListEntry> list, {
    MessageState? currentState,
  }) {
    final current = currentState ?? state;
    var top = current.top.toList();
    var center = current.center;
    var bottom = current.bottom.toList();

    for (final item in list) {
      if (item.conversationId != conversationId) continue;

      if (item.id == center?.id) {
        center = item;
        continue;
      }

      final topIndex = top.indexWhere((element) => element.id == item.id);
      if (topIndex > -1) {
        top[topIndex] = item;
        continue;
      }

      final bottomIndex = bottom.indexWhere((element) => element.id == item.id);
      if (bottomIndex > -1) {
        bottom[bottomIndex] = item;
        continue;
      }

      // if don't have messages or older message after then valid item
      if (current.topMessage?.createdAt.isAfter(item.createdAt) ?? false) {
        continue;
      }

      final currentUserSent = item.senderRelationship.toUpperCase() == 'ME';
      if (current.isLatest) {
        if (center?.createdAt.isBefore(item.createdAt) ?? true) {
          bottom = [...bottom, item]
            ..sort((a, b) => a.createdAt.compareTo(b.createdAt));
        } else {
          top = [item, ...top]
            ..sort((a, b) => b.createdAt.compareTo(a.createdAt));
        }
      } else {
        if (currentUserSent && item.status.toUpperCase() == 'SENDING') {
          loadLatestWindow();
          return null;
        }
      }
    }

    return current.copyWith(top: top, center: center, bottom: bottom);
  }

  Future<void> refreshMessages(Iterable<String> messageIds) async {
    final ids = messageIds.toSet().toList(growable: false);
    if (ids.isEmpty || state.conversationId != conversation.id) return;
    try {
      final messages = await _messageWindowLoader.messagesByIds(ids);
      if (_disposed) return;
      final nextState = _insertOrReplace(conversation.id, messages);
      if (nextState != null) _emit(_pretreatment(nextState));
    } catch (exception, stackTrace) {
      e(
        'Refresh messages failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
    }
  }

  void removeMessages(Iterable<String> messageIds) {
    var nextState = state;
    for (final messageId in messageIds) {
      nextState = nextState.removeMessage(messageId);
    }
    if (!identical(nextState, state)) _emit(_pretreatment(nextState));
  }

  void loadAroundMessage(String messageId) {
    traceChatJump('message loadAround target=${shortMessageId(messageId)}');
    _init(
      centerMessageId: messageId,
      lastReadMessageId: state.lastReadMessageId,
      anchor: AroundMessageWindowAnchor(
        messageId: messageId,
        source: MessageWindowJumpSource.message,
      ),
    );
  }

  void reload() {
    _init();
  }

  void loadLatestWindow() {
    _init(
      lastReadMessageId: state.lastReadMessageId,
      anchor: const LatestMessageWindowAnchor(),
    );
  }

  MessageState _pretreatment(MessageState messageState) {
    final lifecycle = WidgetsBinding.instance.lifecycleState;
    if (lifecycle == null || lifecycle == AppLifecycleState.resumed) {
      _markConversationRead();
    }
    return messageState;
  }

  void _markConversationRead() {
    unawaited(
      account
          .message()
          .markConversationRead(conversationId: conversation.id)
          .catchError((Object exception, StackTrace stackTrace) {
            e(
              'Mark conversation read failed: conversation_id=${conversation.id}',
              exception,
              stackTrace,
            );
          }),
    );
  }

  String _formatWindow(MessageState state) =>
      'top=${state.top.length} '
      'topFirst=${shortMessageId(state.top.firstOrNull?.id)} '
      'topLast=${shortMessageId(state.top.lastOrNull?.id)} '
      'center=${shortMessageId(state.center?.id)} '
      'bottom=${state.bottom.length} '
      'bottomFirst=${shortMessageId(state.bottom.firstOrNull?.id)} '
      'bottomLast=${shortMessageId(state.bottom.lastOrNull?.id)} '
      'latest=${state.isLatest} oldest=${state.isOldest} '
      'anchor=${state.anchor.debugName}';
}
