import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';

import '../controllers/conversation_list_controller.dart';
import '../controllers/conversation_list_viewport.dart';
import '../controllers/home_navigation_controller.dart';
import '../controllers/security_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart';
import '../widgets/command_palette.dart' as command_palette;
import '../widgets/show_forward_conversation_selector.dart';
import 'conversation_create_dialogs.dart';

class HomeShortcuts extends HookWidget {
  const HomeShortcuts({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final account = context.read<AccountHandle>();
    final controller = context.read<ConversationListController>();
    final viewport = context.read<ConversationListViewport>();
    final navigation = context.read<HomeNavigationController>();
    final commandPaletteShowing = useRef(false);
    final isMacOS = defaultTargetPlatform == TargetPlatform.macOS;

    Future<void> showCommandPalette() async {
      if (commandPaletteShowing.value) return;
      commandPaletteShowing.value = true;
      try {
        await command_palette.showCommandPalette(
          context: context,
          search: controller.searchCommandPalette,
          onSelected: (item) {
            final conversation = item.conversation;
            if (conversation != null) {
              navigation.select(conversation);
            } else {
              unawaited(showConversationUser(context, userId: item.id));
            }
          },
        );
      } finally {
        commandPaletteShowing.value = false;
      }
    }

    void navigateConversation({required bool forward}) {
      if (navigation.settingsSelected ||
          navigation.selectedConversation == null) {
        return;
      }
      final conversations = controller.visibleConversations;
      final index = conversations.indexWhere(
        (item) => item.id == navigation.selectedConversation!.id,
      );
      if (index < 0) return;
      final nextIndex = forward ? index + 1 : index - 1;
      if (nextIndex < 0 || nextIndex >= conversations.length) return;
      navigation.select(conversations[nextIndex]);
      if (viewport.itemScrollController.isAttached) {
        viewport.itemScrollController.jumpTo(
          index: nextIndex,
          alignment: forward ? 0.9 : 0,
        );
      }
    }

    Future<void> createConversation() async {
      final conversation = await showConversationSelector(
        context,
        account: account,
        title: context.l10n.createConversation,
        category: ConversationCategoryFilter.contacts,
      );
      if (context.mounted && conversation != null) {
        navigation.select(conversation);
      }
    }

    return CallbackShortcuts(
      bindings: {
        SingleActivator(
          LogicalKeyboardKey.keyK,
          meta: isMacOS,
          control: !isMacOS,
        ): () =>
            unawaited(showCommandPalette()),
        SingleActivator(
          LogicalKeyboardKey.arrowDown,
          meta: isMacOS,
          control: !isMacOS,
        ): () =>
            navigateConversation(forward: true),
        SingleActivator(
          LogicalKeyboardKey.arrowUp,
          meta: isMacOS,
          control: !isMacOS,
        ): () =>
            navigateConversation(forward: false),
        if (isMacOS) ...{
          const SingleActivator(LogicalKeyboardKey.comma, meta: true):
              navigation.showSettings,
          const SingleActivator(
            LogicalKeyboardKey.keyL,
            meta: true,
            shift: true,
          ): context
              .read<SecurityController>()
              .lockNow,
          const SingleActivator(LogicalKeyboardKey.keyN, meta: true): () =>
              unawaited(createConversation()),
          const SingleActivator(
            LogicalKeyboardKey.keyN,
            meta: true,
            shift: true,
          ): () =>
              unawaited(showCreateGroupDialog(context)),
          const SingleActivator(LogicalKeyboardKey.keyM, meta: true): () =>
              unawaited(windowManager.minimize()),
        },
      },
      child: Focus(autofocus: true, child: child),
    );
  }
}
