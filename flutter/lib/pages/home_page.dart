import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';

import '../controllers/app_controller.dart';
import '../controllers/chat_side_notifier.dart';
import '../controllers/conversation_list_controller.dart';
import '../controllers/home_navigation_controller.dart';
import '../controllers/security_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import '../utils/app_logger.dart';
import '../utils/local_notification_center.dart';
import '../utils/web_view.dart';
import '../widgets/account_health_overlays.dart';
import '../widgets/app_icon_badge.dart';
import '../widgets/app_protocol_handler.dart';
import '../widgets/audio_player_bar.dart';
import '../widgets/avatar_view.dart';
import '../widgets/chat_view.dart';
import '../widgets/command_palette.dart';
import '../widgets/conversation_list_view.dart';
import '../widgets/device_transfer_widget.dart';
import '../widgets/home_macos_menu_bar.dart';
import '../widgets/home_notification_bridge.dart';
import '../widgets/home_sidebar.dart';
import '../widgets/mixin_dialog.dart';
import '../widgets/network_status.dart';
import '../widgets/show_forward_conversation_selector.dart';
import '../widgets/show_message_user_dialog.dart';
import '../widgets/toast.dart';
import 'chat_side/chat_side_router.dart';
import 'conversation_info_destination.dart';
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

class HomePage extends StatefulWidget {
  const HomePage({super.key, this.initialProtocolUrl});

  final String? initialProtocolUrl;

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
    child: Portal(
      child: _HomeBody(initialProtocolUrl: widget.initialProtocolUrl),
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
  bool commandPaletteShowing = false;
  late final HomeNavigationController navigationController;
  Stream<List<CircleItem>>? circleChanges;

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

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    circleChanges ??= context.read<AccountHandle>().circleChanges();
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

  void _showActionFailure() {
    if (!mounted) return;
    showToastFailed(null);
  }

  void _selectCategory() {
    navigation.showChats();
  }

  void _selectConversation(ConversationListEntry conversation) {
    navigation.select(conversation);
  }

  void _selectSearchConversation(ConversationListEntry conversation) {
    context.read<ConversationListController>().setQuery('');
    _selectConversation(conversation);
  }

  Future<void> _selectSearchMessage(MessageListEntry message) async {
    final controller = context.read<ConversationListController>();
    final conversation = await controller.findConversation(
      message.conversationId,
    );
    if (!mounted || conversation == null) return;
    controller.setQuery('');
    _selectConversation(conversation);
    _locateMessage(message.id);
  }

  Future<void> _showCommandPalette(
    ConversationListController controller,
  ) async {
    if (commandPaletteShowing) return;
    commandPaletteShowing = true;
    await showCommandPalette(
      context: context,
      search: controller.searchCommandPalette,
      onSelected: (item) {
        final conversation = item.conversation;
        if (conversation != null) {
          _selectConversation(conversation);
        } else {
          unawaited(_showUser(item.id));
        }
      },
    );
    commandPaletteShowing = false;
  }

  Future<void> _openSearchUri(Uri uri) => openBotWebViewWindow(
    context: context,
    url: uri.toString(),
    title: '',
    conversationId: '',
    currency: context.read<AccountHandle>().profile().fiatCurrency,
  );

  void _navigateConversation(
    ConversationListController controller, {
    required bool forward,
  }) {
    if (settingsSelected || selectedConversation == null) return;
    final conversations = controller.visibleConversations;
    final index = conversations.indexWhere(
      (item) => item.id == selectedConversation!.id,
    );
    if (index < 0) return;
    final nextIndex = forward ? index + 1 : index - 1;
    if (nextIndex < 0 || nextIndex >= conversations.length) return;
    _selectConversation(conversations[nextIndex]);
    if (controller.itemScrollController.isAttached) {
      controller.itemScrollController.jumpTo(
        index: nextIndex,
        alignment: forward ? 0.9 : 0,
      );
    }
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

  Future<void> _performCreateAction(ConversationCreateAction action) async {
    switch (action) {
      case ConversationCreateAction.searchContact:
        await _searchContact();
      case ConversationCreateAction.conversation:
        final conversation = await showConversationSelector(
          context,
          account: context.read<AccountHandle>(),
          title: context.l10n.createConversation,
          category: ConversationCategoryFilter.contacts,
        );
        if (mounted && conversation != null) _selectConversation(conversation);
      case ConversationCreateAction.group:
        await _createGroup();
      case ConversationCreateAction.circle:
        await _createCircle();
    }
  }

  Future<void> _searchContact() async {
    final result = await showMixinDialog<MessageUserDialogResult>(
      context: context,
      child: const _SearchUserDialog(),
    );
    if (mounted && result != null) await _handleUserDialogResult(result);
  }

  Future<void> _searchUser(String query) async {
    try {
      final account = context.read<AccountHandle>();
      final profile = await account.user().searchUser(query: query);
      if (!mounted) return;
      await _showUser(profile.userId);
    } on Object catch (error, stackTrace) {
      e('Search user failed: query=$query', error, stackTrace);
      _showActionFailure();
    }
  }

  Future<void> _openLocalUser(UserProfileItem profile) async {
    await _showUser(profile.userId);
  }

  Future<void> _showUser(String userId) async {
    final account = context.read<AccountHandle>();
    final result = await showMessageUserDialog(
      context,
      account: account,
      userId: userId,
    );
    if (!mounted || result == null) return;
    await _handleUserDialogResult(result);
  }

  Future<void> _handleUserDialogResult(MessageUserDialogResult result) async {
    await handleMessageUserDialogResult(
      context,
      account: context.read<AccountHandle>(),
      result: result,
      onSelectConversation: _selectConversation,
      onSelectConversationInfo: (conversation) {
        _selectConversation(conversation);
        navigation.chatSideController.openDestination(
          ConversationInfoDestination.infoPage,
        );
      },
    );
  }

  Future<void> _createGroup() async {
    final selected = await showConversationMultiSelector(
      context,
      account: context.read<AccountHandle>(),
      title: context.l10n.createGroup,
    );
    if (!mounted || selected == null || selected.isEmpty) return;
    final name = await showMixinDialog<String>(
      context: context,
      child: _NewGroupConfirm(
        profile: context.read<AccountHandle>().profile(),
        selected: selected,
      ),
    );
    if (!mounted || name == null || name.isEmpty) return;
    try {
      final controller = context.read<ConversationListController>();
      await context.read<AccountHandle>().conversation().createGroup(
        name: name.trim(),
        userIds: selected.map((conversation) => conversation.ownerId).toList(),
      );
      await controller.refresh();
    } catch (error, stackTrace) {
      e('Create group failed', error, stackTrace);
      _showActionFailure();
    }
  }

  Future<void> _createCircle() async {
    final name = await showMixinDialog<String>(
      context: context,
      child: EditDialog(
        title: Text(context.l10n.circles),
        hintText: context.l10n.editCircleName,
        maxLength: 64,
      ),
    );
    if (!mounted || name == null || name.isEmpty) return;
    final selected = await showConversationMultiSelector(
      context,
      account: context.read<AccountHandle>(),
      title: context.l10n.createCircle,
      category: ConversationCategoryFilter.chats,
      allowEmpty: true,
    );
    if (!mounted || selected == null) return;
    try {
      final controller = context.read<ConversationListController>();
      final account = context.read<AccountHandle>();
      final circle = await account.conversation().createCircle(
        name: name.trim(),
      );
      for (final conversation in selected) {
        await account.conversation().editCircleConversation(
          circleId: circle.circleId,
          conversationId: conversation.id,
          ownerId: conversation.ownerId,
          isGroup: conversation.isGroup,
          add: true,
        );
      }
      await controller.refresh();
    } catch (error, stackTrace) {
      e('Create circle failed', error, stackTrace);
      _showActionFailure();
    }
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
    final controller = context.watch<ConversationListController>();
    final conversationList = StreamBuilder<List<CircleItem>>(
      stream: circleChanges,
      initialData: const [],
      builder: (context, snapshot) => ConversationListView(
        conversations: controller.visibleConversations,
        initialized: controller.initialized,
        itemPositionsListener: controller.itemPositionsListener,
        itemScrollController: controller.itemScrollController,
        loading: controller.loading,
        currentUserId: context.read<AccountHandle>().profile().userId,
        account: context.read<AccountHandle>(),
        circles: {
          for (final circle in snapshot.data!) circle.circleId: circle.name,
        },
        currentCircleId:
            controller.category == ConversationCategoryFilter.circle
            ? controller.circleId
            : null,
        query: controller.query,
        filterUnseen: controller.filterUnseen,
        selectedConversationId: selectedConversation?.id,
        onQueryChanged: controller.setQuery,
        searchMessages: controller.searchMessages,
        searchUsers: controller.searchUsers,
        searchMaoUser: controller.searchMaoUser,
        searchMao: controller.searchMao,
        searchMessageConversations: controller.searchMessageConversations,
        searchMessagesLoading: controller.searchMessagesLoading,
        onSearchMessageSelected: (message) =>
            unawaited(_selectSearchMessage(message)),
        onSearchUser: (query) => unawaited(_searchUser(query)),
        onLocalUserSelected: (profile) => unawaited(_openLocalUser(profile)),
        onMaoBotOpen: (profile) => unawaited(
          openProtocolUri(
            context,
            Uri.parse('mixin://apps/${profile.userId}?action=open'),
            onSelectConversation: navigation.select,
            currentConversation: selectedConversation,
          ),
        ),
        onOpenLink: (uri) => unawaited(_openSearchUri(uri)),
        onToggleUnseen: controller.toggleUnseen,
        onCreateActionSelected: (action) =>
            unawaited(_performCreateAction(action)),
        onSelected: controller.query.trim().isEmpty
            ? _selectConversation
            : _selectSearchConversation,
        onPinned: (conversation) async {
          try {
            await controller.account.conversation().setPinned(
              conversationId: conversation.id,
              pinned: !conversation.isPinned,
            );
          } catch (error, stackTrace) {
            e('Set conversation pinned state failed', error, stackTrace);
            _showActionFailure();
          }
        },
        onMuted: (conversation, duration) async {
          try {
            await controller.account.conversation().setMuted(
              conversationId: conversation.id,
              ownerId: conversation.ownerId,
              category: conversation.category,
              durationSeconds: duration,
            );
          } catch (error, stackTrace) {
            e('Set conversation mute state failed', error, stackTrace);
            _showActionFailure();
          }
        },
        onDeleted: (conversation) async {
          try {
            await controller.account.conversation().deleteConversation(
              conversationId: conversation.id,
            );
          } catch (error, stackTrace) {
            e('Delete conversation failed', error, stackTrace);
            _showActionFailure();
            return;
          }

          if (mounted && selectedConversation?.id == conversation.id) {
            navigation.clearSelection();
          }
        },
        onCircleChanged: (conversation, circleId, add) async {
          try {
            await controller.account.conversation().editCircleConversation(
              circleId: circleId,
              conversationId: conversation.id,
              ownerId: conversation.ownerId,
              isGroup: conversation.isGroup,
              add: add,
            );
          } catch (error, stackTrace) {
            e(
              'Update conversation circle membership failed',
              error,
              stackTrace,
            );
            _showActionFailure();
          }
        },
        audioPlayerBar: AudioPlayerBar(
          selectedConversationId: selectedConversation?.id,
          findConversation: controller.findConversation,
          onConversationSelected: _selectConversation,
        ),
        networkStatus: NetworkStatus(account: context.read<AccountHandle>()),
      ),
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

    final isMacOS = defaultTargetPlatform == TargetPlatform.macOS;
    final shortcutHome = CallbackShortcuts(
      bindings: {
        SingleActivator(
          LogicalKeyboardKey.keyK,
          meta: isMacOS,
          control: !isMacOS,
        ): () =>
            unawaited(_showCommandPalette(controller)),
        SingleActivator(
          LogicalKeyboardKey.arrowDown,
          meta: isMacOS,
          control: !isMacOS,
        ): () =>
            _navigateConversation(controller, forward: true),
        SingleActivator(
          LogicalKeyboardKey.arrowUp,
          meta: isMacOS,
          control: !isMacOS,
        ): () =>
            _navigateConversation(controller, forward: false),
        if (isMacOS) ...{
          const SingleActivator(LogicalKeyboardKey.comma, meta: true):
              _selectProfile,
          const SingleActivator(
            LogicalKeyboardKey.keyL,
            meta: true,
            shift: true,
          ): context
              .read<SecurityController>()
              .lockNow,
          const SingleActivator(LogicalKeyboardKey.keyN, meta: true): () =>
              unawaited(
                _performCreateAction(ConversationCreateAction.conversation),
              ),
          const SingleActivator(
            LogicalKeyboardKey.keyN,
            meta: true,
            shift: true,
          ): () =>
              unawaited(_performCreateAction(ConversationCreateAction.group)),
          const SingleActivator(LogicalKeyboardKey.keyM, meta: true): () =>
              unawaited(windowManager.minimize()),
        },
      },
      child: Focus(
        autofocus: true,
        child: LayoutBuilder(
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
                              draft:
                                  drafts[conversation.id] ?? conversation.draft,
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
                              SizedBox(
                                width: kConversationListWidth,
                                child: conversationPane,
                              ),
                              Expanded(
                                child: conversation == null
                                    ? ColoredBox(
                                        color:
                                            context.mixinTheme.chatBackground,
                                        child: Center(
                                          child: Text(
                                            context.l10n.pickAConversation,
                                            style: TextStyle(
                                              color: context
                                                  .mixinTheme
                                                  .secondaryText,
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
                                        onSelectConversation:
                                            _selectConversation,
                                        onConversationDeleted:
                                            _conversationDeleted,
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
        ),
      ),
    );
    final home = MacosMenuBar(child: shortcutHome);
    final content = _ProfileSetupGate(
      account: context.read<AccountHandle>(),
      child: home,
    );
    final account = context.read<AccountHandle>();
    return ChangeNotifierProvider.value(
      value: navigationController,
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

class _SearchIntent extends Intent {
  const _SearchIntent();
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

class _SearchUserDialog extends HookWidget {
  const _SearchUserDialog();

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();
    useListenable(controller);
    final profile = useState<UserProfileItem?>(null);
    final loading = useState(false);
    final account = context.read<AccountHandle>();
    final searchable = controller.text.trim().length > 3;
    final currentIdentityNumber = account.profile().identityNumber;

    Future<void> search() async {
      if (!searchable || loading.value) return;
      loading.value = true;
      try {
        profile.value = await account.user().searchUser(query: controller.text);
      } on Object catch (error, stackTrace) {
        e('Search user dialog failed', error, stackTrace);
        if (context.mounted) {
          showToastFailed(ToastError(context.l10n.userNotFound));
        }
      } finally {
        if (context.mounted) loading.value = false;
      }
    }

    return AnimatedSize(
      duration: const Duration(milliseconds: 200),
      child: profile.value != null
          ? MessageUserDialog(
              account: account,
              profile: Future.value(profile.value),
            )
          : Stack(
              children: [
                Visibility(
                  visible: !loading.value,
                  maintainSize: true,
                  maintainAnimation: true,
                  maintainState: true,
                  child: AlertDialogLayout(
                    title: Text(context.l10n.addContact),
                    content: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        FocusableActionDetector(
                          shortcuts: {
                            if (searchable)
                              const SingleActivator(LogicalKeyboardKey.enter):
                                  const _SearchIntent(),
                          },
                          actions: {
                            _SearchIntent: CallbackAction<_SearchIntent>(
                              onInvoke: (_) => search(),
                            ),
                          },
                          child: DialogTextField(
                            textEditingController: controller,
                            hintText: context.l10n.addPeopleSearchHint,
                            inputFormatters: [
                              FilteringTextInputFormatter.allow(
                                RegExp('[0-9+]'),
                              ),
                              LengthLimitingTextInputFormatter(128),
                            ],
                          ),
                        ),
                        if (currentIdentityNumber.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(top: 8),
                            child: Text(
                              context.l10n.myMixinId(
                                currentIdentityNumber,
                              ),
                              style: TextStyle(
                                color: context.theme.secondaryText,
                                fontSize: 12,
                              ),
                            ),
                          ),
                      ],
                    ),
                    actions: [
                      MixinButton(
                        backgroundTransparent: true,
                        onTap: () => Navigator.pop(context),
                        child: Text(context.l10n.cancel),
                      ),
                      MixinButton(
                        disable: !searchable,
                        onTap: search,
                        child: Text(context.l10n.search),
                      ),
                    ],
                  ),
                ),
                if (loading.value)
                  Positioned.fill(
                    child: Center(
                      child: CircularProgressIndicator(
                        color: context.theme.accent,
                      ),
                    ),
                  ),
              ],
            ),
    );
  }
}

class _NewGroupConfirm extends HookWidget {
  const _NewGroupConfirm({required this.profile, required this.selected});

  final AccountProfile profile;
  final List<ConversationListEntry> selected;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();
    useListenable(controller);
    final avatars = [
      ConversationAvatarEntry(
        userId: profile.userId,
        name: profile.fullName,
        avatarUrl: profile.avatarUrl,
      ),
      ...selected.map(
        (conversation) => ConversationAvatarEntry(
          userId: conversation.ownerId,
          name: conversation.name,
          avatarUrl: conversation.avatarUrl,
        ),
      ),
    ].take(4).toList(growable: false);
    return AlertDialogLayout(
      title: Text(context.l10n.groups),
      titleMarginBottom: 24,
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          ClipOval(child: AvatarPuzzlesView(avatars: avatars, size: 60)),
          const SizedBox(height: 8),
          Text(
            context.l10n.participantsCount(selected.length + 1),
            style: TextStyle(fontSize: 14, color: context.theme.secondaryText),
          ),
          const SizedBox(height: 48),
          DialogTextField(
            textEditingController: controller,
            hintText: context.l10n.groupName,
            maxLength: 40,
          ),
        ],
      ),
      actions: [
        MixinButton(
          backgroundTransparent: true,
          onTap: () => Navigator.pop(context),
          child: Text(context.l10n.cancel),
        ),
        MixinButton(
          disable: controller.text.isEmpty,
          onTap: () => Navigator.pop(context, controller.text),
          child: Text(context.l10n.create),
        ),
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
