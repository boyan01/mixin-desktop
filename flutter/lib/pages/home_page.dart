import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:provider/provider.dart';

import '../controllers/app_controller.dart';
import '../controllers/chat_side_notifier.dart';
import '../controllers/conversation_filter_controller.dart';
import '../controllers/conversation_list_controller.dart';
import '../controllers/conversation_list_viewport.dart';
import '../controllers/conversation_search_controller.dart';
import '../controllers/home_navigation_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import '../utils/app_logger.dart';
import '../utils/local_notification_center.dart';
import '../widgets/account_health_overlays.dart';
import '../widgets/app_icon_badge.dart';
import '../widgets/app_protocol_handler.dart';
import '../widgets/chat_view.dart';
import '../widgets/device_transfer_widget.dart';
import '../widgets/home_macos_menu_bar.dart';
import '../widgets/home_notification_bridge.dart';
import '../widgets/home_sidebar.dart';
import '../widgets/mixin_dialog.dart';
import '../widgets/toast.dart';
import 'chat_side/chat_side_router.dart';
import 'conversation_info_destination.dart';
import 'conversation_list_pane.dart';
import 'home_shortcuts.dart';
import 'settings_page.dart';

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

class HomePage extends StatelessWidget {
  const HomePage({super.key, this.initialProtocolUrl});

  final String? initialProtocolUrl;

  @override
  Widget build(BuildContext context) => MultiProvider(
    providers: [
      ChangeNotifierProvider(
        create: (_) => ConversationFilterController(),
      ),
      ChangeNotifierProvider(
        create: (context) => ConversationListController(
          context.read<AccountHandle>(),
          context.read<ConversationFilterController>(),
        ),
      ),
      ChangeNotifierProvider(
        create: (context) => ConversationSearchController(
          account: context.read<AccountHandle>(),
          filter: context.read<ConversationFilterController>(),
          conversations: context.read<ConversationListController>(),
        ),
      ),
      Provider(create: (_) => ConversationListViewport()),
    ],
    child: Portal(
      child: _HomeBody(initialProtocolUrl: initialProtocolUrl),
    ),
  );
}

class _HomeBody extends StatefulWidget {
  const _HomeBody({this.initialProtocolUrl});

  final String? initialProtocolUrl;

  @override
  State<_HomeBody> createState() => _HomeBodyState();
}

class _HomeBodyState extends State<_HomeBody> {
  final drafts = <String, String>{};
  bool userCollapsed = false;
  late final HomeNavigationController navigationController;

  ConversationListEntry? get selectedConversation =>
      navigationController.selectedConversation;
  bool get settingsSelected => navigationController.settingsSelected;
  String? get locateMessageId => navigationController.locateMessageId;
  int get locateRequest => navigationController.locateRequest;
  HomeNavigationController get navigation => navigationController;

  @override
  void initState() {
    super.initState();
    navigationController = HomeNavigationController(
      onConversationSelected: _onConversationSelected,
    )..addListener(_onSelectedConversationChanged);
  }

  void _onSelectedConversationChanged() {
    if (mounted) setState(() {});
  }

  void _onConversationSelected(ConversationListEntry conversation) {
    context.read<ConversationListController>().recordRecentConversation(
      conversation.id,
    );
    unawaited(dismissConversationNotifications(conversation.id));
  }

  void _updateDraft(ConversationListEntry conversation, String value) {
    final current = drafts[conversation.id] ?? conversation.draft;
    if (current == value) return;
    setState(() => drafts[conversation.id] = value);
  }

  void _selectCategory() {
    navigation.showChats();
  }

  void _selectConversation(ConversationListEntry conversation) {
    navigation.select(conversation);
  }

  void _locateMessage(String messageId) {
    navigation.locateMessage(messageId);
  }

  void _conversationDeleted() {
    navigation.conversationDeleted();
  }

  void _selectProfile() {
    navigation.showSettings();
  }

  @override
  void dispose() {
    navigationController
      ..removeListener(_onSelectedConversationChanged)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const conversationPane = ConversationListPane();
    final homeShell = LayoutBuilder(
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
                        return _ProfileSettingsPage(
                          account: context.read<AccountHandle>(),
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
                        return _ChatWithSide(
                          account: context.read<AccountHandle>(),
                          conversation: conversation,
                          notifier: navigation.chatSideController,
                          draft: drafts[conversation.id] ?? conversation.draft,
                          locateMessageId: locateMessageId,
                          locateRequest: locateRequest,
                          onDraftChanged: (value) =>
                              _updateDraft(conversation, value),
                          onBack: navigation.clearSelection,
                          onLocateMessage: _locateMessage,
                          onSelectConversation: _selectConversation,
                          onConversationDeleted: _conversationDeleted,
                        );
                      }

                      return Row(
                        children: [
                          const SizedBox(
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
                                : _ChatWithSide(
                                    account: context.read<AccountHandle>(),
                                    conversation: conversation,
                                    notifier: navigation.chatSideController,
                                    draft:
                                        drafts[conversation.id] ??
                                        conversation.draft,
                                    locateMessageId: locateMessageId,
                                    locateRequest: locateRequest,
                                    onDraftChanged: (value) =>
                                        _updateDraft(conversation, value),
                                    onLocateMessage: _locateMessage,
                                    onSelectConversation: _selectConversation,
                                    onConversationDeleted: _conversationDeleted,
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
    final shortcutHome = HomeShortcuts(child: homeShell);
    final home = MacosMenuBar(child: shortcutHome);
    final content = _ProfileSetupGate(
      account: context.read<AccountHandle>(),
      child: home,
    );
    final account = context.read<AccountHandle>();
    return MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: navigationController),
      ],
      child: DeviceTransferHandlerWidget(
        child: AppIconBadge(
          account: account,
          child: AccountHealthOverlays(
            account: account,
            child: AppProtocolHandler(
              initialUrl: widget.initialProtocolUrl,
              child: HomeNotificationBridge(
                account: account,
                child: content,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ProfileSettingsPage extends HookWidget {
  const _ProfileSettingsPage({
    required this.account,
    required this.onSignOut,
    required this.onClose,
  });

  final AccountHandle account;
  final Future<void> Function() onSignOut;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final profile = useStream(
      useMemoized(account.profileChanges, [account]),
      initialData: account.profile(),
    ).data!;
    return SettingsPage(
      profile: profile,
      onSignOut: onSignOut,
      onProfileUpdated: (fullName, biography) => account.updateProfile(
        fullName: fullName.trim(),
        biography: biography.trim(),
      ),
      onProfileRefresh: account.refreshProfile,
      onClose: onClose,
    );
  }
}

class _ProfileSetupGate extends HookWidget {
  const _ProfileSetupGate({
    required this.account,
    required this.child,
  });

  final AccountHandle account;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final profile = useStream(
      useMemoized(account.profileChanges, [account]),
      initialData: account.profile(),
    ).data!;
    if (profile.fullName.trim().isNotEmpty) return child;
    return Stack(
      fit: StackFit.expand,
      children: [
        child,
        _SetupNameOverlay(account: account),
      ],
    );
  }
}

class _SetupNameOverlay extends StatefulWidget {
  const _SetupNameOverlay({required this.account});

  final AccountHandle account;

  @override
  State<_SetupNameOverlay> createState() => _SetupNameOverlayState();
}

class _SetupNameOverlayState extends State<_SetupNameOverlay> {
  final nameController = TextEditingController();

  @override
  void initState() {
    super.initState();
    nameController.addListener(_changed);
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    nameController
      ..removeListener(_changed)
      ..dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final name = nameController.text.trim();
    if (name.isEmpty) return;
    showToastLoading();
    try {
      await widget.account.updateProfile(
        fullName: name,
        biography: widget.account.profile().biography,
      );
    } on Object catch (error, stackTrace) {
      e('Update profile failed', error, stackTrace);
      showToastFailed(error);
      return;
    }
    showToastSuccessful();
  }

  @override
  Widget build(BuildContext context) => Material(
    color: context.mixinTheme.background,
    child: Center(
      child: AlertDialogLayout(
        title: Text(context.l10n.editName),
        content: DialogTextField(
          key: const ValueKey('setup-account-name'),
          textEditingController: nameController,
          hintText: context.l10n.name,
          maxLength: 40,
        ),
        actions: [
          MixinButton(
            key: const ValueKey('setup-account-name-confirm'),
            disable: nameController.text.trim().isEmpty,
            onTap: _save,
            child: Text(context.l10n.confirm),
          ),
        ],
      ),
    ),
  );
}

class _ChatWithSide extends StatelessWidget {
  const _ChatWithSide({
    required this.account,
    required this.conversation,
    required this.notifier,
    required this.draft,
    required this.locateMessageId,
    required this.locateRequest,
    required this.onDraftChanged,
    required this.onLocateMessage,
    required this.onSelectConversation,
    required this.onConversationDeleted,
    this.onBack,
  });

  final AccountHandle account;
  final ConversationListEntry conversation;
  final ChatSideNotifier notifier;
  final String draft;
  final String? locateMessageId;
  final int locateRequest;
  final ValueChanged<String> onDraftChanged;
  final VoidCallback? onBack;
  final ValueChanged<String> onLocateMessage;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final VoidCallback onConversationDeleted;

  @override
  Widget build(BuildContext context) => ChangeNotifierProvider(
    create: (_) =>
        SearchConversationKeywordNotifier(chatSideNotifier: notifier),
    child: LayoutBuilder(
      builder: (context, constraints) {
        final routeMode =
            constraints.maxWidth <
            kResponsiveNavigationMinWidth + kChatSidePageWidth;
        return AnimatedBuilder(
          animation: notifier,
          builder: (context, _) {
            final chatPage = MaterialPage<void>(
              key: ValueKey('chat:${conversation.id}'),
              name: 'chat',
              child: ChatView(
                account: account,
                media: context.read<MediaHandle>(),
                conversation: conversation,
                draft: draft,
                locateMessageId: locateMessageId,
                locateRequest: locateRequest,
                onDraftChanged: onDraftChanged,
                onSelectConversation: onSelectConversation,
                onSelectConversationInfo: (conversation) {
                  onSelectConversation(conversation);
                  notifier.openDestination(
                    ConversationInfoDestination.infoPage,
                  );
                },
                onBack: onBack,
                onSearch: () => notifier.toggleDestination(
                  ConversationInfoDestination.searchMessageHistory,
                ),
                onInfo: notifier.toggleInfoPage,
                onOpenPinnedMessages: () => notifier.toggleDestination(
                  ConversationInfoDestination.pinMessages,
                ),
                showInfoAction: notifier.state.destinations.isEmpty,
              ),
            );
            return Row(
              children: [
                if (!routeMode) Expanded(child: chatPage.child),
                if (!routeMode)
                  Container(width: 1, color: context.mixinTheme.divider),
                ChatSideRouter(
                  account: account,
                  conversation: conversation,
                  notifier: notifier,
                  constraints: constraints,
                  routeMode: routeMode,
                  leadingPages: [if (routeMode) chatPage],
                  destinations: notifier.state.destinations,
                  onLocateMessage: onLocateMessage,
                  onSelectConversation: onSelectConversation,
                  onConversationDeleted: onConversationDeleted,
                ),
              ],
            );
          },
        );
      },
    ),
  );
}
