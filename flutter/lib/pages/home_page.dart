import 'package:flutter/material.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/controllers/conversation_list_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/pages/settings_page.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/chat_view.dart';
import 'package:mixin_desktop_ui/widgets/conversation_list_view.dart';
import 'package:mixin_desktop_ui/widgets/home_sidebar.dart';
import 'package:provider/provider.dart';

enum DesktopShellLayoutMode { drawer, compactRail, fullRail }

const kSlidePageMinWidth = 64.0;
const kSlidePageMaxWidth = 176.0;
const kResponsiveNavigationMinWidth = 320.0;
const kConversationListWidth = 300.0;

class DesktopShellLayout {
  const DesktopShellLayout({
    required this.mode,
    required this.slideWidth,
    required this.availableSlideWidth,
    required this.autoCollapse,
    required this.userCollapse,
  });

  static const mainRouteSwitchWidth =
      kResponsiveNavigationMinWidth + kConversationListWidth;

  final DesktopShellLayoutMode mode;
  final double slideWidth;
  final double availableSlideWidth;
  final bool autoCollapse;
  final bool userCollapse;

  bool get hasDrawer => mode == DesktopShellLayoutMode.drawer;

  bool get collapse =>
      mode == DesktopShellLayoutMode.drawer ||
      mode == DesktopShellLayoutMode.compactRail;

  bool get showCollapseControl => !autoCollapse;

  double get slideMaxWidth =>
      collapse ? kSlidePageMinWidth : availableSlideWidth;

  static DesktopShellLayout resolve({
    required double maxWidth,
    required bool userCollapse,
    required bool isPhone,
  }) {
    final availableSlideWidth = (maxWidth - kResponsiveNavigationMinWidth)
        .clamp(kSlidePageMinWidth, kSlidePageMaxWidth);
    final autoCollapse = availableSlideWidth < kSlidePageMaxWidth;
    final collapse = userCollapse || autoCollapse;
    final hasDrawer = availableSlideWidth <= kSlidePageMinWidth || isPhone;
    final mode = hasDrawer
        ? DesktopShellLayoutMode.drawer
        : collapse
        ? DesktopShellLayoutMode.compactRail
        : DesktopShellLayoutMode.fullRail;

    return DesktopShellLayout(
      mode: mode,
      slideWidth: switch (mode) {
        DesktopShellLayoutMode.drawer => 0,
        DesktopShellLayoutMode.compactRail => kSlidePageMinWidth,
        DesktopShellLayoutMode.fullRail => kSlidePageMaxWidth,
      },
      availableSlideWidth: availableSlideWidth,
      autoCollapse: autoCollapse,
      userCollapse: userCollapse,
    );
  }
}

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
    child: const Portal(child: _HomeBody()),
  );
}

class _HomeBody extends StatefulWidget {
  const _HomeBody();

  @override
  State<_HomeBody> createState() => _HomeBodyState();
}

class _HomeBodyState extends State<_HomeBody> {
  ConversationListEntry? selectedConversation;
  final drafts = <String, String>{};
  bool userCollapsed = false;
  bool settingsSelected = false;

  void _updateDraft(ConversationListEntry conversation, String value) {
    final current = drafts[conversation.id] ?? conversation.draft;
    if (current == value) return;
    setState(() => drafts[conversation.id] = value);
  }

  void _showActionFailure() {
    if (!mounted) return;
    ScaffoldMessenger.maybeOf(
      context,
    )?.showSnackBar(SnackBar(content: Text(context.l10n.failed)));
  }

  void _selectCategory() {
    setState(() {
      settingsSelected = false;
      selectedConversation = null;
    });
  }

  void _selectProfile() {
    setState(() {
      settingsSelected = true;
      selectedConversation = null;
    });
  }

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
      query: controller.query,
      filterUnseen: controller.filterUnseen,
      selectedConversationId: selectedConversation?.id,
      onQueryChanged: controller.setQuery,
      onToggleUnseen: controller.toggleUnseen,
      onSelected: (conversation) {
        setState(() {
          settingsSelected = false;
          selectedConversation = conversation;
        });
      },
      onPinned: (conversation) async {
        try {
          await controller.setPinned(conversation);
        } catch (_) {
          _showActionFailure();
        }
      },
      onMuted: (conversation, duration) async {
        try {
          await controller.setMuted(conversation, duration);
        } catch (_) {
          _showActionFailure();
        }
      },
      onDeleted: (conversation) async {
        try {
          await controller.deleteConversation(conversation);
        } catch (_) {
          _showActionFailure();
          return;
        }

        if (mounted && selectedConversation?.id == conversation.id) {
          setState(() => selectedConversation = null);
        }
      },
      onCircleChanged: (conversation, circleId, add) async {
        try {
          await controller.editCircle(conversation, circleId, add);
        } catch (_) {
          _showActionFailure();
        }
      },
      onRetry: controller.refresh,
    );

    final conversationPane = RepaintBoundary(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: context.mixinTheme.primary,
          border: Border(right: BorderSide(color: context.mixinTheme.divider)),
        ),
        child: conversationList,
      ),
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final shellLayout = DesktopShellLayout.resolve(
          maxWidth: constraints.maxWidth,
          userCollapse: userCollapsed,
          isPhone: false,
        );
        final sidebar = TweenAnimationBuilder<double>(
          tween: Tween(end: shellLayout.slideWidth),
          duration: const Duration(milliseconds: 200),
          builder: (context, width, child) =>
              SizedBox(width: width, child: width == 0 ? null : child),
          child: OverflowBox(
            alignment: Alignment.centerLeft,
            minWidth: kSlidePageMinWidth,
            maxWidth: shellLayout.slideMaxWidth,
            child: HomeSidebar(
              controller: controller,
              collapsed: shellLayout.collapse,
              showCollapse: shellLayout.showCollapseControl,
              profileSelected: settingsSelected,
              onProfileSelected: _selectProfile,
              onCategorySelected: _selectCategory,
              onToggleCollapsed: () =>
                  setState(() => userCollapsed = !userCollapsed),
            ),
          ),
        );

        return Scaffold(
          backgroundColor: context.mixinTheme.primary,
          drawerEnableOpenDragGesture: false,
          drawer: shellLayout.hasDrawer
              ? Drawer(
                  child: Container(
                    width: kSlidePageMaxWidth,
                    color: context.mixinTheme.primary,
                    child: SafeArea(
                      child: HomeSidebar(
                        controller: controller,
                        collapsed: false,
                        showCollapse: false,
                        profileSelected: settingsSelected,
                        onProfileSelected: _selectProfile,
                        onCategorySelected: _selectCategory,
                        onToggleCollapsed: () {},
                      ),
                    ),
                  ),
                )
              : null,
          body: SafeArea(
            child: Row(
              children: [
                sidebar,
                Expanded(
                  child: LayoutBuilder(
                    builder: (context, mainConstraints) {
                      if (settingsSelected) {
                        return SettingsPage(
                          profile: controller.profile,
                          onSignOut: context.read<AppController>().signOut,
                          onClose: _selectCategory,
                        );
                      }

                      final routeMode =
                          mainConstraints.maxWidth <
                          DesktopShellLayout.mainRouteSwitchWidth;
                      final conversation = selectedConversation;
                      if (routeMode) {
                        if (conversation == null) return conversationPane;
                        return ChatView(
                          account: context.read<AccountHandle>(),
                          conversation: conversation,
                          draft: drafts[conversation.id] ?? conversation.draft,
                          onDraftChanged: (value) =>
                              _updateDraft(conversation, value),
                          onBack: () =>
                              setState(() => selectedConversation = null),
                        );
                      }

                      return Row(
                        children: [
                          SizedBox(
                            width: kConversationListWidth,
                            child: conversationPane,
                          ),
                          Expanded(
                            child: conversation == null
                                ? ColoredBox(
                                    color: context.mixinTheme.chatBackground,
                                    child: Center(
                                      child: Text(
                                        context.l10n.pickAConversation,
                                        style: TextStyle(
                                          color:
                                              context.mixinTheme.secondaryText,
                                        ),
                                      ),
                                    ),
                                  )
                                : ChatView(
                                    account: context.read<AccountHandle>(),
                                    conversation: conversation,
                                    draft:
                                        drafts[conversation.id] ??
                                        conversation.draft,
                                    onDraftChanged: (value) =>
                                        _updateDraft(conversation, value),
                                  ),
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
