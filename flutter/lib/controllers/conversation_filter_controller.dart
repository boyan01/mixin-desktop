import 'package:flutter/foundation.dart';

import '../models/conversation_list_entry.dart';

class ConversationFilterController extends ChangeNotifier {
  ConversationCategoryFilter category = ConversationCategoryFilter.chats;
  String? circleId;
  String query = '';
  bool unseenOnly = false;

  void selectCategory(
    ConversationCategoryFilter value, {
    String? circle,
  }) {
    if (category == value && circleId == circle) return;
    category = value;
    circleId = circle;
    notifyListeners();
  }

  void setQuery(String value) {
    if (query == value) return;
    query = value;
    notifyListeners();
  }

  void toggleUnseen() {
    unseenOnly = !unseenOnly;
    notifyListeners();
  }
}
