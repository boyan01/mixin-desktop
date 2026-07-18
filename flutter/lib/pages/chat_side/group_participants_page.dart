import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/constants/icon_fonts.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/conversation_info_destination.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:mixin_desktop_ui/widgets/custom_context_menu.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/search_text_field.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';
import 'package:super_context_menu/super_context_menu.dart';

import 'chat_side_scope.dart';
import 'group_invite/group_invite_dialog.dart';

class GroupParticipantsPage extends StatefulWidget {
  const GroupParticipantsPage({super.key});

  @override
  State<GroupParticipantsPage> createState() => _GroupParticipantsPageState();
}

class _GroupParticipantsPageState extends State<GroupParticipantsPage> {
  List<rust.ConversationParticipantItem> _participants = const [];
  bool _loading = true;
  StreamSubscription<BigInt>? _changes;
  final _searchController = TextEditingController();
  String _keyword = '';

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _changes ??= ChatSideScope.of(context).account.conversationChanges().listen(
      (_) => unawaited(_load()),
      onError: (_) {},
    );
    if (_loading && _participants.isEmpty) unawaited(_load());
  }

  @override
  void dispose() {
    unawaited(_changes?.cancel());
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final scope = ChatSideScope.of(context);
    try {
      final participants = await scope.account
          .conversation()
          .conversationParticipants(conversationId: scope.conversation.id);
      if (!mounted) return;
      setState(() {
        _participants = participants;
        _loading = false;
      });
    } on Object {
      if (!mounted) return;
      setState(() => _loading = false);
    }
  }

  Future<void> _update(
    String action,
    List<String> userIds, {
    String? role,
  }) async {
    final scope = ChatSideScope.of(context);
    final successful = await runFutureWithToast(
      scope.account.conversation().updateParticipants(
        conversationId: scope.conversation.id,
        action: action,
        userIds: userIds,
        role: role,
      ),
    );
    if (!successful || !mounted) return;
    setState(() => _loading = true);
    await _load();
  }

  Future<void> _addParticipant() async {
    final scope = ChatSideScope.of(context);
    final userIds = await _showParticipantSelector(
      context,
      account: scope.account,
      excludedUserIds: _participants.map((item) => item.userId).toSet(),
      maxSelect: 1024 - _participants.length,
    );
    if (userIds == null || userIds.isEmpty || !mounted) return;
    await _update('ADD', userIds);
  }

  Future<void> _openParticipant(
    rust.ConversationParticipantItem participant,
  ) async {
    final scope = ChatSideScope.of(context);
    final result = await showMessageUserDialog(
      context,
      account: scope.account,
      userId: participant.userId,
    );
    if (!context.mounted || !mounted || result == null) return;
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
  Widget build(BuildContext context) {
    final scope = ChatSideScope.of(context);
    rust.ConversationParticipantItem? current;
    for (final participant in _participants) {
      if (participant.userId == scope.currentUserId) {
        current = participant;
        break;
      }
    }
    final canManage = current?.role == 'OWNER' || current?.role == 'ADMIN';
    final keyword = _keyword.toLowerCase();
    final participants = keyword.isEmpty
        ? _participants
        : _participants
              .where(
                (participant) =>
                    participant.fullName.toLowerCase().contains(keyword) ||
                    participant.identityNumber.contains(_keyword),
              )
              .toList(growable: false);
    return ChatSidePageScaffold(
      title: context.l10n.groupParticipants,
      backgroundColor: context.theme.primary,
      actions: [
        if (canManage)
          CustomPopupMenuButton<_ParticipantAction>(
            icon: MixinAssets.add,
            onSelected: (action) {
              switch (action) {
                case _ParticipantAction.add:
                  unawaited(_addParticipant());
                case _ParticipantAction.inviteLink:
                  unawaited(
                    showGroupInviteByLinkDialog(
                      context,
                      account: scope.account,
                      conversation: scope.conversation,
                    ),
                  );
              }
            },
            itemBuilder: (context) => [
              CustomPopupMenuItem(
                value: _ParticipantAction.add,
                icon: MixinAssets.contextMenuSearchUser,
                title: context.l10n.addParticipants,
              ),
              CustomPopupMenuItem(
                value: _ParticipantAction.inviteLink,
                icon: MixinAssets.contextMenuLink,
                title: context.l10n.inviteToGroupViaLink,
              ),
            ],
          ),
      ],
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: SearchTextField(
              controller: _searchController,
              hintText: context.l10n.settingAuthSearchHint,
              autofocus: true,
              onChanged: (value) => setState(() => _keyword = value.trim()),
            ),
          ),
          if (current != null)
            Expanded(
              child: ListView.builder(
                itemCount: participants.length,
                padding: const EdgeInsets.only(top: 8),
                itemBuilder: (context, index) => _ParticipantTile(
                  participant: participants[index],
                  currentUser: current!,
                  currentUserId: scope.currentUserId,
                  keyword: _keyword,
                  onOpen: _openParticipant,
                  onUpdate: _update,
                ),
              ),
            )
          else
            const SizedBox(),
        ],
      ),
    );
  }
}

enum _ParticipantAction { add, inviteLink }

class _ParticipantTile extends StatelessWidget {
  const _ParticipantTile({
    required this.participant,
    required this.currentUser,
    required this.currentUserId,
    required this.keyword,
    required this.onOpen,
    required this.onUpdate,
  });

  final rust.ConversationParticipantItem participant;
  final rust.ConversationParticipantItem currentUser;
  final String currentUserId;
  final String keyword;
  final ValueChanged<rust.ConversationParticipantItem> onOpen;
  final Future<void> Function(String, List<String>, {String? role}) onUpdate;

  @override
  Widget build(BuildContext context) {
    final tile = Material(
      color: context.theme.primary,
      child: InkWell(
        onTap: () => onOpen(participant),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 16),
          child: Row(
            children: [
              AvatarView(
                userId: participant.userId,
                name: participant.fullName,
                avatarUrl: participant.avatarUrl,
                size: 50,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: CustomText(
                            participant.fullName.isEmpty
                                ? '?'
                                : participant.fullName,
                            style: TextStyle(
                              color: context.theme.text,
                              fontSize: 16,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            textMatchers: [
                              EmojiTextMatcher(),
                              KeyWordTextMatcher(
                                keyword,
                                style: TextStyle(color: context.theme.accent),
                              ),
                            ],
                          ),
                        ),
                        BadgesWidget(
                          verified: participant.isVerified,
                          isBot: participant.isBot,
                          membership: participant.membership,
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      participant.identityNumber,
                      style: TextStyle(
                        color: context.theme.secondaryText,
                        fontSize: 14,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              if (participant.role == 'OWNER')
                _RoleLabel(context.l10n.owner)
              else if (participant.role == 'ADMIN')
                _RoleLabel(context.l10n.admin)
              else
                const SizedBox(width: 0),
            ],
          ),
        ),
      ),
    );
    if (participant.userId == currentUserId) return tile;
    return ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (_) => Menu(
        children: [
          MenuAction(
            image: MenuImage.icon(IconFonts.chat),
            title: context.l10n.groupPopMenuMessage(participant.fullName),
            callback: () => onOpen(participant),
          ),
          if (currentUser.role == 'OWNER') ...[
            MenuSeparator(),
            MenuAction(
              image: MenuImage.icon(IconFonts.manageUser),
              title: participant.role == 'ADMIN'
                  ? context.l10n.dismissAsAdmin
                  : context.l10n.makeGroupAdmin,
              callback: () => onUpdate('ROLE', [
                participant.userId,
              ], role: participant.role == 'ADMIN' ? null : 'ADMIN'),
            ),
          ],
          if ((currentUser.role != null && participant.role == null) ||
              currentUser.role == 'OWNER') ...[
            MenuSeparator(),
            MenuAction(
              image: MenuImage.icon(IconFonts.delete),
              title: context.l10n.groupPopMenuRemove(participant.fullName),
              callback: () => onUpdate('REMOVE', [participant.userId]),
              attributes: const MenuActionAttributes(destructive: true),
            ),
          ],
        ],
      ),
      child: tile,
    );
  }
}

class _RoleLabel extends StatelessWidget {
  const _RoleLabel(this.label);

  final String label;

  @override
  Widget build(BuildContext context) => Text(
    label,
    style: TextStyle(color: context.theme.secondaryText, fontSize: 14),
  );
}

Future<List<String>?> _showParticipantSelector(
  BuildContext context, {
  required rust.AccountHandle account,
  required Set<String> excludedUserIds,
  required int maxSelect,
}) async {
  final selected = await showConversationMultiSelector(
    context,
    account: account,
    title: context.l10n.addParticipants,
    filteredOwnerIds: excludedUserIds,
    maxSelect: maxSelect,
  );
  return selected
      ?.map((conversation) => conversation.ownerId)
      .toList(growable: false);
}
