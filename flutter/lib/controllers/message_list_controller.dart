import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:mixin_desktop_ui/controllers/voice_recorder_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;

const _messagePageLimit = 60;
const _initialUnreadBefore = 15;
const _initialUnreadAfterMinimum = 45;
const _initialUnreadAfterMaximum = 200;
final _mentionIdentityNumberPattern = RegExp(r'@(\d+)');

class MessageListController extends ChangeNotifier with WidgetsBindingObserver {
  MessageListController({required this.account, required this.conversation})
    : unreadBoundaryMessageId = conversation.unseenCount > 0
          ? conversation.lastReadMessageId
          : null {
    WidgetsBinding.instance.addObserver(this);
    unawaited(_loadInitial());
    _changeSubscription = account.messageChanges().listen(
      (_) => _scheduleRefresh(),
      onError: _setError,
    );
  }

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final String? unreadBoundaryMessageId;

  List<MessageListEntry> messages = const [];
  List<MessageListEntry> pinnedMessages = const [];
  Map<String, String> mentionNames = const {};
  bool loading = true;
  bool loadingOlder = false;
  bool locating = false;
  bool sending = false;
  bool forwarding = false;
  bool hasMore = true;
  String? error;
  String? currentUserRole;
  bool initialUnreadAnchorAttempted = false;
  bool initialUnreadAnchorPending = false;
  bool initialUnreadAnchorConsumed = false;
  int? initialUnreadMessageIndex;
  String? initialUnreadMessageId;

  StreamSubscription<BigInt>? _changeSubscription;
  bool _refreshing = false;
  bool _refreshPending = false;
  bool _refreshDeferredUntilInitialUnreadAnchor = false;
  bool _disposed = false;
  final Set<String> _markingMentionRead = {};
  final Set<String> _markedMentionRead = {};
  final Set<String> _queriedMentionIdentityNumbers = {};

  bool get _canMarkRead {
    final state = WidgetsBinding.instance.lifecycleState;
    return state == null || state == AppLifecycleState.resumed;
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _scheduleRefresh();
  }

  Future<void> loadOlder() async {
    if (loading || loadingOlder || !hasMore || messages.isEmpty) return;
    loadingOlder = true;
    notifyListeners();
    try {
      final page = await _loadPage(
        beforeCreatedAt: messages.first.createdAt,
        beforeMessageId: messages.first.id,
      );
      if (_disposed) return;
      final existingIds = messages.map((message) => message.id).toSet();
      messages = [
        ...page.where((message) => !existingIds.contains(message.id)),
        ...messages,
      ];
      await _syncMentionNames(page);
      _syncInitialUnreadMessageIndex();
      hasMore = page.length == _messagePageLimit;
      error = null;
    } catch (exception) {
      _setError(exception);
    } finally {
      if (!_disposed) {
        loadingOlder = false;
        notifyListeners();
      }
    }
  }

  Future<bool> sendText(String content, {String? quoteMessageId}) async {
    final text = content.trim();
    if (text.isEmpty || sending) return false;
    sending = true;
    error = null;
    notifyListeners();
    try {
      await account.message().sendText(
        conversationId: conversation.id,
        content: text,
        quoteMessageId: quoteMessageId,
      );
      await _refreshLatest();
      return true;
    } catch (exception) {
      _setError(exception);
      return false;
    } finally {
      if (!_disposed) {
        sending = false;
        notifyListeners();
      }
    }
  }

  Future<bool> sendAudio({
    required String path,
    required Duration duration,
    required List<int> waveform,
    String? quoteMessageId,
  }) async {
    if (sending ||
        duration <= Duration.zero ||
        duration > maxVoiceRecordingDuration ||
        waveform.isEmpty) {
      return false;
    }
    sending = true;
    error = null;
    notifyListeners();
    try {
      await account.message().sendAudio(
        conversationId: conversation.id,
        path: path,
        durationMillis: duration.inMilliseconds,
        waveform: waveform,
        quoteMessageId: quoteMessageId,
      );
      await _refreshLatest();
      return true;
    } catch (exception) {
      _setError(exception);
      return false;
    } finally {
      if (!_disposed) {
        sending = false;
        notifyListeners();
      }
    }
  }

  Future<bool> sendSticker({required String stickerId}) async {
    if (sending || stickerId.trim().isEmpty) return false;
    sending = true;
    error = null;
    notifyListeners();
    try {
      await account.message().sendSticker(
        conversationId: conversation.id,
        stickerId: stickerId,
      );
      await _refreshLatest();
      return true;
    } catch (exception) {
      _setError(exception);
      return false;
    } finally {
      if (!_disposed) {
        sending = false;
        notifyListeners();
      }
    }
  }

  Future<bool> forwardMessages(
    Iterable<MessageListEntry> messages,
    String targetConversationId,
  ) async {
    final selected = messages.toList(growable: false);
    if (selected.isEmpty || forwarding) return false;
    forwarding = true;
    error = null;
    notifyListeners();
    try {
      await account.message().forwardMessages(
        targetConversationId: targetConversationId,
        sourceMessageIds: selected.map((message) => message.id).toList(),
      );
      return true;
    } catch (exception) {
      _setError(exception);
      return false;
    } finally {
      if (!_disposed) {
        forwarding = false;
        notifyListeners();
      }
    }
  }

  Future<bool> combineForwardMessages(
    Iterable<MessageListEntry> messages,
    String targetConversationId,
  ) async {
    final selected = messages.toList(growable: false);
    if (selected.length < 2 || selected.length >= 100 || forwarding) {
      return false;
    }
    forwarding = true;
    error = null;
    notifyListeners();
    try {
      await account.message().combineForwardMessages(
        targetConversationId: targetConversationId,
        sourceMessageIds: selected.map((message) => message.id).toList(),
      );
      return true;
    } catch (exception) {
      _setError(exception);
      return false;
    } finally {
      if (!_disposed) {
        forwarding = false;
        notifyListeners();
      }
    }
  }

  Future<void> setMessagePinned(MessageListEntry message, bool pinned) async {
    try {
      await account.message().setMessagePinned(
        conversationId: message.conversationId,
        messageId: message.id,
        pinned: pinned,
      );
      await _refreshLatest();
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<void> recallMessages(Iterable<MessageListEntry> messages) async {
    final selected = messages.toList(growable: false);
    if (selected.isEmpty) return;
    try {
      await account.message().recallMessages(
        conversationId: conversation.id,
        messageIds: selected.map((message) => message.id).toList(),
      );
      await _refreshLatest();
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<void> deleteMessages(Iterable<MessageListEntry> messages) async {
    final selected = messages.toList(growable: false);
    if (selected.isEmpty) return;
    try {
      await account.message().deleteMessages(
        conversationId: conversation.id,
        messageIds: selected.map((message) => message.id).toList(),
      );
      await _refreshLatest();
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<void> markMentionRead(MessageListEntry message) async {
    if (message.mentionRead != false ||
        _markingMentionRead.contains(message.id) ||
        _markedMentionRead.contains(message.id)) {
      return;
    }
    _markingMentionRead.add(message.id);
    try {
      await account.message().markMentionRead(
        conversationId: message.conversationId,
        messageId: message.id,
      );
      _markedMentionRead.add(message.id);
    } catch (exception) {
      _setError(exception);
    } finally {
      _markingMentionRead.remove(message.id);
    }
  }

  Future<void> downloadAttachment(MessageListEntry message) async {
    try {
      await account.attachment().downloadAttachment(messageId: message.id);
      await _refreshLatest();
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<void> cancelAttachment(MessageListEntry message) async {
    try {
      await account.attachment().cancelAttachment(messageId: message.id);
      await _refreshLatest();
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<void> markAudioRead(MessageListEntry message) async {
    if (message.mediaStatus.toUpperCase() != 'DONE') return;
    try {
      await account.attachment().markAudioRead(messageId: message.id);
      await _refreshLatest();
    } catch (exception) {
      _setError(exception);
    }
  }

  Future<void> addSticker(MessageListEntry message) async {
    final stickerId = message.stickerId?.trim();
    if (stickerId == null || stickerId.isEmpty) {
      throw ArgumentError('Sticker message has no sticker id');
    }
    try {
      await account.sticker().addSticker(stickerId: stickerId);
      error = null;
      if (!_disposed) notifyListeners();
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<void> addImageAsSticker(MessageListEntry message) async {
    try {
      await account.sticker().addStickerFromFile(messageId: message.id);
      error = null;
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<String?> handleStrangerAction(
    MessageListEntry message,
    String action,
  ) async {
    try {
      switch (action) {
        case 'block':
          await account.user().blockUser(userId: message.senderId);
          await _refreshLatest();
          break;
        case 'add_contact':
          await account.user().addContact(
            userId: message.senderId,
            fullName: message.senderName,
          );
          await _refreshLatest();
          break;
        case 'say_hi':
          await account.message().sendText(
            conversationId: conversation.id,
            content: 'Hi',
            quoteMessageId: null,
          );
          await _refreshLatest();
          break;
        case 'open_home':
          final appId = message.senderAppId?.trim();
          if (appId == null || appId.isEmpty) return null;
          return account.user().botHomeUri(appId: appId);
        default:
          return null;
      }
      return null;
    } catch (exception) {
      _setError(exception);
      rethrow;
    }
  }

  Future<bool> locateMessage(
    String messageId, {
    int before = 30,
    int after = 30,
  }) async {
    if (messages.any((message) => message.id == messageId)) return true;
    if (locating) return false;
    locating = true;
    error = null;
    notifyListeners();
    try {
      final result = await account.message().messagesAround(
        conversationId: conversation.id,
        targetMessageId: messageId,
        before: before,
        after: after,
      );
      if (_disposed) return false;
      final window = result
          .map(MessageListEntry.fromRust)
          .toList(growable: false);
      final targetIndex = window.indexWhere(
        (message) => message.id == messageId,
      );
      if (targetIndex < 0) return false;

      final currentIds = messages.map((message) => message.id).toSet();
      final overlaps = window.any((message) => currentIds.contains(message.id));
      if (overlaps) {
        final byId = <String, MessageListEntry>{
          for (final message in messages) message.id: message,
          for (final message in window) message.id: message,
        };
        messages = byId.values.toList(growable: false)..sort(_compareMessages);
      } else {
        messages = window;
      }
      await _syncMentionNames(window);
      _syncInitialUnreadMessageIndex();
      hasMore = targetIndex >= before;
      error = null;
      return true;
    } catch (exception) {
      _setError(exception);
      return false;
    } finally {
      if (!_disposed) {
        locating = false;
        notifyListeners();
      }
    }
  }

  Future<void> retry() => _loadInitial();

  Future<void> _loadInitial() async {
    loading = true;
    error = null;
    initialUnreadAnchorAttempted = false;
    initialUnreadAnchorPending = false;
    initialUnreadAnchorConsumed = false;
    initialUnreadMessageIndex = null;
    initialUnreadMessageId = null;
    notifyListeners();
    try {
      final roleFuture = account.conversation().currentUserRole(
        conversationId: conversation.id,
      );
      final pinnedFuture = _loadPinnedMessages();
      final unreadWindow = await _loadInitialUnreadWindow();
      final page = unreadWindow ?? await _loadPage();
      currentUserRole = await roleFuture;
      pinnedMessages = await pinnedFuture;
      if (_disposed) return;
      messages = page;
      await _syncMentionNames(page);
      if (unreadWindow == null) {
        hasMore = page.length == _messagePageLimit;
      }
      if (_canMarkRead) {
        await account.message().markConversationRead(
          conversationId: conversation.id,
        );
      }
    } catch (exception) {
      _setError(exception);
    } finally {
      if (!_disposed) {
        loading = false;
        notifyListeners();
      }
    }
  }

  Future<List<MessageListEntry>?> _loadInitialUnreadWindow() async {
    final targetMessageId = unreadBoundaryMessageId?.trim();
    if (targetMessageId == null || targetMessageId.isEmpty) return null;

    initialUnreadAnchorAttempted = true;
    final after = conversation.unseenCount
        .clamp(_initialUnreadAfterMinimum, _initialUnreadAfterMaximum)
        .toInt();
    try {
      final result = await account.message().messagesAround(
        conversationId: conversation.id,
        targetMessageId: targetMessageId,
        before: _initialUnreadBefore,
        after: after,
      );
      final window = result
          .map(MessageListEntry.fromRust)
          .toList(growable: false);
      final boundaryIndex = window.indexWhere(
        (message) => message.id == targetMessageId,
      );
      final firstUnreadIndex = boundaryIndex + 1;
      if (boundaryIndex < 0 || firstUnreadIndex >= window.length) return null;

      initialUnreadMessageIndex = firstUnreadIndex;
      initialUnreadMessageId = window[firstUnreadIndex].id;
      initialUnreadAnchorPending = true;
      hasMore = boundaryIndex >= _initialUnreadBefore;
      return window;
    } on Object {
      return null;
    }
  }

  Future<List<MessageListEntry>> _loadPage({
    DateTime? beforeCreatedAt,
    String? beforeMessageId,
    int limit = _messagePageLimit,
  }) async {
    final result = await account.message().messages(
      conversationId: conversation.id,
      beforeCreatedAtMicros: beforeCreatedAt?.microsecondsSinceEpoch,
      beforeMessageId: beforeMessageId,
      limit: limit,
    );
    return result
        .map(MessageListEntry.fromRust)
        .toList(growable: false)
        .reversed
        .toList(growable: false);
  }

  Future<List<MessageListEntry>> _loadPinnedMessages() async {
    final result = await account.message().pinnedMessages(
      conversationId: conversation.id,
    );
    return result.map(MessageListEntry.fromRust).toList(growable: false);
  }

  void _scheduleRefresh() {
    if (initialUnreadAnchorPending) {
      _refreshDeferredUntilInitialUnreadAnchor = true;
      return;
    }
    if (_refreshing) {
      _refreshPending = true;
      return;
    }
    unawaited(_refreshLatest());
  }

  Future<void> _refreshLatest() async {
    if (_refreshing) {
      _refreshPending = true;
      return;
    }
    _refreshing = true;
    do {
      _refreshPending = false;
      try {
        final refreshLimit = messages.length.clamp(_messagePageLimit, 200);
        final latest = await _loadPage(limit: refreshLimit);
        if (_disposed) return;
        final retainedCount = (messages.length - refreshLimit).clamp(
          0,
          messages.length,
        );
        final byId = <String, MessageListEntry>{
          for (final message
              in initialUnreadMessageId == null
                  ? messages.take(retainedCount)
                  : messages)
            message.id: message,
          for (final message in latest) message.id: message,
        };
        messages = byId.values.toList(growable: false)..sort(_compareMessages);
        await _syncMentionNames(latest);
        pinnedMessages = await _loadPinnedMessages();
        _syncInitialUnreadMessageIndex();
        if (_canMarkRead) {
          await account.message().markConversationRead(
            conversationId: conversation.id,
          );
        }
        error = null;
        notifyListeners();
      } catch (exception) {
        _setError(exception);
      }
    } while (_refreshPending && !_disposed);
    _refreshing = false;
  }

  void consumeInitialUnreadAnchor() {
    if (!initialUnreadAnchorPending) return;
    initialUnreadAnchorPending = false;
    initialUnreadAnchorConsumed = true;
    if (_refreshDeferredUntilInitialUnreadAnchor) {
      _refreshDeferredUntilInitialUnreadAnchor = false;
      _scheduleRefresh();
    }
  }

  void _syncInitialUnreadMessageIndex() {
    final messageId = initialUnreadMessageId;
    if (messageId == null) return;
    final index = messages.indexWhere((message) => message.id == messageId);
    initialUnreadMessageIndex = index < 0 ? null : index;
  }

  Future<void> _syncMentionNames(Iterable<MessageListEntry> entries) async {
    final identityNumbers = <String>{};
    for (final message in entries) {
      for (final text in [message.content, message.caption ?? '']) {
        for (final match in _mentionIdentityNumberPattern.allMatches(text)) {
          identityNumbers.add(match.group(1)!);
        }
      }
    }
    identityNumbers.removeAll(_queriedMentionIdentityNumbers);
    if (identityNumbers.isEmpty) return;

    _queriedMentionIdentityNumbers.addAll(identityNumbers);
    try {
      final users = await account.user().usersByIdentityNumbers(
        identityNumbers: identityNumbers.toList(growable: false),
      );
      if (_disposed) return;
      final resolved = <String, String>{};
      for (final user in users) {
        final fullName = user.fullName.trim();
        if (fullName.isNotEmpty) {
          resolved[user.identityNumber] = fullName;
        }
      }
      if (resolved.isNotEmpty) {
        mentionNames = Map.unmodifiable({...mentionNames, ...resolved});
      }
    } catch (_) {
      _queriedMentionIdentityNumbers.removeAll(identityNumbers);
    }
  }

  static int _compareMessages(MessageListEntry first, MessageListEntry second) {
    final created = first.createdAt.compareTo(second.createdAt);
    return created != 0 ? created : first.id.compareTo(second.id);
  }

  void _setError(Object exception) {
    if (_disposed) return;
    error = exception.toString();
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    WidgetsBinding.instance.removeObserver(this);
    unawaited(_changeSubscription?.cancel());
    super.dispose();
  }
}
