import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:window_manager/window_manager.dart';

import '../controllers/app_controller.dart';
import '../controllers/chat_side_notifier.dart';
import '../controllers/conversation_list_controller.dart';
import '../controllers/device_transfer_controller.dart';
import '../controllers/security_controller.dart';
import '../controllers/settings_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import '../utils/local_notification_center.dart';
import '../utils/mixin_uri.dart';
import '../utils/web_view.dart';
import '../widgets/account_health_overlays.dart';
import '../widgets/app_protocol_handler.dart';
import '../widgets/audio_player_bar.dart';
import '../widgets/avatar_view.dart';
import '../widgets/chat_view.dart';
import '../widgets/command_palette.dart';
import '../widgets/conversation_list_view.dart';
import '../widgets/device_transfer_dialog.dart';
import '../widgets/device_transfer_widget.dart';
import '../widgets/home_sidebar.dart';
import '../widgets/mixin_dialog.dart';
import '../widgets/mute_dialog.dart';
import '../widgets/network_status.dart';
import '../widgets/show_conversation_code_dialog.dart';
import '../widgets/show_forward_conversation_selector.dart';
import '../widgets/show_message_user_dialog.dart';
import '../widgets/show_multisigs_payment_dialog.dart';
import '../widgets/show_send_message_dialog.dart';
import '../widgets/show_snapshot_detail_dialog.dart';
import '../widgets/toast.dart';
import '../widgets/unknown_mixin_url_dialog.dart';
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
  static const _platformMenusChannel = MethodChannel(
    'mixin_desktop/platform_menus',
  );
  ConversationListEntry? selectedConversation;
  final drafts = <String, String>{};
  final chatSideController = ChatSideNotifier();
  bool userCollapsed = false;
  bool settingsSelected = false;
  bool commandPaletteShowing = false;
  String? locateMessageId;
  int locateRequest = 0;
  StreamSubscription<NotificationEvent>? notificationEventSubscription;
  StreamSubscription<Uri>? notificationSelectionSubscription;
  DeviceTransferController? deviceTransferController;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (notificationEventSubscription != null) return;
    final account = context.read<AccountHandle>();
    deviceTransferController = DeviceTransferController(account);
    notificationEventSubscription = account.desktopNotificationEvents().listen(
      (event) => unawaited(_showMessageNotification(event)),
      onError: (_) {},
    );
    notificationSelectionSubscription = notificationSelections.listen((uri) {
      unawaited(_selectNotification(uri));
    });
  }

  Future<void> _showMessageNotification(NotificationEvent message) async {
    final dismissMessageId = message.dismissMessageId;
    if (dismissMessageId != null) {
      await dismissMessageNotification(dismissMessageId);
      return;
    }
    final createdAt = DateTime.fromMicrosecondsSinceEpoch(
      message.createdAtMicros,
    );
    if (createdAt.isBefore(
      DateTime.now().subtract(const Duration(minutes: 2)),
    )) {
      return;
    }
    final appActive =
        WidgetsBinding.instance.lifecycleState == null ||
        WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed;
    if (appActive && selectedConversation?.id == message.conversationId) return;

    final showPreview = context.read<SettingsController>().messagePreview;
    final preview = showPreview
        ? notificationPreview(context, message)
        : context.l10n.aMessage;
    await showMessageNotification(
      title: message.conversationName,
      body: preview,
      uri: Uri(
        scheme: 'mixin',
        host: 'conversations',
        path: message.conversationId,
        queryParameters: {'message': message.messageId},
      ),
      conversationId: message.conversationId,
      messageId: message.messageId,
    );
  }

  Future<void> _selectNotification(Uri uri) async {
    await windowManager.show();
    await windowManager.focus();
    if (!mounted) return;
    await _openProtocolUri(uri);
    final messageId = uri.queryParameters['message'];
    if (messageId?.isNotEmpty == true && mounted) _locateMessage(messageId!);
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
    setState(() {
      settingsSelected = false;
      selectedConversation = null;
    });
    chatSideController.clear();
  }

  void _selectConversation(ConversationListEntry conversation) {
    context.read<ConversationListController>().recordRecentConversation(
      conversation.id,
    );
    setState(() {
      settingsSelected = false;
      selectedConversation = conversation;
      locateMessageId = null;
    });
    chatSideController.clear();
    unawaited(dismissConversationNotifications(conversation.id));
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

  Future<void> _openProtocolUri(Uri uri) async {
    if (!uri.isMixin) {
      await launchUrl(uri);
      return;
    }

    final userId = uri.userId;
    if (userId?.trim().isNotEmpty == true) {
      await _showUser(userId!);
      return;
    }

    final conversationId = uri.conversationId;
    if (conversationId?.trim().isNotEmpty == true) {
      final controller = context.read<ConversationListController>();
      final conversation = await controller.findConversation(conversationId!);
      if (!mounted) return;
      if (conversation == null) {
        _showActionFailure();
        return;
      }
      _selectConversation(conversation);
      final startText = uri.startTextOfConversation;
      if (startText?.trim().isNotEmpty == true) {
        try {
          await context.read<AccountHandle>().message().sendText(
            conversationId: conversationId,
            content: startText!,
            silent: false,
          );
        } catch (_) {
          if (mounted) _showActionFailure();
        }
      }
      return;
    }

    final code = uri.code;
    if (code?.trim().isNotEmpty == true) {
      try {
        final account = context.read<AccountHandle>();
        final result = await account.conversation().resolveCode(code: code!);
        if (!mounted) return;
        if (result.kind == 'user' && result.userId != null) {
          await _showUser(result.userId!);
          return;
        }
        if (result.kind == 'conversation' && result.conversationId != null) {
          String? conversationId;
          if (result.alreadyMember) {
            showToast(context.l10n.groupAlreadyIn);
            conversationId = result.conversationId;
          } else {
            conversationId = await showConversationCodeDialog(
              context,
              account: account,
              result: result,
              code: code,
            );
          }
          if (!mounted || conversationId == null) return;
          final controller = context.read<ConversationListController>();
          await controller.refresh();
          final conversation = await controller.findConversation(
            conversationId,
          );
          if (mounted && conversation != null) {
            _selectConversation(conversation);
          }
          return;
        }
        if (result.kind == 'payment' || result.kind == 'multisig_request') {
          await showMultisigsPaymentDialog(context, item: result, uri: uri);
          return;
        }
      } on Object {
        if (mounted) _showActionFailure();
        return;
      }
    }

    final snapshotTraceId = uri.snapshotTraceId?.trim();
    if (snapshotTraceId?.isNotEmpty == true) {
      try {
        final snapshot = await context.read<AccountHandle>().snapshotByTrace(
          traceId: snapshotTraceId!,
        );
        if (!mounted) return;
        await showSnapshotDetailItemDialog(context, snapshot: snapshot);
      } on Object {
        if (mounted) _showActionFailure();
      }
      return;
    }

    if (uri.isSend) {
      final handled = await showSendMessageDialog(
        context,
        account: context.read<AccountHandle>(),
        category: uri.categoryOfSend,
        conversationId: uri.conversationIdOfSend,
        data: uri.dataOfSend,
        userId: uri.userOfSend,
        currentConversation: selectedConversation,
        onSelectConversation: _selectConversation,
      );
      if (!handled && mounted) _showActionFailure();
      return;
    }

    if (uri.isPay ||
        uri.isMultisigs ||
        uri.isSwap ||
        uri.isMarkets ||
        uri.isMembership) {
      await showUnknownMixinUrlDialog(context, uri);
      return;
    }

    final appId = uri.appId;
    if (appId?.trim().isNotEmpty == true) {
      if (!uri.actionIsOpen) {
        await _showUser(appId!);
        return;
      }
      final account = context.read<AccountHandle>();
      late final List<Object?> results;
      try {
        results = await Future.wait<Object?>([
          account.user().botHomeUri(appId: appId!),
          account.user().userProfile(userId: appId),
        ]);
      } on Object {
        if (mounted) {
          showToastFailed(ToastError(context.l10n.botNotFound));
        }
        return;
      }
      if (!mounted) return;
      final homeUri = Uri.tryParse(results.first as String? ?? '');
      if (homeUri == null) {
        showToastFailed(ToastError(context.l10n.botNotFound));
        return;
      }
      final queryParameters = {...homeUri.queryParameters}
        ..addAll({...uri.queryParameters}..remove('action'));
      final profile = results.last as UserProfileItem?;
      await openBotWebViewWindow(
        context: context,
        url: homeUri.replace(queryParameters: queryParameters).toString(),
        title: profile?.fullName ?? '',
        conversationId: conversationId ?? '',
        currency: account.profile().fiatCurrency,
      );
      return;
    }

    if (uri.isMixinScheme) await showUnknownMixinUrlDialog(context, uri);
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
    setState(() {
      locateMessageId = messageId;
      locateRequest++;
    });
  }

  void _conversationDeleted() {
    setState(() {
      selectedConversation = null;
      locateMessageId = null;
    });
  }

  void _selectProfile() {
    setState(() {
      settingsSelected = true;
      selectedConversation = null;
    });
    chatSideController.clear();
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

  Future<void> _muteConversationFromMenu(
    ConversationListController controller,
    ConversationListEntry conversation,
  ) async {
    final duration = conversation.isMuted ? 0 : await showMuteDialog(context);
    if (duration == null) return;
    try {
      await controller.setMuted(conversation, duration);
    } catch (_) {
      _showActionFailure();
    }
  }

  Future<void> _deleteConversationFromMenu(
    ConversationListController controller,
    ConversationListEntry conversation,
  ) async {
    final confirmed = await showConfirmMixinDialog(
      context,
      context.l10n.conversationDeleteTitle(conversation.name),
      description: context.l10n.deleteChatDescription,
      positiveText: context.l10n.delete,
    );
    if (confirmed != DialogEvent.positive) return;
    try {
      await controller.deleteConversation(conversation);
      if (mounted) _conversationDeleted();
    } catch (_) {
      _showActionFailure();
    }
  }

  List<PlatformMenu> _macMenus(ConversationListController controller) {
    final conversation = selectedConversation;
    return [
      PlatformMenu(
        label: 'Mixin',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: '${context.l10n.about} Mixin',
                onSelected: () =>
                    _platformMenusChannel.invokeMethod<void>('showAbout'),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.preferences,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.comma,
                  meta: true,
                ),
                onSelected: _selectProfile,
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.lock,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyL,
                  meta: true,
                  shift: true,
                ),
                onSelected: context.read<SecurityController>().hasPasscode
                    ? context.read<SecurityController>().lockNow
                    : null,
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.quickSearch,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyK,
                  meta: true,
                ),
                onSelected: () => unawaited(_showCommandPalette(controller)),
              ),
              PlatformMenuItem(
                label: context.l10n.hideMixin,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyH,
                  meta: true,
                ),
                onSelected: windowManager.hide,
              ),
              PlatformMenuItem(
                label: context.l10n.showMixin,
                onSelected: windowManager.show,
              ),
            ],
          ),
          PlatformMenuItem(
            label: context.l10n.quitMixin,
            shortcut: const SingleActivator(
              LogicalKeyboardKey.keyQ,
              meta: true,
            ),
            onSelected: () => exit(0),
          ),
        ],
      ),
      PlatformMenu(
        label: context.l10n.file,
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.createConversation,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyN,
                  meta: true,
                ),
                onSelected: () => unawaited(
                  _performCreateAction(ConversationCreateAction.conversation),
                ),
              ),
              PlatformMenuItem(
                label: context.l10n.createGroup,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyN,
                  meta: true,
                  shift: true,
                ),
                onSelected: () => unawaited(
                  _performCreateAction(ConversationCreateAction.group),
                ),
              ),
              if (kDebugMode)
                PlatformMenuItemGroup(
                  members: [
                    PlatformMenuItem(
                      label: 'chat backup and restore',
                      onSelected: () => showDeviceTransferDialog(
                        context,
                        deviceTransferController!,
                      ),
                    ),
                  ],
                ),
              PlatformMenuItem(
                label: context.l10n.createCircle,
                onSelected: () => unawaited(
                  _performCreateAction(ConversationCreateAction.circle),
                ),
              ),
              PlatformMenuItemGroup(
                members: [
                  PlatformMenuItem(
                    label: context.l10n.closeWindow,
                    onSelected: windowManager.close,
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
      PlatformMenu(
        label: context.l10n.conversation,
        menus: [
          PlatformMenuItem(
            label: conversation?.isMuted == true
                ? context.l10n.unmute
                : context.l10n.mute,
            onSelected: conversation == null
                ? null
                : () => unawaited(
                    _muteConversationFromMenu(controller, conversation),
                  ),
          ),
          PlatformMenuItem(
            label: context.l10n.search,
            onSelected: conversation == null
                ? null
                : () => chatSideController.toggleDestination(
                    ConversationInfoDestination.searchMessageHistory,
                  ),
          ),
          PlatformMenuItem(
            label: context.l10n.deleteChat,
            onSelected: conversation == null
                ? null
                : () => unawaited(
                    _deleteConversationFromMenu(controller, conversation),
                  ),
          ),
          PlatformMenuItem(
            label: conversation?.isPinned == true
                ? context.l10n.unpin
                : context.l10n.pinTitle,
            onSelected: conversation == null
                ? null
                : () => unawaited(controller.setPinned(conversation)),
          ),
          PlatformMenuItem(
            label: context.l10n.toggleChatInfo,
            onSelected: conversation == null
                ? null
                : chatSideController.toggleInfoPage,
          ),
        ],
      ),
      PlatformMenu(
        label: context.l10n.window,
        menus: [
          PlatformMenuItem(
            label: context.l10n.minimize,
            shortcut: const SingleActivator(
              LogicalKeyboardKey.keyM,
              meta: true,
            ),
            onSelected: windowManager.minimize,
          ),
          PlatformMenuItem(
            label: context.l10n.zoom,
            onSelected: () async => await windowManager.isMaximized()
                ? windowManager.restore()
                : windowManager.maximize(),
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.previousConversation,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.arrowUp,
                  meta: true,
                ),
                onSelected: conversation == null
                    ? null
                    : () => _navigateConversation(controller, forward: false),
              ),
              PlatformMenuItem(
                label: context.l10n.nextConversation,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.arrowDown,
                  meta: true,
                ),
                onSelected: conversation == null
                    ? null
                    : () => _navigateConversation(controller, forward: true),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Mixin',
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyO,
                  meta: true,
                ),
                onSelected: windowManager.show,
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.bringAllToFront,
                onSelected: windowManager.show,
              ),
            ],
          ),
          PlatformMenuItem(label: 'Mixin', onSelected: windowManager.show),
        ],
      ),
      PlatformMenu(
        label: context.l10n.help,
        menus: [
          PlatformMenuItem(
            label: context.l10n.helpCenter,
            onSelected: () =>
                unawaited(launchUrl(Uri.parse('https://support.mixin.one/'))),
          ),
          PlatformMenuItem(
            label: context.l10n.termsOfService,
            onSelected: () => unawaited(
              launchUrl(Uri.parse('https://mixin.one/pages/terms')),
            ),
          ),
          PlatformMenuItem(
            label: context.l10n.privacyPolicy,
            onSelected: () => unawaited(
              launchUrl(Uri.parse('https://mixin.one/pages/privacy')),
            ),
          ),
        ],
      ),
    ];
  }

  Future<void> _searchContact() async {
    final controller = context.read<ConversationListController>();
    final result = await showMixinDialog<MessageUserDialogResult>(
      context: context,
      child: _SearchUserDialog(
        account: context.read<AccountHandle>(),
        currentIdentityNumber: controller.profile.identityNumber,
      ),
    );
    if (mounted && result != null) await _handleUserDialogResult(result);
  }

  Future<void> _searchUser(String query) async {
    try {
      final account = context.read<AccountHandle>();
      final profile = await account.user().searchUser(query: query);
      if (!mounted) return;
      await _showUser(profile.userId);
    } on Object {
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
        chatSideController.openDestination(
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
        profile: context.read<ConversationListController>().profile,
        selected: selected,
      ),
    );
    if (!mounted || name == null || name.isEmpty) return;
    try {
      await context.read<ConversationListController>().createGroup(
        name,
        selected.map((conversation) => conversation.ownerId).toList(),
      );
    } catch (_) {
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
      final circle = await controller.createCircle(name);
      for (final conversation in selected) {
        await controller.editCircle(conversation, circle.circleId, true);
      }
      await controller.refresh();
    } catch (_) {
      _showActionFailure();
    }
  }

  @override
  void dispose() {
    unawaited(notificationEventSubscription?.cancel());
    unawaited(notificationSelectionSubscription?.cancel());
    unawaited(deviceTransferController?.dispose());
    chatSideController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<ConversationListController>();
    final conversationList = ConversationListView(
      conversations: controller.visibleConversations,
      initialized: controller.initialized,
      itemPositionsListener: controller.itemPositionsListener,
      itemScrollController: controller.itemScrollController,
      loading: controller.loading,
      currentUserId: controller.profile.userId,
      account: context.read<AccountHandle>(),
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
        _openProtocolUri(
          Uri.parse('mixin://apps/${profile.userId}?action=open'),
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
      audioPlayerBar: AudioPlayerBar(
        selectedConversationId: selectedConversation?.id,
        findConversation: controller.findConversation,
        onConversationSelected: _selectConversation,
      ),
      networkStatus: NetworkStatus(account: context.read<AccountHandle>()),
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
                            return SettingsPage(
                              profile: controller.profile,
                              onSignOut: context.read<AppController>().signOut,
                              onProfileUpdated: controller.updateProfile,
                              onProfileRefresh: controller.refreshProfile,
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
                              notifier: chatSideController,
                              draft:
                                  drafts[conversation.id] ?? conversation.draft,
                              locateMessageId: locateMessageId,
                              locateRequest: locateRequest,
                              onDraftChanged: (value) =>
                                  _updateDraft(conversation, value),
                              onBack: () =>
                                  setState(() => selectedConversation = null),
                              onLocateMessage: _locateMessage,
                              onSelectConversation: _selectConversation,
                              onOpenUri: (uri) =>
                                  unawaited(_openProtocolUri(uri)),
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
                                        notifier: chatSideController,
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
                                        onOpenUri: (uri) =>
                                            unawaited(_openProtocolUri(uri)),
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
    final home = isMacOS
        ? PlatformMenuBar(menus: _macMenus(controller), child: shortcutHome)
        : shortcutHome;
    final content = controller.profile.fullName.trim().isNotEmpty
        ? home
        : Stack(
            fit: StackFit.expand,
            children: [
              home,
              _SetupNameOverlay(controller: controller),
            ],
          );
    final account = context.read<AccountHandle>();
    return DeviceTransferHandlerWidget(
      controller: deviceTransferController!,
      child: AccountHealthOverlays(
        account: account,
        child: AppProtocolHandler(
          initialUrl: widget.initialProtocolUrl,
          onUri: (uri) => unawaited(_openProtocolUri(uri)),
          child: content,
        ),
      ),
    );
  }
}

class _SearchIntent extends Intent {
  const _SearchIntent();
}

class _SearchUserDialog extends StatefulWidget {
  const _SearchUserDialog({
    required this.account,
    required this.currentIdentityNumber,
  });

  final AccountHandle account;
  final String currentIdentityNumber;

  @override
  State<_SearchUserDialog> createState() => _SearchUserDialogState();
}

class _SearchUserDialogState extends State<_SearchUserDialog> {
  final controller = TextEditingController();
  UserProfileItem? profile;
  bool loading = false;

  bool get searchable => controller.text.trim().length > 3;

  @override
  void initState() {
    super.initState();
    controller.addListener(_changed);
  }

  void _changed() => setState(() {});

  Future<void> _search() async {
    if (!searchable || loading) return;
    setState(() => loading = true);
    try {
      final result = await widget.account.user().searchUser(
        query: controller.text,
      );
      if (mounted) setState(() => profile = result);
    } on Object {
      if (mounted) {
        showToastFailed(ToastError(context.l10n.userNotFound));
      }
    } finally {
      if (mounted) setState(() => loading = false);
    }
  }

  @override
  void dispose() {
    controller
      ..removeListener(_changed)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedSize(
    duration: const Duration(milliseconds: 200),
    child: profile != null
        ? MessageUserDialog(
            account: widget.account,
            profile: Future.value(profile),
          )
        : Stack(
            children: [
              Visibility(
                visible: !loading,
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
                            onInvoke: (_) => _search(),
                          ),
                        },
                        child: DialogTextField(
                          textEditingController: controller,
                          hintText: context.l10n.addPeopleSearchHint,
                          inputFormatters: [
                            FilteringTextInputFormatter.allow(RegExp('[0-9+]')),
                            LengthLimitingTextInputFormatter(128),
                          ],
                        ),
                      ),
                      if (widget.currentIdentityNumber.isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.only(top: 8),
                          child: Text(
                            context.l10n.myMixinId(
                              widget.currentIdentityNumber,
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
                      onTap: _search,
                      child: Text(context.l10n.search),
                    ),
                  ],
                ),
              ),
              if (loading)
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

class _NewGroupConfirm extends StatefulWidget {
  const _NewGroupConfirm({required this.profile, required this.selected});

  final AccountProfile profile;
  final List<ConversationListEntry> selected;

  @override
  State<_NewGroupConfirm> createState() => _NewGroupConfirmState();
}

class _NewGroupConfirmState extends State<_NewGroupConfirm> {
  final controller = TextEditingController();

  @override
  void initState() {
    super.initState();
    controller.addListener(_changed);
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    controller
      ..removeListener(_changed)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final avatars = [
      ConversationAvatarEntry(
        userId: widget.profile.userId,
        name: widget.profile.fullName,
        avatarUrl: widget.profile.avatarUrl,
      ),
      ...widget.selected.map(
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
            context.l10n.participantsCount(widget.selected.length + 1),
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
  const _SetupNameOverlay({required this.controller});

  final ConversationListController controller;

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
      await widget.controller.updateProfile(
        name,
        widget.controller.profile.biography,
      );
    } on Object catch (error) {
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

String notificationPreview(BuildContext context, NotificationEvent message) {
  final category = message.category;
  if (category == 'MESSAGE_PIN') {
    return _pinNotificationPreview(context, message);
  }
  final text = _notificationContentPreview(context, category, message.content);
  return message.conversationCategory == 'GROUP'
      ? '${message.senderName}: $text'
      : text;
}

String _pinNotificationPreview(
  BuildContext context,
  NotificationEvent message,
) {
  try {
    final value = jsonDecode(message.content) as Map<String, dynamic>;
    final category = value['category']?.toString();
    if (category == null) throw const FormatException('missing pin category');
    final content = value['content']?.toString() ?? '';
    return context.l10n.chatPinMessage(
      message.senderName,
      _notificationContentPreview(context, category, content),
    );
  } on Object {
    return context.l10n.chatPinMessage(
      message.senderName,
      context.l10n.aMessage,
    );
  }
}

String _notificationContentPreview(
  BuildContext context,
  String category,
  String content,
) {
  String text;
  if (category.contains('TEXT')) {
    text = content.trim();
  } else if (category.contains('SNAPSHOT')) {
    text = '[${context.l10n.transfer}]';
  } else if (category.contains('STICKER')) {
    text = '[${context.l10n.sticker}]';
  } else if (category.contains('IMAGE')) {
    text = '[${context.l10n.image}]';
  } else if (category.contains('VIDEO')) {
    text = '[${context.l10n.video}]';
  } else if (category.contains('LIVE')) {
    text = '[${context.l10n.live}]';
  } else if (category.contains('DATA')) {
    text = '[${context.l10n.file}]';
  } else if (category.contains('POST')) {
    text = content.trim().isEmpty ? context.l10n.post : content.trim();
  } else if (category.contains('LOCATION')) {
    text = '[${context.l10n.location}]';
  } else if (category.contains('AUDIO')) {
    text = '[${context.l10n.audio}]';
  } else if (category.contains('CONTACT')) {
    text = '[${context.l10n.contact}]';
  } else if (category.contains('TRANSCRIPT')) {
    text = '[${context.l10n.transcript}]';
  } else if (category.contains('INSCRIPTION')) {
    text = '[${context.l10n.collectible}]';
  } else if (category == 'APP_BUTTON_GROUP') {
    text = _appButtonGroupPreview(content);
  } else if (category == 'APP_CARD') {
    text = _appCardPreview(context, content);
  } else {
    text = context.l10n.messageNotSupport;
  }
  return text;
}

String _appButtonGroupPreview(String content) {
  try {
    return (jsonDecode(content) as List<dynamic>)
        .map((item) => '[${(item as Map<String, dynamic>)['label'] ?? ''}]')
        .join();
  } on Object {
    return '';
  }
}

String _appCardPreview(BuildContext context, String content) {
  try {
    final value = jsonDecode(content) as Map<String, dynamic>;
    return '[${value['title']?.toString() ?? context.l10n.card}]';
  } on Object {
    return '[${context.l10n.card}]';
  }
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
    required this.onOpenUri,
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
  final ValueChanged<Uri> onOpenUri;
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
                conversation: conversation,
                draft: draft,
                locateMessageId: locateMessageId,
                locateRequest: locateRequest,
                onDraftChanged: onDraftChanged,
                onSelectConversation: onSelectConversation,
                onOpenUri: onOpenUri,
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
                  onOpenUri: onOpenUri,
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
