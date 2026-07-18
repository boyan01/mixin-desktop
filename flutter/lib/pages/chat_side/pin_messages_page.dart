import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/conversation_info_destination.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/widgets/pinned_messages_page.dart';

class PinMessagesPage extends StatefulWidget {
  const PinMessagesPage({super.key});

  @override
  State<PinMessagesPage> createState() => _PinMessagesPageState();
}

class _PinMessagesPageState extends State<PinMessagesPage> {
  int count = 0;
  String? role;
  bool loadingRole = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!loadingRole && role == null) {
      loadingRole = true;
      final scope = ChatSideScope.of(context);
      scope.account
          .conversation()
          .currentUserRole(conversationId: scope.conversation.id)
          .then((value) {
            if (mounted) setState(() => role = value);
          }, onError: (Object _) {})
          .whenComplete(() => loadingRole = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final scope = ChatSideScope.of(context);
    return ChatSidePageScaffold(
      title: context.l10n.pinnedMessageTitle(count, count),
      body: PinnedMessagesPage(
        account: scope.account,
        conversationId: scope.conversation.id,
        currentUserId: scope.currentUserId,
        currentUserRole: role,
        onLocate: (messageId) {
          scope.onLocateMessage(messageId);
          scope.notifier.closeAfterContentJump(routeMode: scope.routeMode);
        },
        onSelectConversation: scope.onSelectConversation,
        onOpenUri: scope.onOpenUri,
        onSelectConversationInfo: (conversation) {
          scope.onSelectConversation(conversation);
          scope.notifier.openDestination(ConversationInfoDestination.infoPage);
        },
        onEmpty: scope.notifier.pop,
        onCountChanged: (value) {
          if (mounted && value != count) setState(() => count = value);
        },
      ),
    );
  }
}
