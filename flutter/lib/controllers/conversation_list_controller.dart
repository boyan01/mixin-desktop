import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

import '../models/command_palette_item.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';
import 'conversation_list_store.dart';

const _unseenCountThrottle = Duration(milliseconds: 333);
const _changeMergeWindow = Duration(milliseconds: 16);
const _retryDelays = [
  Duration(seconds: 1),
  Duration(seconds: 2),
  Duration(seconds: 4),
  Duration(seconds: 8),
  Duration(seconds: 16),
  Duration(seconds: 30),
];

Duration _retryDelay(int attempt) =>
    _retryDelays[attempt < _retryDelays.length
        ? attempt
        : _retryDelays.length - 1];

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
    _profileSubscription = account.profileChanges().listen(
      (value) {
        if (_disposed || profile == value) return;
        profile = value;
        notifyListeners();
      },
      onError: (Object exception, StackTrace stackTrace) {
        e('Profile change stream failed', exception, stackTrace);
      },
    );
    _changeSubscription = account.conversationChanges().listen(
      _scheduleChanges,
      onError: (Object exception, StackTrace stackTrace) {
        e('Conversation change stream failed', exception, stackTrace);
      },
    );
    unawaited(_start());
  }

  final rust.AccountHandle account;
  rust.AccountProfile profile;
  final ConversationListStore _store = ConversationListStore();
  final Map<String, int> _unseenCounts = {};
  final Map<String, int> _mutedUnseenCounts = {};
  final ItemPositionsListener _itemPositionsListener =
      ItemPositionsListener.create();
  final ItemScrollController _itemScrollController = ItemScrollController();

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

  StreamSubscription<rust.ConversationChangeEvent>? _changeSubscription;
  StreamSubscription<rust.AccountProfile>? _profileSubscription;
  List<ConversationListEntry> _visibleConversations = const [];
  Timer? _searchTimer;
  Timer? _changeTimer;
  Timer? _reloadRetryTimer;
  Timer? _unseenCountTimer;
  int _searchRevision = 0;
  int _changeRetryAttempt = 0;
  int _reloadRetryAttempt = 0;
  final Set<String> _pendingConversationIds = {};
  bool _reloadAllPending = false;
  bool _flushingChanges = false;
  bool _unseenCountRefreshPending = false;
  bool _refreshingUnseenCounts = false;
  bool _initialized = false;
  final List<String> _recentConversationIds = [];
  var _disposed = false;

  bool get initialized => _initialized;
  ItemPositionsListener get itemPositionsListener => _itemPositionsListener;
  ItemScrollController get itemScrollController => _itemScrollController;

  List<ConversationListEntry> get visibleConversations => _visibleConversations;

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
    _rebuildView();
  }

  void setQuery(String value) {
    if (query == value) return;
    query = value;
    _rebuildView();
    _scheduleMessageSearch();
  }

  void toggleUnseen() {
    filterUnseen = !filterUnseen;
    _rebuildView();
    _scheduleMessageSearch();
  }

  Future<void> refresh() async {
    await _reloadAll();
  }

  Future<ConversationListEntry?> findConversation(
    String conversationId,
  ) async => _store.item(conversationId);

  void recordRecentConversation(String conversationId) {
    _recentConversationIds
      ..remove(conversationId)
      ..add(conversationId);
    if (_recentConversationIds.length > 5) _recentConversationIds.removeAt(0);
  }

  Future<List<CommandPaletteItem>> searchCommandPalette(String keyword) async {
    final normalized = keyword.trim();
    if (normalized.isEmpty) {
      return _recentConversationIds.reversed
          .map(_store.item)
          .whereType<ConversationListEntry>()
          .map((item) => CommandPaletteItem.conversation(item, normalized))
          .toList(growable: false);
    }
    final conversations = _store.filtered((
      category: ConversationCategoryFilter.chats,
      circleId: null,
      query: normalized,
      unseenOnly: false,
    ));
    final users = await account.user().searchLocalUsers(
      query: normalized,
      category: ConversationCategoryFilter.chats.name,
      limit: 100,
    );
    return <CommandPaletteItem>[
      ...conversations.map(
        (item) => CommandPaletteItem.conversation(item, normalized),
      ),
      ...users.map((item) => CommandPaletteItem.user(item, normalized)),
    ]..sort((left, right) => right.matchScore.compareTo(left.matchScore));
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
            limit: 32,
          ),
          account.user().searchLocalUsers(
            query: normalized,
            category: category.name,
            limit: 64,
          ),
        ]);
        if (_disposed || revision != _searchRevision) return;
        final messages = (results[0] as List<rust.MessageListItem>)
            .map(MessageListEntry.fromRust)
            .toList(growable: false);
        if (_disposed || revision != _searchRevision) return;
        searchMessages = messages;
        searchUsers = results[1] as List<rust.UserProfileItem>;
        searchMessageConversations = {
          for (final item in _store.items) item.id: item,
        };
        searchMessagesLoading = false;
        notifyListeners();
      } catch (exception, stackTrace) {
        if (_disposed || revision != _searchRevision) return;
        searchMessages = const [];
        searchUsers = const [];
        searchMessageConversations = const {};
        searchMessagesLoading = false;
        e(
          'Search conversations failed: query=$normalized',
          exception,
          stackTrace,
        );
        notifyListeners();
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
    await _refreshCircles();
    return circle;
  }

  Future<void> updateCircle(String circleId, String name) async {
    await account.conversation().updateCircle(
      circleId: circleId,
      name: name.trim(),
    );
    await _refreshCircles();
  }

  Future<void> deleteCircle(String circleId) async {
    await account.conversation().deleteCircle(circleId: circleId);
    if (category == ConversationCategoryFilter.circle &&
        this.circleId == circleId) {
      selectCategory(ConversationCategoryFilter.chats);
    }
    await _refreshCircles();
  }

  Future<void> replaceCircleConversations(
    String circleId,
    List<ConversationListEntry> selected,
  ) async {
    final existing = _store.filtered((
      category: ConversationCategoryFilter.circle,
      circleId: circleId,
      query: '',
      unseenOnly: false,
    ));
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

  ConversationListFilter get _filter => (
    category: category,
    circleId: circleId,
    query: query.trim(),
    unseenOnly: filterUnseen,
  );

  void _rebuildView() {
    _visibleConversations = _store.filtered(_filter);
    loading = !_initialized;
    if (!_disposed) notifyListeners();
  }

  Future<void> _start() async {
    await _reloadAll();
    if (_pendingConversationIds.isNotEmpty || _reloadAllPending) {
      _scheduleChangeFlush();
    }
  }

  Future<void> _reloadAll() async {
    try {
      final results = await Future.wait<dynamic>([
        account.conversation().conversationItems(),
        account.conversation().circles(),
      ]);
      if (_disposed) return;
      final conversations = (results[0] as List<rust.ConversationListItem>)
          .map(ConversationListEntry.fromRust)
          .toList(growable: false);
      _store.replaceAll(conversations);
      circles = results[1] as List<rust.CircleItem>;
      _initialized = true;
      _reloadRetryTimer?.cancel();
      _reloadRetryTimer = null;
      _reloadRetryAttempt = 0;
      _changeRetryAttempt = 0;
      i('Loaded conversation list: count=${conversations.length}');
      _rebuildView();
      _scheduleUnseenCountRefresh(immediate: true);
    } catch (exception, stackTrace) {
      if (_disposed) return;
      e('Load conversation list failed', exception, stackTrace);
      loading = !_initialized;
      notifyListeners();
      _scheduleReloadRetry();
    }
  }

  void _scheduleReloadRetry() {
    if (_disposed || _reloadRetryTimer != null) return;
    final delay = _retryDelay(_reloadRetryAttempt++);
    w('Retry conversation list load in ${delay.inSeconds}s');
    _reloadRetryTimer = Timer(delay, () {
      _reloadRetryTimer = null;
      unawaited(_start());
    });
  }

  void _scheduleChanges(rust.ConversationChangeEvent event) {
    if (_disposed) return;
    _reloadAllPending |= event.reloadAll;
    _pendingConversationIds.addAll(event.conversationIds);
    _scheduleChangeFlush();
  }

  void _scheduleChangeFlush({Duration delay = _changeMergeWindow}) {
    if (!_initialized || _flushingChanges || _changeTimer != null) return;
    _changeTimer = Timer(delay, () {
      _changeTimer = null;
      unawaited(_flushChanges());
    });
  }

  Future<void> _flushChanges() async {
    _flushingChanges = true;
    Duration? retryDelay;
    final reloadAll = _reloadAllPending;
    final ids = _pendingConversationIds.toList(growable: false);
    _reloadAllPending = false;
    _pendingConversationIds.clear();
    try {
      if (reloadAll) {
        await _reloadAll();
      } else if (ids.isNotEmpty) {
        final changed = await account.conversation().conversationItemsByIds(
          conversationIds: ids,
        );
        if (_disposed) return;
        final entries = changed
            .map(ConversationListEntry.fromRust)
            .toList(growable: false);
        _store.applyChanges(ids, entries);
        _rebuildView();
        _scheduleUnseenCountRefresh();
        _changeRetryAttempt = 0;
      }
    } catch (exception, stackTrace) {
      if (!_disposed) {
        _pendingConversationIds.addAll(ids);
        _reloadAllPending |= reloadAll;
        retryDelay = _retryDelay(_changeRetryAttempt++);
        e(
          'Apply conversation changes failed: reload_all=$reloadAll ids=$ids '
          'retry_in=${retryDelay.inSeconds}s',
          exception,
          stackTrace,
        );
      }
    } finally {
      _flushingChanges = false;
      if (_reloadAllPending || _pendingConversationIds.isNotEmpty) {
        _scheduleChangeFlush(delay: retryDelay ?? _changeMergeWindow);
      }
    }
  }

  Future<void> _refreshCircles() async {
    try {
      circles = await account.conversation().circles();
      if (!_disposed) notifyListeners();
    } catch (exception, stackTrace) {
      if (!_disposed) {
        e('Refresh conversation circles failed', exception, stackTrace);
      }
    }
  }

  void _scheduleUnseenCountRefresh({bool immediate = false}) {
    if (_disposed) return;
    _unseenCountRefreshPending = true;
    if (_refreshingUnseenCounts || _unseenCountTimer != null) return;
    _unseenCountTimer = Timer(
      immediate ? Duration.zero : _unseenCountThrottle,
      () {
        _unseenCountTimer = null;
        unawaited(_refreshUnseenCounts());
      },
    );
  }

  Future<void> _refreshUnseenCounts() async {
    _refreshingUnseenCounts = true;
    _unseenCountRefreshPending = false;
    try {
      final counts = await account.conversation().unseenConversationCounts();
      if (_disposed) return;
      _unseenCounts.clear();
      _mutedUnseenCounts.clear();
      for (final item in counts) {
        final key = item.category == ConversationCategoryFilter.circle.name
            ? 'circle:${item.circleId}'
            : item.category;
        _unseenCounts[key] = item.count;
        _mutedUnseenCounts[key] = item.mutedCount;
      }
      notifyListeners();
    } catch (exception, stackTrace) {
      if (!_disposed) {
        e('Refresh unseen conversation counts failed', exception, stackTrace);
      }
    } finally {
      _refreshingUnseenCounts = false;
      if (_unseenCountRefreshPending) _scheduleUnseenCountRefresh();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _searchTimer?.cancel();
    _changeTimer?.cancel();
    _reloadRetryTimer?.cancel();
    _unseenCountTimer?.cancel();
    unawaited(_changeSubscription?.cancel());
    unawaited(_profileSubscription?.cancel());
    super.dispose();
  }
}
