import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/message_list_controller.dart';
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
    final controller = MessageListController(
      account: account,
      conversation: _conversation,
    );
    addTearDown(() {
      controller.dispose();
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

  testWidgets('refresh replaces messages deleted by Rust core', (tester) async {
    final account = _FakeAccountHandle([
      _message('first'),
      _message('expired'),
    ]);
    final controller = MessageListController(
      account: account,
      conversation: _conversation,
    );
    addTearDown(() {
      controller.dispose();
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

  testWidgets('loads the initial window around the unread boundary', (
    tester,
  ) async {
    final account = _FakeAccountHandle([
      _message('older'),
      _message('last-read'),
      _message('first-unread'),
      _message('latest'),
    ]);
    final controller = MessageListController(
      account: account,
      conversation: _unreadConversation,
    );
    addTearDown(() {
      controller.dispose();
      account.close();
    });

    await tester.pumpAndSettle();

    expect(account.messagesAroundCalls, 1);
    expect(account.aroundTarget, 'last-read');
    expect(account.aroundBefore, 15);
    expect(account.aroundAfter, 45);
    expect(controller.initialUnreadAnchorAttempted, isTrue);
    expect(controller.initialUnreadAnchorPending, isTrue);
    expect(controller.initialUnreadMessageId, 'first-unread');
    expect(controller.initialUnreadMessageIndex, 2);
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
    final controller = MessageListController(
      account: account,
      conversation: _conversation,
    );
    addTearDown(() {
      controller.dispose();
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
    final controller = MessageListController(
      account: account,
      conversation: _conversation,
    );
    addTearDown(() {
      controller.dispose();
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

MessageListItem _message(String id) => MessageListItem(
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
  createdAtMicros: DateTime(2026, 7, 16, 12).microsecondsSinceEpoch,
  mediaDuration: '',
  mediaStatus: '',
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
  var markReadCalls = 0;
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

  void notifyChanged() => _changes.add(BigInt.one);
  Future<void> close() => _changes.close();

  @override
  Stream<BigInt> messageChanges() => _changes.stream;

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
  Future<void> markConversationRead({required String conversationId}) async {
    markReadCalls++;
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
    return List.of(messagesInDatabase);
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
