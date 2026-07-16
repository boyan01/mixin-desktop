import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/widgets/pinned_messages_page.dart';

class PinMessagesPage extends StatefulWidget {
  const PinMessagesPage({super.key});

  @override
  State<PinMessagesPage> createState() => _PinMessagesPageState();
}

class _PinMessagesPageState extends State<PinMessagesPage> {
  int count = 0;
  Future<String?>? role;

  @override
  Widget build(BuildContext context) {
    final scope = ChatSideScope.of(context);
    role ??= scope.account.currentUserRole(
      conversationId: scope.conversation.id,
    );
    return ChatSidePageScaffold(
      title: context.l10n.pinnedMessageTitle(count, count),
      body: FutureBuilder<String?>(
        future: role,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }
          if (snapshot.hasError) {
            return ChatSideError(
              error: snapshot.error!,
              onRetry: () => setState(() {
                role = scope.account.currentUserRole(
                  conversationId: scope.conversation.id,
                );
              }),
            );
          }
          return PinnedMessagesPage(
            account: scope.account,
            conversationId: scope.conversation.id,
            currentUserId: scope.currentUserId,
            currentUserRole: snapshot.data,
            onLocate: (messageId) {
              scope.onLocateMessage(messageId);
              scope.notifier.closeAfterContentJump(routeMode: scope.routeMode);
            },
            embedded: true,
            onEmpty: scope.notifier.pop,
            onCountChanged: (value) {
              if (mounted && value != count) setState(() => count = value);
            },
          );
        },
      ),
    );
  }
}
