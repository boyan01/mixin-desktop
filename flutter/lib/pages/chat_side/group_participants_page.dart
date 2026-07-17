import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';

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
  Object? _error;
  StreamSubscription<BigInt>? _changes;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _changes ??= ChatSideScope.of(context).account.conversationChanges().listen(
      (_) => unawaited(_load()),
      onError: (Object error) {
        if (mounted) setState(() => _error = error);
      },
    );
    if (_loading && _participants.isEmpty && _error == null) unawaited(_load());
  }

  @override
  void dispose() {
    unawaited(_changes?.cancel());
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
        _error = null;
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = error;
      });
    }
  }

  Future<void> _update(
    String action,
    List<String> userIds, {
    String? role,
  }) async {
    final scope = ChatSideScope.of(context);
    await scope.account.conversation().updateParticipants(
      conversationId: scope.conversation.id,
      action: action,
      userIds: userIds,
      role: role,
    );
    if (!mounted) return;
    setState(() {
      _loading = true;
      _error = null;
    });
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
    return ChatSidePageScaffold(
      title: context.l10n.groupParticipants,
      actions: [
        if (canManage)
          PopupMenuButton<_ParticipantAction>(
            icon: const Icon(Icons.add),
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
              PopupMenuItem(
                value: _ParticipantAction.add,
                child: Text(context.l10n.addParticipants),
              ),
              PopupMenuItem(
                value: _ParticipantAction.inviteLink,
                child: Text(context.l10n.inviteToGroupViaLink),
              ),
            ],
          ),
      ],
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
          ? ChatSideError(error: _error!, onRetry: _load)
          : ListView.builder(
              itemCount: _participants.length,
              itemBuilder: (context, index) {
                final participant = _participants[index];
                return ListTile(
                  onTap: () => showMessageUserDialog(
                    context,
                    account: scope.account,
                    userId: participant.userId,
                  ),
                  leading: AvatarView(
                    userId: participant.userId,
                    name: participant.fullName,
                    avatarUrl: participant.avatarUrl,
                    size: 44,
                  ),
                  title: Text(participant.fullName),
                  subtitle: Text(participant.identityNumber),
                  trailing:
                      canManage &&
                          participant.userId != scope.currentUserId &&
                          (current?.role == 'OWNER' || participant.role == null)
                      ? PopupMenuButton<String>(
                          onSelected: (value) {
                            switch (value) {
                              case 'ADMIN':
                                unawaited(
                                  _update('ROLE', [
                                    participant.userId,
                                  ], role: 'ADMIN'),
                                );
                                break;
                              case 'MEMBER':
                                unawaited(
                                  _update('ROLE', [participant.userId]),
                                );
                                break;
                              case 'REMOVE':
                                unawaited(
                                  _update('REMOVE', [participant.userId]),
                                );
                                break;
                            }
                          },
                          itemBuilder: (context) => [
                            if (current?.role == 'OWNER' &&
                                participant.role != 'ADMIN')
                              PopupMenuItem(
                                value: 'ADMIN',
                                child: Text(context.l10n.admin),
                              ),
                            if (current?.role == 'OWNER' &&
                                participant.role == 'ADMIN')
                              PopupMenuItem(
                                value: 'MEMBER',
                                child: Text(context.l10n.dismissAsAdmin),
                              ),
                            PopupMenuItem(
                              value: 'REMOVE',
                              child: Text(
                                context.l10n.removeContact,
                                style: const TextStyle(color: Colors.red),
                              ),
                            ),
                          ],
                        )
                      : participant.role == null
                      ? null
                      : Text(
                          participant.role!,
                          style: TextStyle(
                            color: context.theme.secondaryText,
                            fontSize: 12,
                          ),
                        ),
                );
              },
            ),
    );
  }
}

enum _ParticipantAction { add, inviteLink }

Future<List<String>?> _showParticipantSelector(
  BuildContext context, {
  required rust.AccountHandle account,
  required Set<String> excludedUserIds,
  required int maxSelect,
}) => showDialog<List<String>>(
  context: context,
  builder: (context) => _ParticipantSelectorDialog(
    account: account,
    excludedUserIds: excludedUserIds,
    maxSelect: maxSelect,
  ),
);

class _ParticipantSelectorDialog extends StatefulWidget {
  const _ParticipantSelectorDialog({
    required this.account,
    required this.excludedUserIds,
    required this.maxSelect,
  });

  final rust.AccountHandle account;
  final Set<String> excludedUserIds;
  final int maxSelect;

  @override
  State<_ParticipantSelectorDialog> createState() =>
      _ParticipantSelectorDialogState();
}

class _ParticipantSelectorDialogState
    extends State<_ParticipantSelectorDialog> {
  final searchController = TextEditingController();
  final selected = <String>{};
  Timer? debounce;
  List<rust.ConversationListItem> contacts = const [];
  bool loading = true;
  Object? error;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  @override
  void dispose() {
    debounce?.cancel();
    searchController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      loading = true;
      error = null;
    });
    try {
      final count = await widget.account.conversation().conversationCount(
        category: 'contacts',
        circleId: null,
        keyword: searchController.text.trim(),
        unseenOnly: false,
      );
      final result = <rust.ConversationListItem>[];
      while (result.length < count.toInt()) {
        final page = await widget.account.conversation().conversations(
          category: 'contacts',
          circleId: null,
          keyword: searchController.text.trim(),
          unseenOnly: false,
          limit: 200,
          offset: result.length,
        );
        result.addAll(page);
        if (page.length < 200) break;
      }
      if (!mounted) return;
      setState(() {
        contacts = result
            .where((item) => !widget.excludedUserIds.contains(item.ownerId))
            .toList();
        loading = false;
      });
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        loading = false;
        error = exception;
      });
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(context.l10n.addParticipants),
    content: SizedBox(
      width: 420,
      height: 520,
      child: Column(
        children: [
          TextField(
            controller: searchController,
            autofocus: true,
            onChanged: (_) {
              debounce?.cancel();
              debounce = Timer(const Duration(milliseconds: 300), _load);
            },
            decoration: InputDecoration(
              hintText: context.l10n.search,
              prefixIcon: const Icon(Icons.search),
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: loading
                ? const Center(child: CircularProgressIndicator())
                : error != null
                ? Center(child: Text(error.toString()))
                : ListView.builder(
                    itemCount: contacts.length,
                    itemBuilder: (context, index) {
                      final contact = contacts[index];
                      final checked = selected.contains(contact.ownerId);
                      return CheckboxListTile(
                        value: checked,
                        secondary: AvatarView(
                          userId: contact.ownerId,
                          name: contact.name,
                          avatarUrl: contact.avatarUrl,
                          size: 40,
                        ),
                        title: Text(contact.name),
                        subtitle: Text(contact.identityNumber),
                        onChanged: checked || selected.length < widget.maxSelect
                            ? (_) => setState(() {
                                if (checked) {
                                  selected.remove(contact.ownerId);
                                } else {
                                  selected.add(contact.ownerId);
                                }
                              })
                            : null,
                      );
                    },
                  ),
          ),
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      FilledButton(
        onPressed: selected.isEmpty
            ? null
            : () => Navigator.pop(context, selected.toList()),
        child: Text(context.l10n.confirm),
      ),
    ],
  );
}
