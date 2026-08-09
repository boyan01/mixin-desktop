import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/message_search_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

void main() {
  test('loads pages and appends the next page near the list end', () async {
    final account = _FakeAccountHandle();
    final controller = MessageSearchController(
      account: account,
      conversationId: 'conversation',
      keyword: 'hello',
      senderId: null,
      categories: null,
      limit: 1,
    );
    addTearDown(controller.dispose);

    expect(controller.value.loading, isTrue);
    account.completeNext([_message('first')]);
    await pumpEventQueue();
    expect(controller.value.items.map((item) => item.id), ['first']);
    expect(account.calls.single.anchorMessageId, isNull);

    (controller.itemPositionsListener.itemPositions
            as ValueNotifier<Iterable<ItemPosition>>)
        .value = const [
      ItemPosition(index: 0, itemLeadingEdge: 0, itemTrailingEdge: 1),
    ];
    account.completeNext([_message('second')]);
    await pumpEventQueue();

    expect(controller.value.items.map((item) => item.id), [
      'first',
      'second',
    ]);
    expect(account.calls.last.anchorMessageId, 'first');
  });

  test('supersedes a stale request and retries after a failure', () async {
    final account = _FakeAccountHandle();
    final controller = MessageSearchController(
      account: account,
      conversationId: 'conversation',
      keyword: 'hello',
      senderId: 'sender',
      categories: const ['PLAIN_TEXT'],
    );
    addTearDown(controller.dispose);

    final reload = controller.reload();
    account
      ..completeNext([_message('stale')])
      ..completeNext([_message('fresh')]);
    await reload;
    expect(controller.value.items.map((item) => item.id), ['fresh']);
    expect(account.calls[1].senderId, 'sender');
    expect(account.calls[1].categories, ['PLAIN_TEXT']);

    final failedReload = controller.reload();
    account.failNext(StateError('offline'));
    await failedReload;
    expect(controller.value.error, isA<StateError>());

    final retry = controller.reload();
    account.completeNext([_message('retry')]);
    await retry;
    expect(controller.value.error, isNull);
    expect(controller.value.items.map((item) => item.id), ['retry']);
  });

  test('ignores a result completed after dispose', () async {
    final account = _FakeAccountHandle();
    final controller = MessageSearchController(
      account: account,
      conversationId: 'conversation',
      keyword: 'hello',
      senderId: null,
      categories: null,
    );
    final pending = account.requests.single;

    controller.dispose();
    pending.complete([_message('late')]);
    await pumpEventQueue();

    expect(controller.value.items, isEmpty);
  });
}

class _FakeAccountHandle implements rust.AccountHandle, rust.MessageAccess {
  final requests = <Completer<List<rust.MessageListItem>>>[];
  final calls =
      <
        ({
          String conversationId,
          String query,
          String? senderId,
          List<String> categories,
          String? anchorMessageId,
          int limit,
        })
      >[];

  @override
  rust.MessageAccess message() => this;

  @override
  Future<List<rust.MessageListItem>> searchMessages({
    required String conversationId,
    required String query,
    required List<String> categories,
    required int limit,
    String? senderId,
    String? anchorMessageId,
  }) {
    calls.add(
      (
        conversationId: conversationId,
        query: query,
        senderId: senderId,
        categories: categories,
        anchorMessageId: anchorMessageId,
        limit: limit,
      ),
    );
    final request = Completer<List<rust.MessageListItem>>();
    requests.add(request);
    return request.future;
  }

  void completeNext(List<rust.MessageListItem> items) {
    requests.firstWhere((request) => !request.isCompleted).complete(items);
  }

  void failNext(Object error) {
    requests.firstWhere((request) => !request.isCompleted).completeError(error);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

rust.MessageListItem _message(String id) => rust.MessageListItem(
  messageId: id,
  conversationId: 'conversation',
  senderId: 'sender',
  senderName: 'Sender',
  senderAvatarUrl: '',
  senderIsVerified: false,
  senderRelationship: 'FRIEND',
  senderIsScam: false,
  senderIsBot: false,
  category: 'PLAIN_TEXT',
  content: id,
  status: 'READ',
  createdAtMicros: DateTime(2026, 8, 9).microsecondsSinceEpoch,
  mediaDuration: '',
  mediaStatus: '',
  sharedUserIsVerified: false,
  pinned: false,
);
