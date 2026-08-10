import 'package:flutter/widgets.dart';

import '../models/conversation_list_entry.dart';

class MessageNavigationScope extends InheritedWidget {
  const MessageNavigationScope({
    required this.currentUserId,
    required this.onLocateMessage,
    required super.child,
    this.onReedit,
    this.onSelectConversation,
    this.onSelectConversationInfo,
    super.key,
  });

  final String currentUserId;
  final void Function(String messageId, {String? sourceMessageId})
  onLocateMessage;
  final ValueChanged<String>? onReedit;
  final ValueChanged<ConversationListEntry>? onSelectConversation;
  final ValueChanged<ConversationListEntry>? onSelectConversationInfo;

  static MessageNavigationScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<MessageNavigationScope>();

  @override
  bool updateShouldNotify(MessageNavigationScope oldWidget) =>
      currentUserId != oldWidget.currentUserId ||
      onLocateMessage != oldWidget.onLocateMessage ||
      onReedit != oldWidget.onReedit ||
      onSelectConversation != oldWidget.onSelectConversation ||
      onSelectConversationInfo != oldWidget.onSelectConversationInfo;
}
