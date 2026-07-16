import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:super_context_menu/super_context_menu.dart';

import 'share_media/file_page.dart';
import 'share_media/media_page.dart';
import 'share_media/post_page.dart';

class SharedMediaPage extends StatefulWidget {
  const SharedMediaPage({super.key});

  @override
  State<SharedMediaPage> createState() => _SharedMediaPageState();
}

class _SharedMediaPageState extends State<SharedMediaPage> {
  var selectedIndex = 0;

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.sharedMedia,
    body: Column(
      children: [
        Expanded(
          child: IndexedStack(
            index: selectedIndex,
            children: const [MediaPage(), PostPage(), FilePage()],
          ),
        ),
        Row(
          children: [context.l10n.media, context.l10n.post, context.l10n.file]
              .asMap()
              .entries
              .map(
                (entry) => Expanded(
                  child: InkWell(
                    onTap: () => setState(() => selectedIndex = entry.key),
                    child: SizedBox(
                      height: 56,
                      child: Center(
                        child: Text(
                          entry.value,
                          style: TextStyle(
                            color: entry.key == selectedIndex
                                ? context.theme.accent
                                : context.theme.secondaryText,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              )
              .toList(),
        ),
      ],
    ),
  );
}

class ShareMediaItemMenuWrapper extends StatelessWidget {
  const ShareMediaItemMenuWrapper({
    required this.child,
    required this.messageId,
    super.key,
  });

  final Widget child;
  final String messageId;

  @override
  Widget build(BuildContext context) => ContextMenuWidget(
    menuProvider: (_) => Menu(
      children: [
        MenuAction(
          image: MenuImage.icon(Icons.my_location),
          title: context.l10n.locateToChat,
          callback: () {
            final scope = ChatSideScope.of(context);
            scope.onLocateMessage(messageId);
            scope.notifier.closeAfterContentJump(routeMode: scope.routeMode);
          },
        ),
      ],
    ),
    child: child,
  );
}
