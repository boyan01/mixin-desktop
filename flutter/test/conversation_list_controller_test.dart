import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/conversation_list_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

void main() {
  test('loads once and batches exact conversation id updates', () async {
    final account = _FakeAccount();
    final controller = ConversationListController(account);
    addTearDown(controller.dispose);
    addTearDown(account.dispose);

    await _waitUntil(() => controller.initialized);
    expect(account.fullLoads, 1);
    expect(controller.visibleConversations.map((item) => item.id), ['first']);

    account.items['first'] = _item('first', name: 'First updated', time: 2);
    account.items['second'] = _item('second', name: 'Second', time: 3);
    account.changes
      ..add(
        const ConversationChangeEvent(
          conversationIds: ['first'],
          reloadAll: false,
        ),
      )
      ..add(
        const ConversationChangeEvent(
          conversationIds: ['second'],
          reloadAll: false,
        ),
      );

    await _waitUntil(() => controller.visibleConversations.length == 2);
    expect(account.requestedIds.single, {'first', 'second'});
    expect(account.fullLoads, 1);
    expect(controller.visibleConversations.map((item) => item.id), [
      'second',
      'first',
    ]);
  });
}

Future<void> _waitUntil(bool Function() predicate) async {
  for (var attempt = 0; attempt < 100; attempt++) {
    if (predicate()) return;
    await Future<void>.delayed(const Duration(milliseconds: 5));
  }
  fail('condition was not reached');
}

class _FakeAccount implements AccountHandle, ConversationAccess, UserAccess {
  final changes = StreamController<ConversationChangeEvent>.broadcast();
  final items = <String, ConversationListItem>{
    'first': _item('first', name: 'First', time: 1),
  };
  final requestedIds = <Set<String>>[];
  int fullLoads = 0;
  int incrementalLoads = 0;

  @override
  ConversationAccess conversation() => this;

  @override
  UserAccess user() => this;

  @override
  AccountProfile profile() => const AccountProfile(
    userId: 'me',
    fullName: 'Me',
    avatarUrl: '',
    identityNumber: '1',
    biography: '',
    phone: '',
    createdAt: '',
    isVerified: false,
    fiatCurrency: 'USD',
  );

  @override
  Stream<AccountProfile> profileChanges() => const Stream.empty();

  @override
  Stream<ConversationChangeEvent> conversationChanges() => changes.stream;

  @override
  Future<List<ConversationListItem>> conversationItems() async {
    fullLoads++;
    return items.values.toList(growable: false);
  }

  @override
  Future<List<ConversationListItem>> conversationItemsByIds({
    required List<String> conversationIds,
  }) async {
    incrementalLoads++;
    requestedIds.add(conversationIds.toSet());
    return conversationIds
        .map((id) => items[id])
        .whereType<ConversationListItem>()
        .toList(growable: false);
  }

  @override
  Future<List<CircleItem>> circles() async => const [];

  @override
  Future<List<ConversationUnseenCount>> unseenConversationCounts() async =>
      const [];

  @override
  Future<List<UserProfileItem>> usersByIdentityNumbers({
    required List<String> identityNumbers,
  }) async => const [];

  @override
  Future<List<String>> replaceMentions({
    required List<String> contents,
  }) async => contents;

  @override
  Future<Map<String, String>> mentionNames({
    required List<String> contents,
  }) async => const {};

  @override
  void dispose() => changes.close();

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

ConversationListItem _item(
  String id, {
  required String name,
  required int time,
}) => ConversationListItem(
  conversationId: id,
  ownerId: '$id-owner',
  name: name,
  avatarUrl: '',
  category: 'CONTACT',
  draft: '',
  status: 2,
  lastMessage: '',
  updatedAtMillis: time,
  unseenCount: 0,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isScam: false,
  isBot: false,
  isBotGroup: false,
  isPinned: false,
  pinTimeMillis: 0,
  relationship: 'FRIEND',
  identityNumber: '',
  circleIds: const [],
  participantCount: 0,
  groupAvatars: const [],
);
