import 'dart:async';
import 'package:flutter/widgets.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';

class MentionController extends ChangeNotifier {
  MentionController({
    required this.account,
    required this.conversation,
    required this.inputController,
    required this.onTextChanged,
    required this.onMentionSelected,
    required this.onMentionNamesChanged,
  }) {
    inputController.addListener(_onInputChanged);
    _onInputChanged();
  }

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final TextEditingController inputController;
  final ValueChanged<String> onTextChanged;
  final ValueChanged<String> onMentionSelected;
  final ValueChanged<Map<String, String>> onMentionNamesChanged;

  List<rust.ConversationParticipantItem> users = const [];
  String keyword = '';
  int index = 0;
  final ScrollController scrollController = ScrollController();
  int _revision = 0;
  int _mentionNamesRevision = 0;
  bool _disposed = false;

  void _onInputChanged() {
    _resolveMentionNames(inputController.text);
    if (!conversation.isGroup && !conversation.isBot) return _clear();
    final selection = inputController.selection.baseOffset;
    if (selection < 0 || selection > inputController.text.length) {
      return _clear();
    }
    final match = RegExp(
      r'@(\S*)$',
    ).firstMatch(inputController.text.substring(0, selection));
    if (match == null) return _clear();

    final nextKeyword = match.group(1) ?? '';
    if (keyword != nextKeyword && scrollController.hasClients) {
      scrollController.jumpTo(0);
    }
    keyword = nextKeyword;
    final revision = ++_revision;
    unawaited(_load(revision, keyword));
  }

  Future<void> _resolveMentionNames(String text) async {
    final revision = ++_mentionNamesRevision;
    try {
      final names = await account.user().mentionNames(contents: [text]);
      if (_disposed || revision != _mentionNamesRevision) return;
      onMentionNamesChanged(names);
    } on Object catch (error, stackTrace) {
      e('resolve input mention names failed', error, stackTrace);
    }
  }

  Future<void> _load(int revision, String searchKeyword) async {
    try {
      final result = switch ((conversation.isGroup, searchKeyword.isEmpty)) {
        (true, true) =>
          (await account.conversation().conversationParticipants(
                conversationId: conversation.id,
              ))
              .where((user) => user.userId != account.accountId())
              .toList(growable: false),
        (true, false) => await account.conversation().searchGroupUsers(
          conversationId: conversation.id,
          keyword: searchKeyword,
        ),
        (false, _) => await account.conversation().searchBotGroupUsers(
          conversationId: conversation.id,
          keyword: searchKeyword,
        ),
      };
      if (_disposed || revision != _revision) return;
      users = result;
      index = 0;
      notifyListeners();
    } on Object catch (error, stackTrace) {
      e('load mention candidates failed', error, stackTrace);
      if (_disposed || revision != _revision) return;
      _clear();
    }
  }

  void move(int delta) {
    if (users.isEmpty) return;
    index = (index + delta).clamp(0, users.length - 1);
    _jumpToPosition(index);
    notifyListeners();
  }

  void _jumpToPosition(int selectedIndex) {
    if (!scrollController.hasClients) return;
    final viewportDimension = scrollController.position.viewportDimension;
    final offset = scrollController.offset;
    final maxScrollExtent = users.length * 50.0;
    final maxValidScrollExtent = maxScrollExtent - viewportDimension;
    final startIndex = offset ~/ 50.0;
    final endIndex = (offset + viewportDimension - 50.0) ~/ 50.0;
    final target = switch (selectedIndex) {
      _ when selectedIndex <= startIndex =>
        (50.0 * selectedIndex - viewportDimension + 100.0)
            .clamp(0, maxValidScrollExtent)
            .toDouble(),
      _ when selectedIndex >= endIndex =>
        (50.0 * selectedIndex - 50.0).clamp(0, maxValidScrollExtent).toDouble(),
      _ => null,
    };
    if (target != null) {
      unawaited(
        scrollController.animateTo(
          target,
          duration: const Duration(milliseconds: 150),
          curve: Curves.easeIn,
        ),
      );
    }
  }

  void select([int? selectedIndex]) {
    if (users.isEmpty) return;
    final user = users[selectedIndex ?? index];
    final selection = inputController.selection.baseOffset.clamp(
      0,
      inputController.text.length,
    );
    final before = inputController.text
        .substring(0, selection)
        .replaceFirst(RegExp(r'@(\S*)$'), '@${user.identityNumber} ');
    inputController.value = TextEditingValue(
      text: before + inputController.text.substring(selection),
      selection: TextSelection.collapsed(offset: before.length),
    );
    onMentionSelected(user.identityNumber);
    onTextChanged(inputController.text);
    _clear();
  }

  void _clear() {
    _revision++;
    if (users.isEmpty && keyword.isEmpty && index == 0) return;
    users = const [];
    keyword = '';
    index = 0;
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    inputController.removeListener(_onInputChanged);
    scrollController.dispose();
    super.dispose();
  }
}
