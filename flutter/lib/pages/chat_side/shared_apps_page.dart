import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/conversation_info_destination.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';

import 'chat_side_scope.dart';

class SharedAppsPage extends StatefulWidget {
  const SharedAppsPage({super.key});

  @override
  State<SharedAppsPage> createState() => _SharedAppsPageState();
}

class _SharedAppsPageState extends State<SharedAppsPage> {
  List<rust.SharedAppItem> apps = const [];
  bool loadStarted = false;
  bool localLoaded = false;
  Object? error;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!loadStarted) {
      loadStarted = true;
      unawaited(_load());
    }
  }

  Future<void> _load() async {
    final scope = ChatSideScope.of(context);
    if (scope.conversation.isGroup) {
      setState(() => localLoaded = true);
      return;
    }
    try {
      final values = await scope.account.user().localSharedApps(
        userId: scope.conversation.ownerId,
      );
      if (!mounted) return;
      setState(() {
        apps = values;
        localLoaded = true;
        error = null;
      });
      unawaited(_refresh());
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        localLoaded = true;
        error = exception;
      });
    }
  }

  Future<void> _refresh() async {
    final scope = ChatSideScope.of(context);
    try {
      final values = await scope.account.user().sharedApps(
        userId: scope.conversation.ownerId,
      );
      if (!mounted) return;
      setState(() {
        apps = values;
        error = null;
      });
    } on Object catch (exception) {
      if (mounted && apps.isEmpty) setState(() => error = exception);
    }
  }

  Future<void> _openApp(rust.SharedAppItem app) async {
    final scope = ChatSideScope.of(context);
    final result = await showMessageUserDialog(
      context,
      account: scope.account,
      userId: app.appId,
    );
    if (!mounted || !context.mounted || result == null) return;
    await handleMessageUserDialogResult(
      context,
      account: scope.account,
      result: result,
      onSelectConversation: scope.onSelectConversation,
      onSelectConversationInfo: (conversation) {
        scope.onSelectConversation(conversation);
        scope.notifier.openDestination(ConversationInfoDestination.infoPage);
      },
    );
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.shareApps,
    backgroundColor: context.theme.primary,
    body: Column(
      children: [
        const SizedBox(height: 6),
        for (final app in apps) _AppTile(app: app, onTap: () => _openApp(app)),
      ],
    ),
  );
}

class _AppTile extends StatelessWidget {
  const _AppTile({required this.app, required this.onTap});

  final rust.SharedAppItem app;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => InkWell(
    onTap: onTap,
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 16),
      child: Row(
        children: [
          _AppIcon(app: app, size: 50),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  app.name,
                  style: TextStyle(color: context.theme.text, fontSize: 16),
                ),
                const SizedBox(height: 3),
                Text(
                  app.description,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 14,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class OverlappedAppIcons extends StatelessWidget {
  const OverlappedAppIcons({required this.apps, super.key})
    : assert(apps.length > 0);

  final List<rust.SharedAppItem> apps;

  @override
  Widget build(BuildContext context) => Stack(
    children: [
      for (var index = 0; index < apps.length; index++)
        Padding(
          padding: EdgeInsets.fromLTRB(index * 14, 0, 0, 0),
          child: ClipOval(
            child: Container(
              color: Color.alphaBlend(
                context.theme.listSelected,
                context.theme.popUp,
              ),
              padding: const EdgeInsets.all(2),
              child: _AppIcon(app: apps[index], size: 24),
            ),
          ),
        ),
    ].reversed.toList(),
  );
}

class _AppIcon extends StatelessWidget {
  const _AppIcon({required this.app, required this.size});

  final rust.SharedAppItem app;
  final double size;

  @override
  Widget build(BuildContext context) => ClipOval(
    child: MixinImage.network(
      app.iconUrl,
      width: size,
      height: size,
      placeholder: () => SizedBox.fromSize(
        size: Size.square(size),
        child: ColoredBox(color: context.theme.listSelected),
      ),
    ),
  );
}
