import 'package:flutter/foundation.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';

@immutable
class MessageSearchState {
  const MessageSearchState({
    this.items = const [],
    this.loading = false,
    this.error,
  });

  final List<MessageListEntry> items;
  final bool loading;
  final Object? error;

  bool get initializing => items.isEmpty && loading;
}

class MessageSearchController extends ValueNotifier<MessageSearchState> {
  MessageSearchController({
    required this.account,
    required this.conversationId,
    required this.keyword,
    required this.senderId,
    required this.categories,
    this.limit = 60,
  }) : super(const MessageSearchState()) {
    itemPositionsListener.itemPositions.addListener(_onItemPosition);
    _load();
  }

  final rust.AccountHandle account;
  final String conversationId;
  final String keyword;
  final String? senderId;
  final List<String>? categories;
  final int limit;
  final ItemPositionsListener itemPositionsListener =
      ItemPositionsListener.create();

  var _hasMore = true;
  var _loading = false;
  var _disposed = false;
  var _generation = 0;

  Future<void> reload() async {
    if (_disposed) return;
    final generation = ++_generation;
    _hasMore = true;
    _loading = false;
    value = const MessageSearchState();
    await _load(generation: generation);
  }

  Future<void> _load({int? generation, bool append = false}) async {
    final requestGeneration = generation ?? _generation;
    if (_disposed || requestGeneration != _generation) return;
    if (_loading || !_hasMore) return;
    if (keyword.isEmpty) {
      _hasMore = false;
      return;
    }

    _loading = true;
    value = MessageSearchState(items: value.items, loading: true);
    try {
      final result = await account.message().searchMessages(
        conversationId: conversationId,
        query: keyword,
        senderId: senderId,
        categories: categories ?? const [],
        anchorMessageId: append && value.items.isNotEmpty
            ? value.items.last.id
            : null,
        limit: limit,
      );
      if (_disposed || requestGeneration != _generation) return;
      final items = result.map(MessageListEntry.fromRust).toList();
      _hasMore = items.length == limit;
      value = MessageSearchState(
        items: [...value.items, ...items],
      );
    } on Object catch (error, stackTrace) {
      if (_disposed || requestGeneration != _generation) return;
      _hasMore = false;
      e('Search conversation messages failed', error, stackTrace);
      value = MessageSearchState(items: value.items, error: error);
    } finally {
      if (!_disposed && requestGeneration == _generation) _loading = false;
    }
  }

  void _onItemPosition() {
    final positions = itemPositionsListener.itemPositions.value;
    if (positions.isEmpty || value.items.isEmpty) return;
    final lastIndex = positions
        .map((position) => position.index)
        .reduce((first, second) => first > second ? first : second);
    if (lastIndex >= value.items.length - 4) {
      _load(append: true);
    }
  }

  @override
  void dispose() {
    _disposed = true;
    itemPositionsListener.itemPositions.removeListener(_onItemPosition);
    super.dispose();
  }
}
