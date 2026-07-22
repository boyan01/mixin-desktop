import '../models/conversation_list_entry.dart';

typedef ConversationListFilter = ({
  ConversationCategoryFilter category,
  String? circleId,
  String query,
  bool unseenOnly,
});

class ConversationListStore {
  final Map<String, ConversationListEntry> _items = {};
  final List<ConversationListEntry> _sortedItems = [];

  List<ConversationListEntry> get items => List.unmodifiable(_sortedItems);

  ConversationListEntry? item(String conversationId) => _items[conversationId];

  void replaceAll(Iterable<ConversationListEntry> items) {
    _items
      ..clear()
      ..addEntries(items.map((item) => MapEntry(item.id, item)));
    _resort();
  }

  void applyChanges(
    Iterable<String> requestedIds,
    Iterable<ConversationListEntry> changedItems,
  ) {
    final changedById = {for (final item in changedItems) item.id: item};
    for (final id in requestedIds) {
      final item = changedById[id];
      if (item == null) {
        _items.remove(id);
      } else {
        _items[id] = item;
      }
    }
    _resort();
  }

  List<ConversationListEntry> filtered(ConversationListFilter filter) {
    final query = filter.query.trim().toLowerCase();
    return _sortedItems
        .where((item) {
          if (filter.unseenOnly && item.unseenCount <= 0) return false;
          if (!_matchesCategory(item, filter.category, filter.circleId)) {
            return false;
          }
          if (query.isEmpty) return true;
          return item.name.toLowerCase().contains(query) ||
              item.identityNumber.toLowerCase().contains(query) ||
              item.content.toLowerCase().contains(query);
        })
        .toList(growable: false);
  }

  bool _matchesCategory(
    ConversationListEntry item,
    ConversationCategoryFilter category,
    String? circleId,
  ) => switch (category) {
    ConversationCategoryFilter.chats => true,
    ConversationCategoryFilter.contacts =>
      !item.isGroup && !item.isBot && item.relationship == 'FRIEND',
    ConversationCategoryFilter.groups => item.isGroup,
    ConversationCategoryFilter.bots => !item.isGroup && item.isBot,
    ConversationCategoryFilter.strangers =>
      !item.isGroup && !item.isBot && item.relationship == 'STRANGER',
    ConversationCategoryFilter.circle =>
      circleId != null && item.circleIds.contains(circleId),
  };

  void _resort() {
    _sortedItems
      ..clear()
      ..addAll(_items.values)
      ..sort(_compare);
  }

  int _compare(ConversationListEntry left, ConversationListEntry right) {
    final pinCompare = _milliseconds(
      right.pinTime,
    ).compareTo(_milliseconds(left.pinTime));
    if (pinCompare != 0) return pinCompare;
    final draftCompare = _hasActiveDraft(
      right,
    ).compareTo(_hasActiveDraft(left));
    if (draftCompare != 0) return draftCompare;
    final updatedCompare = right.updatedAt.compareTo(left.updatedAt);
    if (updatedCompare != 0) return updatedCompare;
    return left.id.compareTo(right.id);
  }

  int _milliseconds(DateTime? value) => value?.millisecondsSinceEpoch ?? 0;

  int _hasActiveDraft(ConversationListEntry item) =>
      item.status != 3 && item.draft.isNotEmpty ? 1 : 0;
}
