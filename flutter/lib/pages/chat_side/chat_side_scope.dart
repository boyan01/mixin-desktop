import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/controllers/chat_side_notifier.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';

class ChatSideScope extends InheritedWidget {
  const ChatSideScope({
    required this.account,
    required this.conversation,
    required this.notifier,
    required this.currentUserId,
    required this.routeMode,
    required this.onLocateMessage,
    required this.onSelectConversation,
    required this.onConversationDeleted,
    required super.child,
    super.key,
  });

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final ChatSideNotifier notifier;
  final String currentUserId;
  final bool routeMode;
  final ValueChanged<String> onLocateMessage;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final VoidCallback onConversationDeleted;

  static ChatSideScope of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<ChatSideScope>();
    assert(scope != null, 'ChatSideScope is missing');
    return scope!;
  }

  @override
  bool updateShouldNotify(ChatSideScope oldWidget) =>
      account != oldWidget.account ||
      conversation != oldWidget.conversation ||
      notifier != oldWidget.notifier ||
      currentUserId != oldWidget.currentUserId ||
      routeMode != oldWidget.routeMode;
}

class ChatSidePageScaffold extends StatelessWidget {
  const ChatSidePageScaffold({
    required this.title,
    required this.body,
    this.root = false,
    this.actions = const [],
    this.backgroundColor,
    super.key,
  });

  final String title;
  final Widget body;
  final bool root;
  final List<Widget> actions;
  final Color? backgroundColor;

  @override
  Widget build(BuildContext context) {
    final scope = ChatSideScope.of(context);
    return Scaffold(
      backgroundColor: backgroundColor ?? context.theme.popUp,
      appBar: AppBar(
        automaticallyImplyLeading: false,
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: root
            ? null
            : IconButton(
                onPressed: scope.notifier.pop,
                icon: const Icon(Icons.arrow_back),
              ),
        title: Text(title),
        centerTitle: true,
        actions: [
          ...actions,
          IconButton(
            key: root ? const Key('chat-side-close') : null,
            onPressed: scope.notifier.clear,
            icon: const Icon(Icons.close),
          ),
        ],
      ),
      body: body,
    );
  }
}

class ChatSideCellGroup extends StatelessWidget {
  const ChatSideCellGroup({required this.children, super.key});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 10),
    child: Material(
      color: context.theme.primary,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (var index = 0; index < children.length; index++) ...[
            children[index],
            if (index + 1 < children.length)
              Divider(height: 1, indent: 16, color: context.theme.divider),
          ],
        ],
      ),
    ),
  );
}

class ChatSideCell extends StatelessWidget {
  const ChatSideCell({
    required this.title,
    this.description,
    this.trailing,
    this.onTap,
    this.destructive = false,
    super.key,
  });

  final String title;
  final String? description;
  final Widget? trailing;
  final VoidCallback? onTap;
  final bool destructive;

  @override
  Widget build(BuildContext context) => ListTile(
    minTileHeight: 54,
    title: Text(
      title,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: TextStyle(
        color: destructive ? Colors.red : context.theme.text,
        fontSize: 15,
      ),
    ),
    subtitle: description == null
        ? null
        : Text(
            description!,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: context.theme.secondaryText, fontSize: 13),
          ),
    trailing:
        trailing ??
        (onTap == null
            ? null
            : Icon(Icons.chevron_right, color: context.theme.secondaryText)),
    onTap: onTap,
  );
}

class ChatSideError extends StatelessWidget {
  const ChatSideError({required this.error, required this.onRetry, super.key});

  final Object error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(error.toString(), textAlign: TextAlign.center),
        const SizedBox(height: 12),
        TextButton(onPressed: onRetry, child: Text(context.l10n.retry)),
      ],
    ),
  );
}
