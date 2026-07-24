import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:window_manager/window_manager.dart';

import '../controllers/conversation_list_controller.dart';
import '../controllers/home_navigation_controller.dart';
import '../controllers/security_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../pages/conversation_info_destination.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import 'avatar_view.dart';
import 'command_palette.dart';
import 'device_transfer_dialog.dart';
import 'mixin_dialog.dart';
import 'mute_dialog.dart';
import 'show_forward_conversation_selector.dart';
import 'show_message_user_dialog.dart';
import 'toast.dart';

class MacosMenuBar extends StatefulWidget {
  const MacosMenuBar({required this.child, super.key});

  final Widget child;

  @override
  State<MacosMenuBar> createState() => _MacosMenuBarState();
}

class _MacosMenuBarState extends State<MacosMenuBar> {
  static const _platformMenusChannel = MethodChannel(
    'mixin_desktop/platform_menus',
  );
  bool commandPaletteShowing = false;

  ConversationListEntry? get selectedConversation =>
      context.watch<HomeNavigationController>().selectedConversation;

  HomeNavigationController get navigation =>
      context.read<HomeNavigationController>();

  @override
  Widget build(BuildContext context) {
    if (defaultTargetPlatform != TargetPlatform.macOS) return widget.child;
    return PlatformMenuBar(menus: _macMenus(), child: widget.child);
  }

  List<PlatformMenu> _macMenus() {
    final controller = context.read<ConversationListController>();
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
                onSelected: navigation.showSettings,
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
                onSelected: () => unawaited(_createConversation()),
              ),
              PlatformMenuItem(
                label: context.l10n.createGroup,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyN,
                  meta: true,
                  shift: true,
                ),
                onSelected: () => unawaited(_createGroup()),
              ),
              if (kDebugMode)
                PlatformMenuItemGroup(
                  members: [
                    PlatformMenuItem(
                      label: 'chat backup and restore',
                      onSelected: () => showDeviceTransferDialog(context),
                    ),
                  ],
                ),
              PlatformMenuItem(
                label: context.l10n.createCircle,
                onSelected: () => unawaited(_createCircle()),
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
                : () => unawaited(_muteConversation(conversation)),
          ),
          PlatformMenuItem(
            label: context.l10n.search,
            onSelected: conversation == null
                ? null
                : () => navigation.chatSideController.toggleDestination(
                    ConversationInfoDestination.searchMessageHistory,
                  ),
          ),
          PlatformMenuItem(
            label: context.l10n.deleteChat,
            onSelected: conversation == null
                ? null
                : () => unawaited(_deleteConversation(conversation)),
          ),
          PlatformMenuItem(
            label: conversation?.isPinned == true
                ? context.l10n.unpin
                : context.l10n.pinTitle,
            onSelected: conversation == null
                ? null
                : () => unawaited(
                    context.read<ConversationListController>().setPinned(
                      conversation,
                    ),
                  ),
          ),
          PlatformMenuItem(
            label: context.l10n.toggleChatInfo,
            onSelected: conversation == null
                ? null
                : navigation.chatSideController.toggleInfoPage,
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
          navigation.select(conversation);
        } else {
          unawaited(_showUser(item.id));
        }
      },
    );
    commandPaletteShowing = false;
  }

  Future<void> _showUser(String userId) async {
    final result = await showMessageUserDialog(
      context,
      account: context.read<AccountHandle>(),
      userId: userId,
    );
    if (!mounted || result == null) return;
    await handleMessageUserDialogResult(
      context,
      account: context.read<AccountHandle>(),
      result: result,
      onSelectConversation: navigation.select,
      onSelectConversationInfo: (conversation) {
        navigation.select(conversation);
        navigation.chatSideController.openDestination(
          ConversationInfoDestination.infoPage,
        );
      },
    );
  }

  Future<void> _createConversation() async {
    final conversation = await showConversationSelector(
      context,
      account: context.read<AccountHandle>(),
      title: context.l10n.createConversation,
      category: ConversationCategoryFilter.contacts,
    );
    if (mounted && conversation != null) navigation.select(conversation);
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
      for (final item in selected) {
        await controller.editCircle(item, circle.circleId, true);
      }
      await controller.refresh();
    } catch (_) {
      _showActionFailure();
    }
  }

  Future<void> _muteConversation(ConversationListEntry conversation) async {
    final duration = conversation.isMuted ? 0 : await showMuteDialog(context);
    if (duration == null) return;
    try {
      await context.read<ConversationListController>().setMuted(
        conversation,
        duration,
      );
    } catch (_) {
      _showActionFailure();
    }
  }

  Future<void> _deleteConversation(ConversationListEntry conversation) async {
    final confirmed = await showConfirmMixinDialog(
      context,
      context.l10n.conversationDeleteTitle(conversation.name),
      description: context.l10n.deleteChatDescription,
      positiveText: context.l10n.delete,
    );
    if (confirmed != DialogEvent.positive) return;
    try {
      await context.read<ConversationListController>().deleteConversation(
        conversation,
      );
      if (mounted) navigation.conversationDeleted();
    } catch (_) {
      _showActionFailure();
    }
  }

  void _navigateConversation(
    ConversationListController controller, {
    required bool forward,
  }) {
    final conversation = selectedConversation;
    if (navigation.settingsSelected || conversation == null) return;
    final conversations = controller.visibleConversations;
    final index = conversations.indexWhere(
      (item) => item.id == conversation.id,
    );
    if (index < 0) return;
    final nextIndex = forward ? index + 1 : index - 1;
    if (nextIndex < 0 || nextIndex >= conversations.length) return;
    navigation.select(conversations[nextIndex]);
    if (controller.itemScrollController.isAttached) {
      controller.itemScrollController.jumpTo(
        index: nextIndex,
        alignment: forward ? 0.9 : 0,
      );
    }
  }

  void _showActionFailure() {
    if (!mounted) return;
    showToastFailed(null);
  }
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
