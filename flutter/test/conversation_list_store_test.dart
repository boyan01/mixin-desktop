import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/conversation_list_store.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';

void main() {
  test('filters the in-memory snapshot without querying another page', () {
    final store = ConversationListStore()
      ..replaceAll([
        _conversation(
          id: 'contact',
          name: 'Alice',
          relationship: 'FRIEND',
          unseenCount: 1,
        ),
        _conversation(id: 'group', name: 'Team', category: 'GROUP'),
        _conversation(id: 'bot', name: 'Weather', isBot: true),
      ]);

    expect(
      store
          .filtered((
            category: ConversationCategoryFilter.contacts,
            circleId: null,
            query: 'ali',
            unseenOnly: true,
          ))
          .map((item) => item.id),
      ['contact'],
    );
    expect(
      store
          .filtered((
            category: ConversationCategoryFilter.groups,
            circleId: null,
            query: '',
            unseenOnly: false,
          ))
          .map((item) => item.id),
      ['group'],
    );
  });

  test('applies changed and removed ids and keeps list ordering', () {
    final store = ConversationListStore()
      ..replaceAll([
        _conversation(id: 'older', name: 'Older', updatedAt: 1),
        _conversation(id: 'newer', name: 'Newer', updatedAt: 2),
      ]);

    store.applyChanges(
      ['older', 'newer'],
      [_conversation(id: 'older', name: 'Updated', updatedAt: 3)],
    );

    expect(store.items.map((item) => item.id), ['older']);
    expect(store.item('older')?.name, 'Updated');
    expect(store.item('newer'), isNull);
  });
}

ConversationListEntry _conversation({
  required String id,
  required String name,
  String category = 'CONTACT',
  String relationship = 'STRANGER',
  bool isBot = false,
  int unseenCount = 0,
  int updatedAt = 0,
}) => ConversationListEntry(
  id: id,
  ownerId: '$id-owner',
  name: name,
  avatarUrl: '',
  category: category,
  draft: '',
  status: 2,
  content: '',
  contentType: null,
  messageStatus: null,
  senderId: null,
  senderName: null,
  updatedAt: DateTime.fromMillisecondsSinceEpoch(updatedAt),
  unseenCount: unseenCount,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: isBot,
  isPinned: false,
  relationship: relationship,
  identityNumber: '',
  circleIds: const [],
  groupAvatars: const [],
);
