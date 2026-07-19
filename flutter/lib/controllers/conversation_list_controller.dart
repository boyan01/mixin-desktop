import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/controllers/paging_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/command_palette_item.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/widgets/message_selectable_text.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

const _pageLimit = 15;

typedef _PageKey = ({
  ConversationCategoryFilter category,
  String? circleId,
  String query,
  bool unseenOnly,
});

String? _completeMao(String value) {
  final text = value.trim();
  final candidate = text.replaceFirst(RegExp(r'\.$'), '');
  if (candidate.isEmpty ||
      candidate.runes.length > 128 ||
      candidate.runes.every((rune) => rune >= 48 && rune <= 57) ||
      RegExp(r'[\sA-Z]').hasMatch(candidate)) {
    return null;
  }
  if (text.endsWith('.mao')) return text;
  if (text.endsWith('.ma')) return '${text}o';
  if (text.endsWith('.m')) return '${text}ao';
  if (text.endsWith('.')) return '${text}mao';
  return '$text.mao';
}

class ConversationListController extends ChangeNotifier {
  ConversationListController(this.account) : profile = account.profile() {
    _activatePage();
    unawaited(_refreshMetadata());
    _profileSubscription = account.profileChanges().listen((value) {
      if (_disposed || profile == value) return;
      profile = value;
      notifyListeners();
    }, onError: _setError);
    _changeSubscription = account.conversationChanges().listen((_) {
      _activePage.update();
      unawaited(_refreshMetadata());
    }, onError: _setError);
  }

  final rust.AccountHandle account;
  rust.AccountProfile profile;
  final Map<_PageKey, PagingController<ConversationListEntry>> _pages = {};
  final Map<String, int> _unseenCounts = {};
  final Map<String, int> _mutedUnseenCounts = {};

  List<rust.CircleItem> circles = const [];
  ConversationCategoryFilter category = ConversationCategoryFilter.chats;
  String? circleId;
  String query = '';
  bool filterUnseen = false;
  bool loading = true;
  bool searchMessagesLoading = false;
  List<rust.UserProfileItem> searchUsers = const [];
  rust.UserProfileItem? searchMaoUser;
  String? searchMao;
  List<MessageListEntry> searchMessages = const [];
  Map<String, ConversationListEntry> searchMessageConversations = const {};
  Map<String, String> mentionNames = const {};
  String? error;

  StreamSubscription<BigInt>? _changeSubscription;
  StreamSubscription<rust.AccountProfile>? _profileSubscription;
  PagingController<ConversationListEntry>? _currentPage;
  VoidCallback? _currentPageListener;
  PagingController<ConversationListEntry>? _transientPage;
  _PageKey? _transientPageKey;
  Timer? _searchTimer;
  int _searchRevision = 0;
  List<ConversationListEntry>? _allChats;
  final List<String> _recentConversationIds = [];
  var _disposed = false;

  PagingController<ConversationListEntry> get _activePage => _currentPage!;
  PagingState<ConversationListEntry> get pagingState => _activePage.value;
  ItemPositionsListener get itemPositionsListener =>
      _activePage.itemPositionsListener;
  ItemScrollController get itemScrollController =>
      _activePage.itemScrollController;

  List<ConversationListEntry> get visibleConversations {
    final entries = _activePage.value.map.entries.toList()
      ..sort((left, right) => left.key.compareTo(right.key));
    return entries
        .map((entry) => entry.value)
        .whereType<ConversationListEntry>()
        .toList(growable: false);
  }

  int countFor(ConversationCategoryFilter filter, {String? circle}) =>
      _unseenCounts[filter == ConversationCategoryFilter.circle
          ? 'circle:$circle'
          : filter.name] ??
      0;

  int mutedCountFor(ConversationCategoryFilter filter, {String? circle}) =>
      _mutedUnseenCounts[filter == ConversationCategoryFilter.circle
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
    _scheduleMessageSearch();
  }

  void toggleUnseen() {
    filterUnseen = !filterUnseen;
    _activatePage();
    _scheduleMessageSearch();
  }

  Future<void> refresh() async {
    _allChats = null;
    _activePage.update();
    await _refreshMetadata();
  }

  Future<ConversationListEntry?> findConversation(String conversationId) async {
    for (final page in _pages.values) {
      for (final item in page.value.map.values) {
        if (item.id == conversationId) return item;
      }
    }
    final chats = _allChats ??= await _allConversations(
      ConversationCategoryFilter.chats,
    );
    for (final item in chats) {
      if (item.id == conversationId) return item;
    }
    return null;
  }

  void recordRecentConversation(String conversationId) {
    _recentConversationIds
      ..remove(conversationId)
      ..add(conversationId);
    if (_recentConversationIds.length > 5) _recentConversationIds.removeAt(0);
  }

  Future<List<CommandPaletteItem>> searchCommandPalette(String keyword) async {
    final normalized = keyword.trim();
    if (normalized.isEmpty) {
      final conversations = _allChats ??= await _allConversations(
        ConversationCategoryFilter.chats,
      );
      final byId = {for (final item in conversations) item.id: item};
      return _recentConversationIds.reversed
          .map((id) => byId[id])
          .whereType<ConversationListEntry>()
          .map((item) => CommandPaletteItem.conversation(item, normalized))
          .toList(growable: false);
    }
    final results = await Future.wait<dynamic>([
      account.conversation().conversations(
        category: ConversationCategoryFilter.chats.name,
        circleId: null,
        keyword: normalized,
        unseenOnly: false,
        limit: 100,
        offset: 0,
      ),
      account.user().searchLocalUsers(
        query: normalized,
        category: ConversationCategoryFilter.chats.name,
        limit: 100,
      ),
    ]);
    final items = <CommandPaletteItem>[
      ...(results[0] as List<rust.ConversationListItem>).map(
        (item) => CommandPaletteItem.conversation(
          ConversationListEntry.fromRust(item),
          normalized,
        ),
      ),
      ...(results[1] as List<rust.UserProfileItem>).map(
        (item) => CommandPaletteItem.user(item, normalized),
      ),
    ];
    items.sort((left, right) => right.matchScore.compareTo(left.matchScore));
    return items;
  }

  void _scheduleMessageSearch() {
    _searchTimer?.cancel();
    final normalized = query.trim();
    final revision = ++_searchRevision;
    if (normalized.isEmpty || filterUnseen) {
      searchMessagesLoading = false;
      searchMessages = const [];
      searchUsers = const [];
      searchMaoUser = null;
      searchMao = null;
      searchMessageConversations = const {};
      notifyListeners();
      return;
    }
    searchMessagesLoading = true;
    searchMaoUser = null;
    searchMao = null;
    notifyListeners();
    _searchTimer = Timer(const Duration(milliseconds: 150), () async {
      unawaited(_searchMaoUser(normalized, revision));
      try {
        final results = await Future.wait<dynamic>([
          account.message().searchGlobalMessages(
            query: normalized,
            offset: 0,
            limit: 32,
          ),
          account.user().searchLocalUsers(
            query: normalized,
            category: category.name,
            limit: 64,
          ),
          _allChats == null
              ? _allConversations(ConversationCategoryFilter.chats).then((
                  items,
                ) {
                  _allChats = items;
                  return const <rust.MessageListItem>[];
                })
              : Future.value(const <rust.MessageListItem>[]),
        ]);
        if (_disposed || revision != _searchRevision) return;
        final messages = (results[0] as List<rust.MessageListItem>)
            .map(MessageListEntry.fromRust)
            .toList(growable: false);
        await _cacheMentionNames(
          messages.expand((message) => [message.content, message.caption]),
        );
        if (_disposed || revision != _searchRevision) return;
        searchMessages = messages;
        searchUsers = results[1] as List<rust.UserProfileItem>;
        searchMessageConversations = {
          for (final item in _allChats ?? const <ConversationListEntry>[])
            item.id: item,
        };
        searchMessagesLoading = false;
        notifyListeners();
      } on Object catch (exception) {
        if (_disposed || revision != _searchRevision) return;
        searchMessages = const [];
        searchUsers = const [];
        searchMessageConversations = const {};
        searchMessagesLoading = false;
        _setError(exception);
      }
    });
  }

  Future<void> _searchMaoUser(String query, int revision) async {
    final mao = _completeMao(query);
    if (mao == null) return;
    try {
      final user = await account.user().searchMaoUser(query: query);
      if (_disposed || revision != _searchRevision) return;
      searchMaoUser = user;
      searchMao = user == null ? null : mao;
      notifyListeners();
    } on Object {
      if (_disposed || revision != _searchRevision) return;
      searchMaoUser = null;
      searchMao = null;
      notifyListeners();
    }
  }

  Future<void> setPinned(ConversationListEntry item) => account
      .conversation()
      .setPinned(conversationId: item.id, pinned: !item.isPinned);

  Future<void> setMuted(ConversationListEntry item, int durationSeconds) =>
      account.conversation().setMuted(
        conversationId: item.id,
        ownerId: item.ownerId,
        category: item.category,
        durationSeconds: durationSeconds,
      );

  Future<void> deleteConversation(ConversationListEntry item) =>
      account.conversation().deleteConversation(conversationId: item.id);

  Future<void> editCircle(
    ConversationListEntry item,
    String circleId,
    bool add,
  ) => account.conversation().editCircleConversation(
    circleId: circleId,
    conversationId: item.id,
    ownerId: item.ownerId,
    isGroup: item.isGroup,
    add: add,
  );

  Future<rust.CircleItem> createCircle(String name) async {
    final normalized = name.trim();
    if (normalized.isEmpty) throw ArgumentError.value(name, 'name');
    final circle = await account.conversation().createCircle(name: normalized);
    await _refreshMetadata();
    return circle;
  }

  Future<void> updateCircle(String circleId, String name) async {
    await account.conversation().updateCircle(
      circleId: circleId,
      name: name.trim(),
    );
    await _refreshMetadata();
  }

  Future<void> deleteCircle(String circleId) async {
    await account.conversation().deleteCircle(circleId: circleId);
    if (category == ConversationCategoryFilter.circle &&
        this.circleId == circleId) {
      selectCategory(ConversationCategoryFilter.chats);
    }
    await _refreshMetadata();
  }

  Future<void> replaceCircleConversations(
    String circleId,
    List<ConversationListEntry> selected,
  ) async {
    final existing = await _allConversations(
      ConversationCategoryFilter.chats,
      circleId: circleId,
    );
    final existingById = {for (final item in existing) item.id: item};
    final selectedById = {for (final item in selected) item.id: item};
    for (final item in selectedById.values) {
      if (!existingById.containsKey(item.id)) {
        await editCircle(item, circleId, true);
      }
    }
    for (final item in existingById.values) {
      if (!selectedById.containsKey(item.id)) {
        await editCircle(item, circleId, false);
      }
    }
    await refresh();
  }

  Future<void> reorderCircles(int oldIndex, int newIndex) async {
    if (oldIndex == newIndex) return;
    final previous = circles;
    final reordered = [...circles];
    final circle = reordered.removeAt(oldIndex);
    reordered.insert(newIndex, circle);
    circles = reordered;
    notifyListeners();
    try {
      await account.conversation().reorderCircles(
        circleIds: reordered.map((item) => item.circleId).toList(),
      );
    } on Object {
      circles = previous;
      notifyListeners();
      rethrow;
    }
  }

  Future<void> createGroup(String name, List<String> userIds) async {
    await account.conversation().createGroup(
      name: name.trim(),
      userIds: userIds,
    );
    await refresh();
  }

  Future<void> updateProfile(String fullName, String biography) async {
    await account.updateProfile(
      fullName: fullName.trim(),
      biography: biography.trim(),
    );
  }

  Future<rust.AccountProfile> refreshProfile() => account.refreshProfile();

  Future<List<ConversationListEntry>> _allConversations(
    ConversationCategoryFilter category, {
    String? circleId,
  }) async {
    final count = await account.conversation().conversationCount(
      category: category.name,
      circleId: circleId,
      keyword: '',
      unseenOnly: false,
    );
    final result = <ConversationListEntry>[];
    while (result.length < count.toInt()) {
      final page = await account.conversation().conversations(
        category: category.name,
        circleId: circleId,
        keyword: '',
        unseenOnly: false,
        limit: 200,
        offset: result.length,
      );
      result.addAll(page.map(ConversationListEntry.fromRust));
      if (page.length < 200) break;
    }
    return result;
  }

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
      final result = await account.conversation().conversationCount(
        category: key.category.name,
        circleId: key.circleId,
        keyword: key.query,
        unseenOnly: key.unseenOnly,
      );
      error = null;
      return result.toInt();
    } catch (exception, stackTrace) {
      _setError(exception, stackTrace);
      return 0;
    }
  }

  Future<List<ConversationListEntry>> _queryRange(
    _PageKey key,
    int limit,
    int offset,
  ) async {
    try {
      final result = await account.conversation().conversations(
        category: key.category.name,
        circleId: key.circleId,
        keyword: key.query,
        unseenOnly: key.unseenOnly,
        limit: limit,
        offset: offset,
      );
      await _cacheMentionNames(
        result.map((conversation) => conversation.lastMessage),
      );
      error = null;
      return result.map(ConversationListEntry.fromRust).toList(growable: false);
    } catch (exception, stackTrace) {
      _setError(exception, stackTrace);
      return const [];
    }
  }

  Future<void> _cacheMentionNames(Iterable<String?> texts) async {
    final identityNumbers = messageMentionIdentityNumbers(texts)
        .where((identityNumber) => !mentionNames.containsKey(identityNumber))
        .toList(growable: false);
    if (identityNumbers.isEmpty) return;
    final users = await account.user().usersByIdentityNumbers(
      identityNumbers: identityNumbers,
    );
    if (_disposed) return;
    mentionNames = Map.unmodifiable({
      ...mentionNames,
      for (final user in users)
        if (user.fullName.trim().isNotEmpty)
          user.identityNumber: user.fullName.trim(),
    });
  }

  Future<void> _refreshMetadata() async {
    try {
      final updatedCircles = await account.conversation().circles();
      final conversations = await _allConversations(
        ConversationCategoryFilter.chats,
      );
      if (_disposed) return;
      circles = updatedCircles;
      _allChats = conversations;
      _unseenCounts
        ..clear()
        ..addAll(_categoryCounts(conversations, muted: false));
      _mutedUnseenCounts
        ..clear()
        ..addAll(_categoryCounts(conversations, muted: true));
      error = null;
      notifyListeners();
    } catch (exception, stackTrace) {
      _setError(exception, stackTrace);
    }
  }

  Map<String, int> _categoryCounts(
    List<ConversationListEntry> conversations, {
    required bool muted,
  }) {
    final result = <String, int>{};
    for (final conversation in conversations) {
      if (conversation.unseenCount <= 0 || (muted && !conversation.isMuted)) {
        continue;
      }
      void increment(String key) =>
          result.update(key, (count) => count + 1, ifAbsent: () => 1);

      if (conversation.isGroup) {
        increment(ConversationCategoryFilter.groups.name);
      } else if (conversation.isBot) {
        increment(ConversationCategoryFilter.bots.name);
      } else if (conversation.relationship == 'FRIEND') {
        increment(ConversationCategoryFilter.contacts.name);
      } else {
        increment(ConversationCategoryFilter.strangers.name);
      }
      for (final circleId in conversation.circleIds) {
        increment('circle:$circleId');
      }
    }
    return result;
  }

  void _setError(Object exception, [StackTrace? stackTrace]) {
    if (_disposed) return;
    e('Conversation list failed', exception, stackTrace);
    loading = false;
    error = exception.toString();
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _searchTimer?.cancel();
    unawaited(_changeSubscription?.cancel());
    unawaited(_profileSubscription?.cancel());
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
