import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/controllers/paging_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

const _pageLimit = 15;

typedef _PageKey = ({
  ConversationCategoryFilter category,
  String? circleId,
  String query,
  bool unseenOnly,
});

class ConversationListController extends ChangeNotifier {
  ConversationListController(this.account) : profile = account.profile() {
    _activatePage();
    unawaited(_refreshMetadata());
    _changeSubscription = account.conversationChanges().listen((_) {
      _activePage.update();
      unawaited(_refreshMetadata());
    }, onError: _setError);
  }

  final rust.AccountHandle account;
  final rust.AccountProfile profile;
  final Map<_PageKey, PagingController<ConversationListEntry>> _pages = {};
  final Map<String, int> _unseenCounts = {};

  List<rust.CircleItem> circles = const [];
  ConversationCategoryFilter category = ConversationCategoryFilter.chats;
  String? circleId;
  String query = '';
  bool filterUnseen = false;
  bool loading = true;
  String? error;

  StreamSubscription<BigInt>? _changeSubscription;
  PagingController<ConversationListEntry>? _currentPage;
  VoidCallback? _currentPageListener;
  PagingController<ConversationListEntry>? _transientPage;
  _PageKey? _transientPageKey;
  var _disposed = false;

  PagingController<ConversationListEntry> get _activePage => _currentPage!;
  PagingState<ConversationListEntry> get pagingState => _activePage.value;
  ItemPositionsListener get itemPositionsListener =>
      _activePage.itemPositionsListener;
  ItemScrollController get itemScrollController =>
      _activePage.itemScrollController;

  int countFor(ConversationCategoryFilter filter, {String? circle}) =>
      _unseenCounts[filter == ConversationCategoryFilter.circle
          ? 'circle:$circle'
          : filter.name] ??
      0;

  void selectCategory(ConversationCategoryFilter value, {String? circle}) {
    category = value;
    circleId = circle;
    _activatePage();
  }

  void setQuery(String value) {
    if (query == value) return;
    query = value;
    _activatePage();
  }

  void toggleUnseen() {
    filterUnseen = !filterUnseen;
    _activatePage();
  }

  Future<void> refresh() async {
    _activePage.update();
    await _refreshMetadata();
  }

  Future<void> setPinned(ConversationListEntry item) => account
      .setConversationPinned(conversationId: item.id, pinned: !item.isPinned);

  Future<void> setMuted(ConversationListEntry item, int durationSeconds) =>
      account.setConversationMuted(
        conversationId: item.id,
        ownerId: item.ownerId,
        category: item.category,
        durationSeconds: durationSeconds,
      );

  Future<void> deleteConversation(ConversationListEntry item) =>
      account.deleteConversation(conversationId: item.id);

  Future<void> editCircle(
    ConversationListEntry item,
    String circleId,
    bool add,
  ) => account.editCircleConversation(
    circleId: circleId,
    conversationId: item.id,
    ownerId: item.ownerId,
    isGroup: item.isGroup,
    add: add,
  );

  void _activatePage() {
    final key = (
      category: category,
      circleId: circleId,
      query: query.trim(),
      unseenOnly: filterUnseen,
    );
    if (_currentPage != null && _currentPageListener != null) {
      _currentPage!.removeListener(_currentPageListener!);
    }

    PagingController<ConversationListEntry> createPage() => PagingController(
      limit: _pageLimit,
      queryCount: () => _queryCount(key),
      queryRange: (limit, offset) => _queryRange(key, limit, offset),
    );

    final cacheable = key.query.isEmpty && !key.unseenOnly;
    late final PagingController<ConversationListEntry> page;
    if (cacheable) {
      _transientPage?.dispose();
      _transientPage = null;
      _transientPageKey = null;
      page = _pages.putIfAbsent(key, createPage);
    } else {
      if (_transientPageKey != key) {
        _transientPage?.dispose();
        _transientPage = createPage();
        _transientPageKey = key;
      }
      page = _transientPage!;
    }
    _currentPage = page;
    _currentPageListener = () {
      loading = !page.value.initialized;
      if (!_disposed) notifyListeners();
    };
    page.addListener(_currentPageListener!);
    loading = !page.value.initialized;
    if (!_disposed) notifyListeners();
  }

  Future<int> _queryCount(_PageKey key) async {
    try {
      final result = await account.conversationCount(
        category: key.category.name,
        circleId: key.circleId,
        keyword: key.query,
        unseenOnly: key.unseenOnly,
      );
      error = null;
      return result.toInt();
    } catch (exception) {
      _setError(exception);
      return 0;
    }
  }

  Future<List<ConversationListEntry>> _queryRange(
    _PageKey key,
    int limit,
    int offset,
  ) async {
    try {
      final result = await account.conversations(
        category: key.category.name,
        circleId: key.circleId,
        keyword: key.query,
        unseenOnly: key.unseenOnly,
        limit: limit,
        offset: offset,
      );
      error = null;
      return result.map(ConversationListEntry.fromRust).toList(growable: false);
    } catch (exception) {
      _setError(exception);
      return const [];
    }
  }

  Future<void> _refreshMetadata() async {
    try {
      final updatedCircles = await account.circles();
      final requests = <Future<(String, int)>>[
        for (final filter in ConversationCategoryFilter.values)
          if (filter != ConversationCategoryFilter.chats &&
              filter != ConversationCategoryFilter.circle)
            account
                .conversationCount(
                  category: filter.name,
                  circleId: null,
                  keyword: '',
                  unseenOnly: true,
                )
                .then((count) => (filter.name, count.toInt())),
        for (final circle in updatedCircles)
          account
              .conversationCount(
                category: ConversationCategoryFilter.circle.name,
                circleId: circle.circleId,
                keyword: '',
                unseenOnly: true,
              )
              .then((count) => ('circle:${circle.circleId}', count.toInt())),
      ];
      final counts = await Future.wait(requests);
      if (_disposed) return;
      circles = updatedCircles;
      _unseenCounts
        ..clear()
        ..addEntries(counts.map((count) => MapEntry(count.$1, count.$2)));
      error = null;
      notifyListeners();
    } catch (exception) {
      _setError(exception);
    }
  }

  void _setError(Object exception) {
    if (_disposed) return;
    loading = false;
    error = exception.toString();
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    unawaited(_changeSubscription?.cancel());
    if (_currentPage != null && _currentPageListener != null) {
      _currentPage!.removeListener(_currentPageListener!);
    }
    for (final page in _pages.values) {
      page.dispose();
    }
    _transientPage?.dispose();
    super.dispose();
  }
}
