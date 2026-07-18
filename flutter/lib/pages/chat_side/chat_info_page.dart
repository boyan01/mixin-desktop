import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/pages/chat_side/shared_apps_page.dart';
import 'package:mixin_desktop_ui/pages/conversation_info_destination.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/mute_dialog.dart';
import 'package:mixin_desktop_ui/widgets/more_extended_text.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';

class ChatInfoPage extends StatefulWidget {
  const ChatInfoPage({super.key});

  @override
  State<ChatInfoPage> createState() => _ChatInfoPageState();
}

class _ChatInfoPageState extends State<ChatInfoPage> {
  rust.ConversationDetailItem? detail;
  rust.UserProfileItem? user;
  rust.ConversationParticipantItem? currentParticipant;
  List<rust.SharedAppItem> sharedApps = const [];
  String? developerId;
  bool loadStarted = false;
  bool participantsLoaded = false;
  StreamSubscription<BigInt>? conversationChanges;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final account = ChatSideScope.of(context).account;
    conversationChanges ??= account.conversationChanges().listen(
      (_) => unawaited(_loadConversationState()),
      onError: (_) {},
    );
    if (!loadStarted) {
      loadStarted = true;
      unawaited(_load());
    }
  }

  Future<void> _loadConversationState() async {
    final scope = ChatSideScope.of(context);
    try {
      final loadedDetail = await scope.account
          .conversation()
          .localConversationDetail(conversationId: scope.conversation.id);
      rust.ConversationParticipantItem? loadedParticipant;
      if (scope.conversation.isGroup) {
        final participants = await scope.account
            .conversation()
            .conversationParticipants(conversationId: scope.conversation.id);
        for (final participant in participants) {
          if (participant.userId == scope.currentUserId) {
            loadedParticipant = participant;
            break;
          }
        }
      }
      if (!mounted) return;
      setState(() {
        detail = loadedDetail;
        currentParticipant = loadedParticipant;
        participantsLoaded = true;
      });
    } on Object {
      return;
    }
  }

  @override
  void dispose() {
    unawaited(conversationChanges?.cancel());
    super.dispose();
  }

  Future<void> _load() async {
    final scope = ChatSideScope.of(context);
    try {
      final detailFuture = scope.account.conversation().localConversationDetail(
        conversationId: scope.conversation.id,
      );
      rust.UserProfileItem? loadedUser;
      rust.ConversationParticipantItem? loadedParticipant;
      var loadedApps = const <rust.SharedAppItem>[];
      String? loadedDeveloperId;
      if (scope.conversation.isGroup) {
        final participants = await scope.account
            .conversation()
            .conversationParticipants(conversationId: scope.conversation.id);
        for (final participant in participants) {
          if (participant.userId == scope.currentUserId) {
            loadedParticipant = participant;
            break;
          }
        }
      } else {
        loadedUser = await scope.account.user().userProfile(
          userId: scope.conversation.ownerId,
          identityNumber: null,
        );
        loadedDeveloperId = await scope.account.user().botCreatorId(
          userId: scope.conversation.ownerId,
        );
        loadedApps = await scope.account.user().localSharedApps(
          userId: scope.conversation.ownerId,
        );
      }
      final loadedDetail = await detailFuture;
      if (!mounted) return;
      setState(() {
        detail = loadedDetail;
        user = loadedUser;
        currentParticipant = loadedParticipant;
        participantsLoaded = true;
        sharedApps = loadedApps;
        developerId = loadedDeveloperId;
      });
      unawaited(_refreshRemoteState());
    } on Object {
      return;
    }
  }

  Future<void> _refreshRemoteState() async {
    final scope = ChatSideScope.of(context);
    final appsFuture = scope.conversation.isGroup
        ? null
        : scope.account.user().sharedApps(userId: scope.conversation.ownerId);
    try {
      final refreshedDetail = await scope.account
          .conversation()
          .conversationDetail(conversationId: scope.conversation.id);
      if (!mounted) return;
      setState(() {
        detail = refreshedDetail;
      });
    } on Object {
      // Keep showing the local conversation detail if refresh fails.
    }
    if (appsFuture == null) return;
    try {
      final loadedApps = await appsFuture;
      if (mounted) setState(() => sharedApps = loadedApps);
    } on Object {
      // Shared apps are optional on ChatInfo; keep showing the local cache.
    }
  }

  Future<void> _run(Future<void> Function() action) async {
    final successful = await runWithToast(action);
    if (successful && mounted) await _load();
  }

  Future<bool> _confirm(String title) async =>
      await showConfirmMixinDialog(context, title) == DialogEvent.positive;

  Future<String?> _edit(String title, String value, {int maxLines = 1}) =>
      showMixinDialog<String>(
        context: context,
        child: EditDialog(
          title: Text(title),
          editText: value,
          positiveAction: context.l10n.change,
          maxLines: maxLines,
          maxLength: maxLines == 1 ? 40 : 512,
        ),
      );

  Future<void> _shareContact() async {
    final scope = ChatSideScope.of(context);
    final conversation = await showConversationSelector(
      context,
      account: scope.account,
      title: context.l10n.shareContact,
    );
    if (conversation == null) return;
    await _run(
      () => scope.account
          .message()
          .sendContact(
            conversationId: conversation.id,
            sharedUserId: scope.conversation.ownerId,
            quoteMessageId: null,
            silent: false,
          )
          .then((_) {}),
    );
  }

  Future<void> _mute() async {
    final scope = ChatSideScope.of(context);
    final muted = detail == null
        ? scope.conversation.isMuted
        : detail!.muteUntilMillis.toInt() >
              DateTime.now().millisecondsSinceEpoch;
    var seconds = 0;
    if (!muted) {
      final selected = await showMuteDialog(context);
      if (selected == null) return;
      seconds = selected;
    }
    await _run(
      () => scope.account.conversation().setMuted(
        conversationId: scope.conversation.id,
        ownerId: scope.conversation.ownerId,
        category: scope.conversation.category,
        durationSeconds: seconds,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final scope = ChatSideScope.of(context);
    final conversation = scope.conversation;
    final isGroup = conversation.isGroup;
    final announcement = detail?.announcement ?? '';
    final biography = user?.biography ?? '';
    final detailName = detail?.name ?? '';
    final relationship = user?.relationship ?? conversation.relationship;
    final isBot = user?.isBot ?? conversation.isBot;
    final isOwnerOrAdmin =
        currentParticipant?.role == 'OWNER' ||
        currentParticipant?.role == 'ADMIN';
    final isExited =
        isGroup && participantsLoaded && currentParticipant == null;
    final muted = detail == null
        ? conversation.isMuted
        : detail!.muteUntilMillis.toInt() >
              DateTime.now().millisecondsSinceEpoch;
    final expireIn = detail?.expireIn.toInt() ?? 0;
    final canModifyExpire = !isGroup || isOwnerOrAdmin;
    return ChatSidePageScaffold(
      title: '',
      root: true,
      body: SingleChildScrollView(
        child: Column(
          children: [
            const SizedBox(height: 8),
            GestureDetector(
              onTap: () {
                final keys = HardwareKeyboard.instance.logicalKeysPressed;
                if (!keys.contains(LogicalKeyboardKey.altLeft) &&
                    !keys.contains(LogicalKeyboardKey.altRight)) {
                  return;
                }
                Clipboard.setData(
                  ClipboardData(
                    text: 'mixin://conversations/${conversation.id}',
                  ),
                );
              },
              child: ConversationAvatarView(
                conversation: conversation,
                size: 90,
              ),
            ),
            const SizedBox(height: 10),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Flexible(
                    child: Text(
                      detailName.isEmpty ? conversation.name : detailName,
                      textAlign: TextAlign.center,
                      style: TextStyle(color: context.theme.text, fontSize: 18),
                    ),
                  ),
                  if (!isGroup)
                    BadgesWidget(
                      verified: user?.isVerified ?? conversation.isVerified,
                      isBot: isBot,
                      membership: user?.membership ?? conversation.membership,
                    ),
                ],
              ),
            ),
            const SizedBox(height: 4),
            Text(
              isGroup
                  ? context.l10n.participantsCount(
                      conversation.participantCount,
                    )
                  : 'Mixin ID: ${conversation.identityNumber}',
              style: TextStyle(
                color: context.theme.secondaryText,
                fontSize: 12,
              ),
            ),
            if (!isGroup)
              AnimatedSize(
                duration: const Duration(milliseconds: 200),
                curve: Curves.easeInOut,
                child: relationship == 'STRANGER'
                    ? Padding(
                        padding: const EdgeInsets.only(top: 12),
                        child: TextButton(
                          style: TextButton.styleFrom(
                            backgroundColor: context.theme.statusBackground,
                            padding: const EdgeInsets.symmetric(
                              horizontal: 15,
                              vertical: 7,
                            ),
                            shape: const RoundedRectangleBorder(
                              borderRadius: BorderRadius.all(
                                Radius.circular(15),
                              ),
                            ),
                          ),
                          onPressed: () => _run(
                            () => scope.account.user().addContact(
                              userId: conversation.ownerId,
                              fullName: conversation.name,
                            ),
                          ),
                          child: Text(
                            isBot
                                ? context.l10n.addBotWithPlus
                                : context.l10n.addContactWithPlus,
                            style: TextStyle(
                              color: context.theme.accent,
                              fontSize: 12,
                            ),
                          ),
                        ),
                      )
                    : const SizedBox(),
              ),
            const SizedBox(height: 12),
            Container(
              margin: const EdgeInsets.symmetric(horizontal: 36),
              child: (isGroup ? announcement : biography).isEmpty
                  ? const SizedBox()
                  : MoreExtendedText(
                      isGroup ? announcement : biography,
                      style: TextStyle(color: context.theme.text, fontSize: 14),
                    ),
            ),
            const SizedBox(height: 32),
            if (isGroup && !isExited)
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(context.l10n.groupParticipants),
                  onTap: () => scope.notifier.openDestination(
                    ConversationInfoDestination.participants,
                  ),
                ),
              ),
            if (!isGroup)
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(context.l10n.shareContact),
                  trailing: CustomPopupMenuButton<Object?>(
                    alignment: Alignment.bottomCenter,
                    color: context.theme.icon,
                    icon: MixinAssets.inviteShare,
                    onSelected: (_) {
                      final url = user?.codeUrl ?? '';
                      if (url.isNotEmpty) {
                        Clipboard.setData(ClipboardData(text: url));
                      }
                    },
                    itemBuilder: (context) => [
                      CustomPopupMenuItem(
                        icon: MixinAssets.contextMenuCopy,
                        title: context.l10n.copyLink,
                        value: null,
                      ),
                    ],
                  ),
                  onTap: _shareContact,
                ),
              ),
            CellGroup(
              cellBackgroundColor: context.theme.listSelected,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  CellItem(
                    title: Text(context.l10n.sharedMedia),
                    onTap: () => scope.notifier.openDestination(
                      ConversationInfoDestination.sharedMedia,
                    ),
                  ),
                  if (sharedApps.isNotEmpty)
                    CellItem(
                      title: Text(context.l10n.shareApps),
                      trailing: OverlappedAppIcons(apps: sharedApps),
                      onTap: () => scope.notifier.openDestination(
                        ConversationInfoDestination.sharedApps,
                      ),
                    ),
                  CellItem(
                    title: Text(context.l10n.searchConversation, maxLines: 1),
                    onTap: () => scope.notifier.openDestination(
                      ConversationInfoDestination.searchMessageHistory,
                    ),
                  ),
                ],
              ),
            ),
            if (!(isGroup && isExited))
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(context.l10n.disappearingMessage),
                  description: Text(_duration(context, expireIn)),
                  trailing: canModifyExpire ? const Arrow() : null,
                  onTap: canModifyExpire
                      ? () => scope.notifier.openDestination(
                          ConversationInfoDestination.disappearMessages,
                        )
                      : null,
                ),
              ),
            if (isGroup && isOwnerOrAdmin)
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(
                    announcement.isEmpty
                        ? context.l10n.addGroupDescription
                        : context.l10n.editGroupDescription,
                  ),
                  onTap: () async {
                    final value = await _edit(
                      announcement.isEmpty
                          ? context.l10n.addGroupDescription
                          : context.l10n.editGroupDescription,
                      announcement,
                      maxLines: 7,
                    );
                    if (value == null) return;
                    await _run(
                      () => scope.account.conversation().editConversation(
                        conversationId: conversation.id,
                        name: null,
                        announcement: value,
                      ),
                    );
                  },
                ),
              ),
            CellGroup(
              cellBackgroundColor: context.theme.listSelected,
              child: Column(
                children: [
                  if (!(isGroup && isExited))
                    CellItem(
                      title: Text(
                        muted ? context.l10n.unmute : context.l10n.mute,
                      ),
                      description: muted && detail != null
                          ? Text(
                              DateFormat('yyyy/MM/dd, hh:mm a').format(
                                DateTime.fromMillisecondsSinceEpoch(
                                  detail!.muteUntilMillis.toInt(),
                                ).toLocal(),
                              ),
                            )
                          : null,
                      trailing: null,
                      onTap: _mute,
                    ),
                  if (!isGroup || isOwnerOrAdmin)
                    CellItem(
                      title: Text(context.l10n.editName),
                      trailing: null,
                      onTap: () async {
                        final value = await _edit(
                          context.l10n.editName,
                          conversation.name,
                        );
                        if (value == null || value.isEmpty) return;
                        await _run(
                          isGroup
                              ? () => scope.account
                                    .conversation()
                                    .editConversation(
                                      conversationId: conversation.id,
                                      name: value,
                                      announcement: null,
                                    )
                              : () => scope.account.user().addContact(
                                  userId: conversation.ownerId,
                                  fullName: value,
                                ),
                        );
                      },
                    ),
                ],
              ),
            ),
            if (!isGroup)
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(context.l10n.groupsInCommon),
                  onTap: () => scope.notifier.openDestination(
                    ConversationInfoDestination.groupsInCommon,
                  ),
                ),
              ),
            if (developerId != null)
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(context.l10n.developer),
                  trailing: null,
                  onTap: () async {
                    final result = await showMessageUserDialog(
                      context,
                      account: scope.account,
                      userId: developerId,
                    );
                    if (!context.mounted || !mounted || result == null) {
                      return;
                    }
                    await handleMessageUserDialogResult(
                      context,
                      account: scope.account,
                      result: result,
                      onSelectConversation: scope.onSelectConversation,
                      onSelectConversationInfo: (conversation) {
                        scope.onSelectConversation(conversation);
                        scope.notifier.openDestination(
                          ConversationInfoDestination.infoPage,
                        );
                      },
                    );
                  },
                ),
              ),
            CellGroup(
              cellBackgroundColor: context.theme.listSelected,
              child: CellItem(
                title: Text(context.l10n.editConversations),
                onTap: () => scope.notifier.openDestination(
                  ConversationInfoDestination.circles,
                ),
              ),
            ),
            CellGroup(
              cellBackgroundColor: context.theme.listSelected,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (!isGroup && relationship == 'BLOCKED')
                    CellItem(
                      title: Text(context.l10n.unblock),
                      color: context.theme.red,
                      trailing: null,
                      onTap: () async {
                        if (!await _confirm(context.l10n.unblock)) return;
                        await _run(
                          () => scope.account.user().unblockUser(
                            userId: conversation.ownerId,
                          ),
                        );
                      },
                    ),
                  if (!isGroup && relationship != 'STRANGER')
                    CellItem(
                      title: Text(
                        isBot
                            ? context.l10n.removeBot
                            : context.l10n.removeContact,
                      ),
                      color: context.theme.red,
                      trailing: null,
                      onTap: () async {
                        final title = isBot
                            ? context.l10n.removeBot
                            : context.l10n.removeContact;
                        if (!await _confirm(title)) return;
                        await _run(
                          () => scope.account.user().removeContact(
                            userId: conversation.ownerId,
                          ),
                        );
                      },
                    ),
                  if (!isGroup && relationship == 'STRANGER')
                    CellItem(
                      title: Text(context.l10n.block),
                      color: context.theme.red,
                      trailing: null,
                      onTap: () async {
                        if (!await _confirm(context.l10n.block)) return;
                        await _run(
                          () => scope.account.user().blockUser(
                            userId: conversation.ownerId,
                          ),
                        );
                      },
                    ),
                  CellItem(
                    title: Text(context.l10n.clearChat),
                    color: context.theme.red,
                    trailing: null,
                    onTap: () async {
                      if (!await _confirm(context.l10n.clearChat)) return;
                      await _run(
                        () => scope.account.conversation().clearConversation(
                          conversationId: conversation.id,
                        ),
                      );
                    },
                  ),
                  if (isGroup)
                    CellItem(
                      title: Text(
                        isExited
                            ? context.l10n.deleteGroup
                            : context.l10n.exitGroup,
                      ),
                      color: context.theme.red,
                      trailing: null,
                      onTap: () async {
                        final title = isExited
                            ? context.l10n.deleteGroup
                            : context.l10n.exitGroup;
                        if (!await _confirm(title)) return;
                        if (isExited) {
                          await scope.account.conversation().deleteConversation(
                            conversationId: conversation.id,
                          );
                        } else {
                          await scope.account.conversation().exitGroup(
                            conversationId: conversation.id,
                          );
                        }
                        if (!mounted) return;
                        scope.notifier.clear();
                        scope.onConversationDeleted();
                      },
                    ),
                ],
              ),
            ),
            if (!isGroup)
              CellGroup(
                cellBackgroundColor: context.theme.listSelected,
                child: CellItem(
                  title: Text(context.l10n.report),
                  color: context.theme.red,
                  trailing: null,
                  onTap: () async {
                    if (!await _confirm(context.l10n.reportAndBlock)) {
                      return;
                    }
                    await _run(
                      () => scope.account.user().reportUser(
                        userId: conversation.ownerId,
                      ),
                    );
                  },
                ),
              ),
            if (isGroup && detail != null)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 16),
                child: Text(
                  context.l10n.createdAt(
                    DateFormat.yMMMd().format(
                      DateTime.fromMillisecondsSinceEpoch(
                        detail!.createdAtMillis.toInt(),
                      ).toLocal(),
                    ),
                  ),
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 12,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

String _duration(BuildContext context, int seconds) {
  if (seconds <= 0) return context.l10n.close;
  if (seconds < 60) return '$seconds ${context.l10n.unitSecond(seconds)}';
  if (seconds < 3600) {
    final minutes = seconds ~/ 60;
    return '$minutes ${context.l10n.unitMinute(minutes)}';
  }
  if (seconds < 86400) {
    final hours = seconds ~/ 3600;
    return '$hours ${context.l10n.unitHour(hours)}';
  }
  final days = seconds ~/ 86400;
  return '$days ${context.l10n.unitDay(days)}';
}
