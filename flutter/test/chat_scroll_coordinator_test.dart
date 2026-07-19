import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/widgets/chat/chat_scroll_coordinator.dart';

void main() {
  testWidgets('viewport anchor correction runs after sliver layout', (
    tester,
  ) async {
    final coordinator = ChatScrollCoordinator();
    addTearDown(coordinator.dispose);
    final viewportKey = GlobalKey();
    final messageKey = const MessageGlobalKey('message');
    coordinator
      ..viewportKey = viewportKey
      ..registerRenderedMessageId('message');

    Widget build(double spacerHeight) => Directionality(
      textDirection: TextDirection.ltr,
      child: FractionalTranslation(
        translation: const Offset(0.1, 0),
        child: SizedBox(
          key: viewportKey,
          height: 200,
          child: CustomScrollView(
            controller: coordinator.scrollController,
            slivers: [
              SliverList.list(
                children: [
                  SizedBox(height: spacerHeight),
                  SizedBox(key: messageKey, height: 40),
                  const SizedBox(height: 800),
                ],
              ),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(build(200));
    coordinator.scrollController.jumpTo(180);
    await tester.pump();
    final initialTop = tester.getTopLeft(find.byKey(messageKey)).dy;
    coordinator.captureViewportState([_message], {'message': messageKey});

    await tester.pumpWidget(build(220));
    coordinator.scheduleRestore(
      messages: [_message],
      keysByMessageId: {'message': messageKey},
      reset: false,
      isLatest: false,
    );
    await tester.pump();
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(
      tester.getTopLeft(find.byKey(messageKey)).dy,
      moreOrLessEquals(initialTop, epsilon: 0.5),
    );
  });
}

final _message = MessageListEntry(
  id: 'message',
  conversationId: 'conversation',
  senderId: 'sender',
  senderName: 'Sender',
  senderAvatarUrl: '',
  senderIsVerified: false,
  category: 'PLAIN_TEXT',
  content: 'Message',
  status: 'READ',
  createdAt: DateTime(2026),
  mediaDuration: '',
  mediaStatus: '',
);
