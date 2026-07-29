import 'dart:async';

import 'package:flutter/foundation.dart';

import '../models/command_palette_item.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';
import 'conversation_filter_controller.dart';
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

class ConversationListController extends ChangeNotifier {
  ConversationListController(this.account, this.filter) {
    filter.addListener(_rebuildView);
    _changeSubscription = account.conversationChanges().listen(
      _scheduleChanges,
      onError: (Object exception, StackTrace stackTrace) {
        e('Conversation change stream failed', exception, stackTrace);
      },
    );
    unawaited(_start());
  }

  final rust.AccountHandle account;
  final ConversationFilterController filter;
  final ConversationListStore _store = ConversationListStore();

  bool loading = true;

  StreamSubscription<rust.ConversationChangeEvent>? _changeSubscription;
  List<ConversationListEntry> _visibleConversations = const [];
  List<String> _visibleConversationIds = const [];
  Timer? _changeTimer;
  Timer? _reloadRetryTimer;
  int _changeRetryAttempt = 0;
  int _reloadRetryAttempt = 0;
  final Set<String> _pendingConversationIds = {};
  bool _reloadAllPending = false;
  bool _flushingChanges = false;
  bool _initialized = false;
  final List<String> _recentConversationIds = [];
  var _disposed = false;

  bool get initialized => _initialized;

  List<ConversationListEntry> get items => _store.items;
  List<ConversationListEntry> get visibleConversations => _visibleConversations;
  List<String> get visibleConversationIds => _visibleConversationIds;

  ConversationListEntry? item(String conversationId) =>
      _store.item(conversationId);

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

  ConversationListFilter get _filter => (
    category: filter.category,
    circleId: filter.circleId,
    query: filter.query.trim(),
    unseenOnly: filter.unseenOnly,
  );

  void _rebuildView() {
    _visibleConversations = _store.filtered(_filter);
    final ids = _visibleConversations
        .map((conversation) => conversation.id)
        .toList(growable: false);
    if (!listEquals(_visibleConversationIds, ids)) {
      _visibleConversationIds = ids;
    }
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
    filter.removeListener(_rebuildView);
    _changeTimer?.cancel();
    _reloadRetryTimer?.cancel();
    unawaited(_changeSubscription?.cancel());
    super.dispose();
  }
}
