import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/controllers/conversation_list_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/conversation_list_view.dart';
import 'package:mixin_desktop_ui/widgets/home_sidebar.dart';
import 'package:provider/provider.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  ConversationListController? controller;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    controller ??= ConversationListController(context.read<AccountHandle>());
  }

  @override
  void dispose() {
    controller?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ChangeNotifierProvider.value(
    value: controller!,
    child: const _HomeBody(),
  );
}

class _HomeBody extends StatefulWidget {
  const _HomeBody();

  @override
  State<_HomeBody> createState() => _HomeBodyState();
}

class _HomeBodyState extends State<_HomeBody> {
  String? selectedConversationId;
  bool userCollapsed = false;

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<ConversationListController>();
    final conversationList = ConversationListView(
      pagingState: controller.pagingState,
      itemPositionsListener: controller.itemPositionsListener,
      itemScrollController: controller.itemScrollController,
      loading: controller.loading,
      error: controller.error,
      currentUserId: controller.profile.userId,
      circles: {
        for (final circle in controller.circles) circle.circleId: circle.name,
      },
      currentCircleId: controller.category == ConversationCategoryFilter.circle
          ? controller.circleId
          : null,
      filterUnseen: controller.filterUnseen,
      selectedConversationId: selectedConversationId,
      onQueryChanged: controller.setQuery,
      onToggleUnseen: controller.toggleUnseen,
      onSelected: (conversation) {
        setState(() => selectedConversationId = conversation.id);
      },
      onPinned: (conversation) {
        unawaited(controller.setPinned(conversation));
      },
      onMuted: (conversation, duration) {
        unawaited(controller.setMuted(conversation, duration));
      },
      onDeleted: (conversation) {
        if (selectedConversationId == conversation.id) {
          setState(() => selectedConversationId = null);
        }
        unawaited(controller.deleteConversation(conversation));
      },
      onCircleChanged: (conversation, circleId, add) {
        unawaited(controller.editCircle(conversation, circleId, add));
      },
      onRetry: controller.refresh,
    );

    return Scaffold(
      backgroundColor: context.mixinTheme.primary,
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            if (constraints.maxWidth <= 384) return conversationList;
            final autoCollapsed = constraints.maxWidth < 496;
            final collapsed = userCollapsed || autoCollapsed;
            final sidebar = TweenAnimationBuilder<double>(
              tween: Tween(end: collapsed ? 64 : 176),
              duration: const Duration(milliseconds: 200),
              builder: (context, width, child) =>
                  SizedBox(width: width, child: child),
              child: OverflowBox(
                alignment: Alignment.centerLeft,
                minWidth: 64,
                maxWidth: collapsed ? 64 : 176,
                child: HomeSidebar(
                  controller: controller,
                  collapsed: collapsed,
                  showCollapse: !autoCollapsed,
                  onToggleCollapsed: () =>
                      setState(() => userCollapsed = !userCollapsed),
                ),
              ),
            );
            if (constraints.maxWidth < 620) {
              return Row(
                children: [
                  sidebar,
                  Expanded(child: conversationList),
                ],
              );
            }
            return Row(
              children: [
                sidebar,
                SizedBox(width: 300, child: conversationList),
                VerticalDivider(width: 1, color: context.mixinTheme.divider),
                Expanded(
                  child: ColoredBox(
                    color: context.mixinTheme.chatBackground,
                    child: Center(
                      child: Text(
                        context.l10n.pickAConversation,
                        style: TextStyle(
                          color: context.mixinTheme.secondaryText,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}
