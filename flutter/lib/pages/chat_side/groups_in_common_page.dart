import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';

class GroupsInCommonPage extends StatefulWidget {
  const GroupsInCommonPage({super.key});

  @override
  State<GroupsInCommonPage> createState() => _GroupsInCommonPageState();
}

class _GroupsInCommonPageState extends State<GroupsInCommonPage> {
  Future<List<rust.GroupConversationItem>>? _future;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _future ??= _load();
  }

  Future<List<rust.GroupConversationItem>> _load() {
    final scope = ChatSideScope.of(context);
    final userId = scope.conversation.ownerId;
    return scope.account.conversation().groupsInCommon(userId: userId);
  }

  void _reload() => setState(() => _future = _load());

  Future<void> _select(rust.GroupConversationItem group) async {
    final scope = ChatSideScope.of(context);
    final count = await scope.account.conversation().conversationCount(
      category: 'groups',
      circleId: null,
      keyword: '',
      unseenOnly: false,
    );
    var offset = 0;
    while (offset < count.toInt()) {
      final conversations = await scope.account.conversation().conversations(
        category: 'groups',
        circleId: null,
        keyword: '',
        unseenOnly: false,
        limit: 200,
        offset: offset,
      );
      for (final conversation in conversations) {
        if (conversation.conversationId == group.conversationId && mounted) {
          scope.onSelectConversation(
            ConversationListEntry.fromRust(conversation),
          );
          return;
        }
      }
      if (conversations.length < 200) return;
      offset += conversations.length;
    }
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.groupsInCommon,
    body: FutureBuilder<List<rust.GroupConversationItem>>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return ChatSideError(error: snapshot.error!, onRetry: _reload);
        }
        if (!snapshot.hasData) {
          return const Center(child: CircularProgressIndicator());
        }
        final groups = snapshot.data!;
        if (groups.isEmpty) {
          return Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                SvgPicture.asset(
                  MixinAssets.empty,
                  width: 80,
                  height: 80,
                  colorFilter: ColorFilter.mode(
                    context.theme.secondaryText,
                    BlendMode.srcIn,
                  ),
                ),
                const SizedBox(height: 20),
                Text(
                  context.l10n.noResults,
                  style: TextStyle(color: context.theme.secondaryText),
                ),
              ],
            ),
          );
        }
        return ListView.builder(
          itemCount: groups.length,
          itemBuilder: (context, index) {
            final group = groups[index];
            return SizedBox(
              height: 72,
              child: ListTile(
                contentPadding: const EdgeInsets.symmetric(horizontal: 20),
                leading: AvatarView(
                  userId: group.conversationId,
                  name: group.name,
                  avatarUrl: group.avatarUrl,
                  size: 48,
                ),
                title: Text(
                  group.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                subtitle: Text(
                  context.l10n.participantsCount(
                    group.participantCount.toInt(),
                  ),
                ),
                onTap: () => _select(group),
              ),
            );
          },
        );
      },
    ),
  );
}
