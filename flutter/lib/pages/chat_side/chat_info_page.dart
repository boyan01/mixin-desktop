import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/pages/conversation_info_destination.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';

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
  bool loading = true;
  bool acting = false;
  Object? error;
  StreamSubscription<BigInt>? conversationChanges;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final account = ChatSideScope.of(context).account;
    conversationChanges ??= account.conversationChanges().listen(
      (_) => unawaited(_loadConversationState()),
      onError: (Object exception) {
        if (mounted) setState(() => error = exception);
      },
    );
    if (loading && detail == null && error == null) unawaited(_load());
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
      });
    } on Object catch (exception) {
      if (mounted) setState(() => error = exception);
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
      final detailFuture = scope.account.conversation().conversationDetail(
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
        loadedApps = await scope.account.user().sharedApps(
          userId: scope.conversation.ownerId,
        );
        loadedDeveloperId = await scope.account.user().botCreatorId(
          userId: scope.conversation.ownerId,
        );
      }
      final loadedDetail = await detailFuture;
      if (!mounted) return;
      setState(() {
        detail = loadedDetail;
        user = loadedUser;
        currentParticipant = loadedParticipant;
        sharedApps = loadedApps;
        developerId = loadedDeveloperId;
        loading = false;
        error = null;
      });
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        loading = false;
        error = exception;
      });
    }
  }

  Future<void> _run(Future<void> Function() action) async {
    if (acting) return;
    setState(() {
      acting = true;
      error = null;
    });
    try {
      await action();
      if (mounted) await _load();
    } on Object catch (exception) {
      if (mounted) setState(() => error = exception);
    } finally {
      if (mounted) setState(() => acting = false);
    }
  }

  Future<bool> _confirm(String title) async =>
      await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(title),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(context.l10n.cancel),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: Text(context.l10n.confirm),
            ),
          ],
        ),
      ) ??
      false;

  Future<String?> _edit(String title, String value, {int maxLines = 1}) =>
      showDialog<String>(
        context: context,
        builder: (context) =>
            _EditDialog(title: title, value: value, maxLines: maxLines),
      );

  Future<void> _shareContact() async {
    final scope = ChatSideScope.of(context);
    final conversationId = await showForwardConversationSelector(
      context,
      account: scope.account,
    );
    if (conversationId == null) return;
    await _run(
      () => scope.account
          .message()
          .sendContact(
            conversationId: conversationId,
            sharedUserId: scope.conversation.ownerId,
          )
          .then((_) {}),
    );
  }

  Future<void> _mute() async {
    final scope = ChatSideScope.of(context);
    final muted =
        detail!.muteUntilMillis.toInt() > DateTime.now().millisecondsSinceEpoch;
    var seconds = 0;
    if (!muted) {
      final selected = await showDialog<int>(
        context: context,
        builder: (context) => SimpleDialog(
          title: Text(context.l10n.contactMuteTitle),
          children:
              [
                    (context.l10n.oneHour, 3600),
                    (context.l10n.hour(8, 8), 8 * 3600),
                    ('1 ${context.l10n.unitWeek(1)}', 7 * 24 * 3600),
                    (context.l10n.oneYear, 365 * 24 * 3600),
                  ]
                  .map(
                    (option) => SimpleDialogOption(
                      onPressed: () => Navigator.pop(context, option.$2),
                      child: Text(option.$1),
                    ),
                  )
                  .toList(),
        ),
      );
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
    if (loading) {
      return ChatSidePageScaffold(
        title: '',
        root: true,
        body: const Center(child: CircularProgressIndicator()),
      );
    }
    if (detail == null) {
      return ChatSidePageScaffold(
        title: '',
        root: true,
        body: ChatSideError(error: error!, onRetry: _load),
      );
    }
    final scope = ChatSideScope.of(context);
    final conversation = scope.conversation;
    final isGroup = conversation.isGroup;
    final isOwnerOrAdmin =
        currentParticipant?.role == 'OWNER' ||
        currentParticipant?.role == 'ADMIN';
    final isExited = isGroup && currentParticipant == null;
    final muted =
        detail!.muteUntilMillis.toInt() > DateTime.now().millisecondsSinceEpoch;
    final expireIn = detail!.expireIn.toInt();
    final canModifyExpire = !isGroup || isOwnerOrAdmin;
    return ChatSidePageScaffold(
      title: '',
      root: true,
      body: Stack(
        children: [
          SingleChildScrollView(
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
                  child: Text(
                    detail!.name.isEmpty ? conversation.name : detail!.name,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: context.theme.text, fontSize: 18),
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
                if (!isGroup && user?.relationship == 'STRANGER')
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: TextButton(
                      onPressed: () => _run(
                        () => scope.account.user().addContact(
                          userId: conversation.ownerId,
                          fullName: conversation.name,
                        ),
                      ),
                      child: Text(
                        user!.isBot
                            ? context.l10n.addBotWithPlus
                            : context.l10n.addContactWithPlus,
                      ),
                    ),
                  ),
                if ((isGroup ? detail!.announcement : user?.biography ?? '')
                    .isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.fromLTRB(36, 12, 36, 20),
                    child: Text(
                      isGroup ? detail!.announcement : user!.biography,
                      textAlign: TextAlign.center,
                    ),
                  )
                else
                  const SizedBox(height: 32),
                if (isGroup && !isExited)
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: context.l10n.groupParticipants,
                        onTap: () => scope.notifier.openDestination(
                          ConversationInfoDestination.participants,
                        ),
                      ),
                    ],
                  ),
                if (!isGroup)
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: context.l10n.shareContact,
                        trailing: PopupMenuButton<void>(
                          onSelected: (_) {
                            final url = user?.codeUrl ?? '';
                            if (url.isNotEmpty) {
                              Clipboard.setData(ClipboardData(text: url));
                            }
                          },
                          itemBuilder: (context) => [
                            PopupMenuItem(child: Text(context.l10n.copyLink)),
                          ],
                        ),
                        onTap: _shareContact,
                      ),
                    ],
                  ),
                ChatSideCellGroup(
                  children: [
                    ChatSideCell(
                      title: context.l10n.sharedMedia,
                      onTap: () => scope.notifier.openDestination(
                        ConversationInfoDestination.sharedMedia,
                      ),
                    ),
                    if (sharedApps.isNotEmpty)
                      ChatSideCell(
                        title: context.l10n.shareApps,
                        onTap: () => scope.notifier.openDestination(
                          ConversationInfoDestination.sharedApps,
                        ),
                      ),
                    ChatSideCell(
                      title: context.l10n.searchConversation,
                      onTap: () => scope.notifier.openDestination(
                        ConversationInfoDestination.searchMessageHistory,
                      ),
                    ),
                  ],
                ),
                if (!(isGroup && isExited))
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: context.l10n.disappearingMessage,
                        description: _duration(context, expireIn),
                        onTap: canModifyExpire
                            ? () => scope.notifier.openDestination(
                                ConversationInfoDestination.disappearMessages,
                              )
                            : null,
                      ),
                    ],
                  ),
                if (isGroup && isOwnerOrAdmin)
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: detail!.announcement.isEmpty
                            ? context.l10n.addGroupDescription
                            : context.l10n.editGroupDescription,
                        onTap: () async {
                          final value = await _edit(
                            detail!.announcement.isEmpty
                                ? context.l10n.addGroupDescription
                                : context.l10n.editGroupDescription,
                            detail!.announcement,
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
                    ],
                  ),
                ChatSideCellGroup(
                  children: [
                    if (!(isGroup && isExited))
                      ChatSideCell(
                        title: muted ? context.l10n.unmute : context.l10n.mute,
                        description: muted
                            ? DateFormat('yyyy/MM/dd, hh:mm a').format(
                                DateTime.fromMillisecondsSinceEpoch(
                                  detail!.muteUntilMillis.toInt(),
                                ).toLocal(),
                              )
                            : null,
                        trailing: const SizedBox(),
                        onTap: _mute,
                      ),
                    if (!isGroup || isOwnerOrAdmin)
                      ChatSideCell(
                        title: context.l10n.editName,
                        trailing: const SizedBox(),
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
                if (!isGroup)
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: context.l10n.groupsInCommon,
                        onTap: () => scope.notifier.openDestination(
                          ConversationInfoDestination.groupsInCommon,
                        ),
                      ),
                    ],
                  ),
                if (developerId != null)
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: context.l10n.developer,
                        trailing: const SizedBox(),
                        onTap: () => showMessageUserDialog(
                          context,
                          account: scope.account,
                          userId: developerId,
                        ),
                      ),
                    ],
                  ),
                ChatSideCellGroup(
                  children: [
                    ChatSideCell(
                      title: context.l10n.editConversations,
                      onTap: () => scope.notifier.openDestination(
                        ConversationInfoDestination.circles,
                      ),
                    ),
                  ],
                ),
                ChatSideCellGroup(
                  children: [
                    if (!isGroup && user?.relationship == 'BLOCKED')
                      ChatSideCell(
                        title: context.l10n.unblock,
                        destructive: true,
                        onTap: () async {
                          if (!await _confirm(context.l10n.unblock)) return;
                          await _run(
                            () => scope.account.user().unblockUser(
                              userId: conversation.ownerId,
                            ),
                          );
                        },
                      ),
                    if (!isGroup &&
                        user != null &&
                        user!.relationship != 'STRANGER')
                      ChatSideCell(
                        title: user!.isBot
                            ? context.l10n.removeBot
                            : context.l10n.removeContact,
                        destructive: true,
                        onTap: () async {
                          final title = user!.isBot
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
                    if (!isGroup && user?.relationship == 'STRANGER')
                      ChatSideCell(
                        title: context.l10n.block,
                        destructive: true,
                        onTap: () async {
                          if (!await _confirm(context.l10n.block)) return;
                          await _run(
                            () => scope.account.user().blockUser(
                              userId: conversation.ownerId,
                            ),
                          );
                        },
                      ),
                    ChatSideCell(
                      title: context.l10n.clearChat,
                      destructive: true,
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
                      ChatSideCell(
                        title: isExited
                            ? context.l10n.deleteGroup
                            : context.l10n.exitGroup,
                        destructive: true,
                        onTap: () async {
                          final title = isExited
                              ? context.l10n.deleteGroup
                              : context.l10n.exitGroup;
                          if (!await _confirm(title)) return;
                          if (isExited) {
                            await scope.account
                                .conversation()
                                .deleteConversation(
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
                if (!isGroup)
                  ChatSideCellGroup(
                    children: [
                      ChatSideCell(
                        title: context.l10n.report,
                        destructive: true,
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
                    ],
                  ),
                if (isGroup)
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
          if (error != null)
            Positioned(
              left: 8,
              right: 8,
              top: 0,
              child: MaterialBanner(
                content: Text(error.toString()),
                actions: [
                  TextButton(
                    onPressed: () => setState(() => error = null),
                    child: const Icon(Icons.close),
                  ),
                ],
              ),
            ),
          if (acting)
            const Positioned.fill(
              child: ColoredBox(
                color: Color(0x22000000),
                child: Center(child: CircularProgressIndicator()),
              ),
            ),
        ],
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

class _EditDialog extends StatefulWidget {
  const _EditDialog({
    required this.title,
    required this.value,
    required this.maxLines,
  });

  final String title;
  final String value;
  final int maxLines;

  @override
  State<_EditDialog> createState() => _EditDialogState();
}

class _EditDialogState extends State<_EditDialog> {
  late final controller = TextEditingController(text: widget.value);

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.title),
    content: TextField(
      controller: controller,
      autofocus: true,
      maxLines: widget.maxLines,
      maxLength: widget.maxLines == 1 ? 40 : 512,
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      FilledButton(
        onPressed: () => Navigator.pop(context, controller.text.trim()),
        child: Text(context.l10n.confirm),
      ),
    ],
  );
}
