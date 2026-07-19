import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/message_controller.dart';
import 'package:mixin_desktop_ui/controllers/message_action_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

void main() {
  testWidgets('marks messages read only while the app is active', (
    tester,
  ) async {
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    addTearDown(
      () => tester.binding.handleAppLifecycleStateChanged(
        AppLifecycleState.resumed,
      ),
    );
    final account = _FakeAccountHandle([_message('first')]);
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    final controller = MessageActionController(
      account: account,
      conversation: _conversation,
      messageController: messages,
    );
    addTearDown(() {
      controller.dispose();
      messages.dispose();
      account.close();
    });

    await tester.pumpAndSettle();
    expect(account.markReadCalls, 0);

    account.messagesInDatabase.add(_message('second'));
    account.notifyChanged();
    await tester.pumpAndSettle();
    expect(account.markReadCalls, 0);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(account.markReadCalls, 1);
  });

  testWidgets('coalesces concurrent mark read requests with a trailing run', (
    tester,
  ) async {
    final account = _FakeAccountHandle([_message('first')]);
    final firstMarkRead = Completer<void>();
    account.markReadCompleter = firstMarkRead;
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    addTearDown(() {
      messages.dispose();
      account.close();
    });

    await tester.pump();
    expect(account.markReadCalls, 1);

    account.notifyMessageRevision();
    account.notifyMessageRevision();
    await tester.pump();
    await tester.pump();
    expect(account.markReadCalls, 1);

    account.markReadCompleter = null;
    firstMarkRead.complete();
    await tester.pumpAndSettle();
    expect(account.markReadCalls, 2);
  });

  testWidgets('refresh replaces messages deleted by Rust core', (tester) async {
    final account = _FakeAccountHandle([
      _message('first'),
      _message('expired'),
    ]);
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    final controller = MessageActionController(
      account: account,
      conversation: _conversation,
      messageController: messages,
    );
    addTearDown(() {
      controller.dispose();
      messages.dispose();
      account.close();
    });
    await tester.pumpAndSettle();
    expect(controller.messages.map((message) => message.id), [
      'first',
      'expired',
    ]);

    account.messagesInDatabase.removeWhere(
      (message) => message.messageId == 'expired',
    );
    account.notifyChanged();
    await tester.pumpAndSettle();

    expect(controller.messages.map((message) => message.id), ['first']);
  });

  testWidgets('message revision refreshes attachment state', (tester) async {
    final account = _FakeAccountHandle([
      _message('attachment', mediaStatus: 'CANCELED'),
    ]);
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    addTearDown(() {
      messages.dispose();
      account.close();
    });
    await tester.pumpAndSettle();

    account.messagesInDatabase[0] = _message('attachment', mediaStatus: 'DONE');
    account.notifyMessageRevision();
    await tester.pumpAndSettle();

    expect(messages.state.list.single.mediaStatus, 'DONE');
  });

  testWidgets('new message refresh keeps loaded history', (tester) async {
    final account = _FakeAccountHandle([
      for (var index = 0; index < 120; index++)
        _message('message-$index', minute: index),
    ]);
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    addTearDown(() {
      messages.dispose();
      account.close();
    });
    await tester.pumpAndSettle();

    messages.before();
    await tester.pumpAndSettle();
    expect(messages.state.list, hasLength(120));

    account.messagesInDatabase.add(_message('new-message', minute: 120));
    account.notifyChanged();
    await tester.pumpAndSettle();

    expect(messages.state.list, hasLength(121));
    expect(
      messages.state.list.map((message) => message.id),
      containsAll(['message-0', 'new-message']),
    );
  });

  testWidgets('loads the initial window around the unread boundary', (
    tester,
  ) async {
    final account = _FakeAccountHandle([
      _message('older'),
      _message('last-read'),
      _message('first-unread'),
      _message('latest'),
    ]);
    final messages = MessageController(
      account: account,
      conversation: _unreadConversation,
      limit: 60,
    );
    final controller = MessageActionController(
      account: account,
      conversation: _unreadConversation,
      messageController: messages,
    );
    addTearDown(() {
      controller.dispose();
      messages.dispose();
      account.close();
    });

    await tester.pumpAndSettle();

    expect(account.messagesAroundCalls, 0);
    expect(messages.state.center?.id, 'last-read');
    expect(messages.state.lastReadMessageId, 'last-read');
    expect(controller.messages.map((message) => message.id), [
      'older',
      'last-read',
      'first-unread',
      'latest',
    ]);
  });

  testWidgets('sends recorded audio through the Rust account handle', (
    tester,
  ) async {
    final account = _FakeAccountHandle([]);
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    final controller = MessageActionController(
      account: account,
      conversation: _conversation,
      messageController: messages,
    );
    addTearDown(() {
      controller.dispose();
      messages.dispose();
      account.close();
    });
    await tester.pumpAndSettle();

    final sent = await controller.sendAudio(
      path: '/tmp/voice.ogg',
      duration: const Duration(milliseconds: 1200),
      waveform: const [1, 2, 3],
      quoteMessageId: 'quoted',
    );

    expect(sent, isTrue);
    expect(account.sentAudioPath, '/tmp/voice.ogg');
    expect(account.sentAudioDuration, 1200);
    expect(account.sentAudioWaveform, [1, 2, 3]);
    expect(account.sentAudioQuoteId, 'quoted');
  });

  testWidgets('sends a sticker through the Rust account handle', (
    tester,
  ) async {
    final account = _FakeAccountHandle([]);
    final messages = MessageController(
      account: account,
      conversation: _conversation,
      limit: 60,
    );
    final controller = MessageActionController(
      account: account,
      conversation: _conversation,
      messageController: messages,
    );
    addTearDown(() {
      controller.dispose();
      messages.dispose();
      account.close();
    });
    await tester.pumpAndSettle();

    final sent = await controller.sendSticker(stickerId: 'sticker-id');

    expect(sent, isTrue);
    expect(account.sentStickerId, 'sticker-id');
    expect(account.sentStickerConversationId, 'conversation');
  });
}

final _conversation = ConversationListEntry(
  id: 'conversation',
  ownerId: 'other',
  name: 'Other',
  avatarUrl: '',
  category: 'CONTACT',
  draft: '',
  status: 2,
  content: '',
  contentType: null,
  messageStatus: null,
  senderId: null,
  senderName: null,
  updatedAt: DateTime(2026, 7, 16, 12),
  unseenCount: 0,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: false,
  isPinned: false,
  relationship: 'FRIEND',
  identityNumber: '7000',
  circleIds: [],
  groupAvatars: [],
);

final _unreadConversation = ConversationListEntry(
  id: 'conversation',
  ownerId: 'other',
  name: 'Other',
  avatarUrl: '',
  category: 'CONTACT',
  draft: '',
  status: 2,
  lastReadMessageId: 'last-read',
  content: '',
  contentType: null,
  messageStatus: null,
  senderId: null,
  senderName: null,
  updatedAt: DateTime(2026, 7, 16, 12),
  unseenCount: 2,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: false,
  isPinned: false,
  relationship: 'FRIEND',
  identityNumber: '7000',
  circleIds: [],
  groupAvatars: [],
);

MessageListItem _message(
  String id, {
  String mediaStatus = '',
  int minute = 0,
}) => MessageListItem(
  messageId: id,
  conversationId: 'conversation',
  senderId: 'other',
  senderName: 'Other',
  senderAvatarUrl: '',
  senderIsVerified: false,
  senderRelationship: 'FRIEND',
  senderIsScam: false,
  senderIsBot: false,
  category: 'PLAIN_TEXT',
  content: id,
  status: 'SENT',
  createdAtMicros: DateTime(
    2026,
    7,
    16,
    12,
  ).add(Duration(minutes: minute)).microsecondsSinceEpoch,
  mediaDuration: '',
  mediaStatus: mediaStatus,
  sharedUserIsVerified: false,
  pinned: false,
);

class _FakeAccountHandle
    implements
        AccountHandle,
        AttachmentAccess,
        ConversationAccess,
        MessageAccess,
        StickerAccess,
        UserAccess {
  _FakeAccountHandle(this.messagesInDatabase);

  final List<MessageListItem> messagesInDatabase;
  final _changes = StreamController<BigInt>.broadcast();
  final _conversationChanges =
      StreamController<ConversationChangeEvent>.broadcast();
  var markReadCalls = 0;
  Completer<void>? markReadCompleter;
  var messagesAroundCalls = 0;
  String? aroundTarget;
  int? aroundBefore;
  int? aroundAfter;
  String? sentAudioPath;
  int? sentAudioDuration;
  List<int>? sentAudioWaveform;
  String? sentAudioQuoteId;
  String? sentStickerId;
  String? sentStickerConversationId;

  @override
  AttachmentAccess attachment() => this;

  @override
  ConversationAccess conversation() => this;

  @override
  MessageAccess message() => this;

  @override
  StickerAccess sticker() => this;

  @override
  UserAccess user() => this;

  void notifyChanged() {
    _changes.add(BigInt.one);
    _conversationChanges.add(
      const ConversationChangeEvent(
        conversationIds: ['conversation'],
        reloadAll: false,
      ),
    );
  }

  void notifyMessageRevision() => _changes.add(BigInt.one);

  Future<void> close() async {
    await _changes.close();
    await _conversationChanges.close();
  }

  @override
  Stream<BigInt> messageChanges() => _changes.stream;

  @override
  Stream<ConversationChangeEvent> conversationChanges() =>
      _conversationChanges.stream;

  @override
  Stream<NotificationEvent> desktopNotificationEvents() => const Stream.empty();

  @override
  Future<List<MessageListItem>> messages({
    required String conversationId,
    int? beforeCreatedAtMicros,
    String? beforeMessageId,
    required int limit,
  }) async {
    final before = beforeMessageId == null
        ? messagesInDatabase.length
        : messagesInDatabase.indexWhere(
            (message) => message.messageId == beforeMessageId,
          );
    return messagesInDatabase
        .take(before < 0 ? 0 : before)
        .toList()
        .reversed
        .take(limit)
        .toList();
  }

  @override
  Future<List<MessageListItem>> messageItemsByIds({
    required List<String> messageIds,
  }) async {
    final ids = messageIds.toSet();
    return messagesInDatabase
        .where((message) => ids.contains(message.messageId))
        .toList(growable: false);
  }

  @override
  Future<MessageOrderInfoView?> messageOrderInfo({
    required String messageId,
  }) async {
    final index = messagesInDatabase.indexWhere(
      (message) => message.messageId == messageId,
    );
    if (index < 0) return null;
    return MessageOrderInfoView(
      messageId: messageId,
      rowId: index + 1,
      createdAtMicros: messagesInDatabase[index].createdAtMicros,
    );
  }

  @override
  Future<List<String>> messageIdsBefore({
    required String conversationId,
    required int anchorRowId,
    required int anchorCreatedAtMicros,
    required int limit,
  }) async => messagesInDatabase
      .take(anchorRowId - 1)
      .toList()
      .reversed
      .take(limit)
      .map((message) => message.messageId)
      .toList(growable: false);

  @override
  Future<List<String>> messageIdsAfter({
    required String conversationId,
    required int anchorRowId,
    required int anchorCreatedAtMicros,
    required int limit,
  }) async => messagesInDatabase
      .skip(anchorRowId)
      .take(limit)
      .map((message) => message.messageId)
      .toList(growable: false);

  @override
  Future<List<String>> pinnedMessageIds({
    required String conversationId,
  }) async => const [];

  @override
  Future<void> markConversationRead({required String conversationId}) async {
    markReadCalls++;
    await markReadCompleter?.future;
  }

  @override
  Future<List<MessageListItem>> messagesAround({
    required String conversationId,
    required String targetMessageId,
    required int before,
    required int after,
  }) async {
    messagesAroundCalls++;
    aroundTarget = targetMessageId;
    aroundBefore = before;
    aroundAfter = after;
    final target = messagesInDatabase.indexWhere(
      (message) => message.messageId == targetMessageId,
    );
    if (target < 0) return const [];
    final start = (target - before).clamp(0, messagesInDatabase.length);
    final end = (target + after + 1).clamp(0, messagesInDatabase.length);
    return messagesInDatabase.sublist(start, end);
  }

  @override
  Future<String> sendAudio({
    required String conversationId,
    required String path,
    required int durationMillis,
    required List<int> waveform,
    String? quoteMessageId,
  }) async {
    sentAudioPath = path;
    sentAudioDuration = durationMillis;
    sentAudioWaveform = waveform;
    sentAudioQuoteId = quoteMessageId;
    return 'audio-message';
  }

  @override
  Future<String> sendSticker({
    required String conversationId,
    required String stickerId,
  }) async {
    sentStickerConversationId = conversationId;
    sentStickerId = stickerId;
    return 'sticker-message';
  }

  @override
  Future<String?> currentUserRole({required String conversationId}) async =>
      null;

  @override
  Future<List<MessageListItem>> pinnedMessages({
    required String conversationId,
  }) async => const [];

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
