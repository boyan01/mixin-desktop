import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import '../../constants/assets.dart';
import '../../controllers/chat_side_notifier.dart';
import '../../models/conversation_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../widgets/settings_widgets.dart';

class ChatSideScope extends InheritedWidget {
  const ChatSideScope({
    required this.account,
    required this.conversation,
    required this.notifier,
    required this.currentUserId,
    required this.routeMode,
    required this.onLocateMessage,
    required this.onSelectConversation,
    required this.onOpenUri,
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
  final ValueChanged<Uri> onOpenUri;
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
      routeMode != oldWidget.routeMode ||
      onOpenUri != oldWidget.onOpenUri;
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
      appBar: MixinAppBar(
        title: title.isEmpty ? null : Text(title),
        backgroundColor: backgroundColor ?? context.theme.popUp,
        leading: root
            ? const SizedBox(width: 56)
            : IconButton(
                onPressed: scope.notifier.pop,
                icon: SvgPicture.asset(
                  MixinAssets.back,
                  width: 20,
                  height: 20,
                  colorFilter: ColorFilter.mode(
                    context.theme.icon,
                    BlendMode.srcIn,
                  ),
                ),
              ),
        actions: [
          ...actions,
          if (root)
            IconButton(
              key: const Key('chat-side-close'),
              onPressed: scope.notifier.clear,
              icon: SvgPicture.asset(
                MixinAssets.close,
                width: 20,
                height: 20,
                colorFilter: ColorFilter.mode(
                  context.theme.icon,
                  BlendMode.srcIn,
                ),
              ),
            ),
        ],
      ),
      body: body,
    );
  }
}
