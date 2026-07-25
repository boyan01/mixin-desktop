import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

import '../models/command_palette_item.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';
import 'conversation_list_store.dart';

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
  ConversationListController(this.account) {
    _changeSubscription = account.conversationChanges().listen(
      _scheduleChanges,
      onError: (Object exception, StackTrace stackTrace) {
        e('Conversation change stream failed', exception, stackTrace);
      },
    );
    unawaited(_start());
  }

  final rust.AccountHandle account;
  final ConversationListStore _store = ConversationListStore();
  final ItemPositionsListener _itemPositionsListener =
      ItemPositionsListener.create();
  final ItemScrollController _itemScrollController = ItemScrollController();

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
  List<ConversationListEntry> _visibleConversations = const [];
  Timer? _searchTimer;
  Timer? _changeTimer;
  Timer? _reloadRetryTimer;
  int _searchRevision = 0;
  int _changeRetryAttempt = 0;
  int _reloadRetryAttempt = 0;
  final Set<String> _pendingConversationIds = {};
  bool _reloadAllPending = false;
  bool _flushingChanges = false;
  bool _initialized = false;
  final List<String> _recentConversationIds = [];
  var _disposed = false;

  bool get initialized => _initialized;
  ItemPositionsListener get itemPositionsListener => _itemPositionsListener;
  ItemScrollController get itemScrollController => _itemScrollController;

  List<ConversationListEntry> get visibleConversations => _visibleConversations;

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
    } on Object catch (error, stackTrace) {
      e('Search Mixin ID user failed: query=$query', error, stackTrace);
      if (_disposed || revision != _searchRevision) return;
      searchMaoUser = null;
      searchMao = null;
      notifyListeners();
    }
  }

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
      final conversations = await account.conversation().conversationItems();
      if (_disposed) return;
      final entries = conversations
          .map(ConversationListEntry.fromRust)
          .toList(growable: false);
      _store.replaceAll(entries);
      _initialized = true;
      _reloadRetryTimer?.cancel();
      _reloadRetryTimer = null;
      _reloadRetryAttempt = 0;
      _changeRetryAttempt = 0;
      i('Loaded conversation list: count=${entries.length}');
      _rebuildView();
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

  @override
  void dispose() {
    _disposed = true;
    _searchTimer?.cancel();
    _changeTimer?.cancel();
    _reloadRetryTimer?.cancel();
    unawaited(_changeSubscription?.cancel());
    super.dispose();
  }
}
