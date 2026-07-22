import 'package:flutter/material.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../../constants/icon_fonts.dart';
import '../../l10n/l10n.dart';
import '../../theme.dart';
import '../../widgets/custom_context_menu.dart';
import 'chat_side_scope.dart';
import 'share_media/file_page.dart';
import 'share_media/media_page.dart';
import 'share_media/post_page.dart';

class SharedMediaPage extends StatefulWidget {
  const SharedMediaPage({super.key});

  @override
  State<SharedMediaPage> createState() => _SharedMediaPageState();
}

class _SharedMediaPageState extends State<SharedMediaPage> {
  int selectedIndex = 0;

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.sharedMedia,
    backgroundColor: context.theme.primary,
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
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTap: () => setState(() => selectedIndex = entry.key),
                    child: Container(
                      height: 56,
                      alignment: Alignment.center,
                      child: Text(
                        entry.value,
                        style: TextStyle(
                          fontSize: 14,
                          color: entry.key == selectedIndex
                              ? context.theme.accent
                              : context.theme.secondaryText,
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
    desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
    menuProvider: (_) => Menu(
      children: [
        MenuAction(
          image: MenuImage.icon(IconFonts.positionToChat),
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
