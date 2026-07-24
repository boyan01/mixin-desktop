import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/home_navigation_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';

void main() {
  test(
    'selecting a conversation clears navigation intent and records selection',
    () {
      final selected = <ConversationListEntry>[];
      final controller = HomeNavigationController(
        onConversationSelected: selected.add,
      );
      final conversation = _conversation('conversation-1');

      controller
        ..showSettings()
        ..locateMessage('message-1')
        ..select(conversation);

      expect(controller.selectedConversation, same(conversation));
      expect(controller.settingsSelected, isFalse);
      expect(controller.locateMessageId, isNull);
      expect(selected, [conversation]);
    },
  );

  test(
    'notification message intent increments request for repeated targets',
    () {
      final controller =
          HomeNavigationController(
              onConversationSelected: (_) {},
            )
            ..locateMessage('message-1')
            ..locateMessage('message-1');

      expect(controller.locateMessageId, 'message-1');
      expect(controller.locateRequest, 2);
    },
  );
}

ConversationListEntry _conversation(String id) => ConversationListEntry(
  id: id,
  ownerId: 'owner',
  name: 'Conversation',
  avatarUrl: '',
  category: 'CONTACT',
  draft: '',
  status: 0,
  content: '',
  contentType: null,
  messageStatus: null,
  senderId: null,
  senderName: null,
  updatedAt: DateTime(2026),
  unseenCount: 0,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: false,
  isPinned: false,
  relationship: '',
  identityNumber: '',
  circleIds: const [],
  groupAvatars: const [],
);
