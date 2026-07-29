import 'dart:async';

import 'package:flutter/foundation.dart';

import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';
import 'conversation_filter_controller.dart';
import 'conversation_list_controller.dart';

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

class ConversationSearchController extends ChangeNotifier {
  ConversationSearchController({
    required this.account,
    required this.filter,
    required this.conversations,
  }) {
    filter.addListener(_scheduleSearch);
  }

  final rust.AccountHandle account;
  final ConversationFilterController filter;
  final ConversationListController conversations;

  bool loading = false;
  List<rust.UserProfileItem> users = const [];
  rust.UserProfileItem? maoUser;
  String? mao;
  List<MessageListEntry> messages = const [];
  Map<String, ConversationListEntry> messageConversations = const {};

  Timer? _timer;
  int _revision = 0;
  var _disposed = false;

  void _scheduleSearch() {
    _timer?.cancel();
    final normalized = filter.query.trim();
    final revision = ++_revision;
    if (normalized.isEmpty || filter.unseenOnly) {
      loading = false;
      messages = const [];
      users = const [];
      maoUser = null;
      mao = null;
      messageConversations = const {};
      notifyListeners();
      return;
    }
    loading = true;
    maoUser = null;
    mao = null;
    notifyListeners();
    _timer = Timer(const Duration(milliseconds: 150), () async {
      unawaited(_searchMaoUser(normalized, revision));
      try {
        final results = await Future.wait<dynamic>([
          account.message().searchGlobalMessages(
            query: normalized,
            limit: 32,
          ),
          account.user().searchLocalUsers(
            query: normalized,
            category: filter.category.name,
            limit: 64,
          ),
        ]);
        if (_disposed || revision != _revision) return;
        messages = (results[0] as List<rust.MessageListItem>)
            .map(MessageListEntry.fromRust)
            .toList(growable: false);
        users = results[1] as List<rust.UserProfileItem>;
        messageConversations = {
          for (final item in conversations.items) item.id: item,
        };
        loading = false;
        notifyListeners();
      } on Object catch (error, stackTrace) {
        if (_disposed || revision != _revision) return;
        messages = const [];
        users = const [];
        messageConversations = const {};
        loading = false;
        e(
          'Search conversations failed: query=$normalized',
          error,
          stackTrace,
        );
        notifyListeners();
      }
    });
  }

  Future<void> _searchMaoUser(String query, int revision) async {
    final completedMao = _completeMao(query);
    if (completedMao == null) return;
    try {
      final user = await account.user().searchMaoUser(query: query);
      if (_disposed || revision != _revision) return;
      maoUser = user;
      mao = user == null ? null : completedMao;
      notifyListeners();
    } on Object catch (error, stackTrace) {
      e('Search Mixin ID user failed: query=$query', error, stackTrace);
      if (_disposed || revision != _revision) return;
      maoUser = null;
      mao = null;
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _timer?.cancel();
    filter.removeListener(_scheduleSearch);
    super.dispose();
  }
}
