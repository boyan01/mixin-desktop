import 'package:flutter/foundation.dart';

import '../models/conversation_list_entry.dart';
import 'chat_side_notifier.dart';

class HomeNavigationController extends ChangeNotifier {
  HomeNavigationController({required this.onConversationSelected});

  ConversationListEntry? _selectedConversation;
  bool _settingsSelected = false;
  String? _locateMessageId;
  int _locateRequest = 0;

  final ValueChanged<ConversationListEntry> onConversationSelected;
  final ChatSideNotifier chatSideController = ChatSideNotifier();

  ConversationListEntry? get selectedConversation => _selectedConversation;
  bool get settingsSelected => _settingsSelected;
  String? get locateMessageId => _locateMessageId;
  int get locateRequest => _locateRequest;

  void select(ConversationListEntry conversation) {
    final changed =
        _selectedConversation != conversation ||
        _settingsSelected ||
        _locateMessageId != null;
    _selectedConversation = conversation;
    _settingsSelected = false;
    _locateMessageId = null;
    chatSideController.clear();
    onConversationSelected(conversation);
    if (changed) notifyListeners();
  }

  void showChats() {
    final changed = _selectedConversation != null || _settingsSelected;
    _selectedConversation = null;
    _settingsSelected = false;
    chatSideController.clear();
    if (changed) notifyListeners();
  }

  void showSettings() {
    final changed = _selectedConversation != null || !_settingsSelected;
    _selectedConversation = null;
    _settingsSelected = true;
    chatSideController.clear();
    if (changed) notifyListeners();
  }

  void clearSelection() {
    if (_selectedConversation == null) return;
    _selectedConversation = null;
    notifyListeners();
  }

  void conversationDeleted() {
    final changed = _selectedConversation != null || _locateMessageId != null;
    _selectedConversation = null;
    _locateMessageId = null;
    if (changed) notifyListeners();
  }

  void locateMessage(String messageId) {
    _locateMessageId = messageId;
    _locateRequest++;
    notifyListeners();
  }

  @override
  void dispose() {
    chatSideController.dispose();
    super.dispose();
  }
}
