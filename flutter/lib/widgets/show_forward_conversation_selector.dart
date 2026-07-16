import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';

Future<String?> showForwardConversationSelector(
  BuildContext context, {
  required rust.AccountHandle account,
}) => showDialog<String>(
  context: context,
  builder: (context) => _ForwardConversationSelector(account: account),
);

class _ForwardConversationSelector extends StatefulWidget {
  const _ForwardConversationSelector({required this.account});

  final rust.AccountHandle account;

  @override
  State<_ForwardConversationSelector> createState() =>
      _ForwardConversationSelectorState();
}

class _ForwardConversationSelectorState
    extends State<_ForwardConversationSelector> {
  late final Future<List<ConversationListEntry>> _conversations = _load();
  String _query = '';

  Future<List<ConversationListEntry>> _load() async {
    final count = await widget.account.conversationCount(
      category: ConversationCategoryFilter.chats.name,
      circleId: null,
      keyword: '',
      unseenOnly: false,
    );
    final result = <ConversationListEntry>[];
    while (result.length < count.toInt()) {
      final page = await widget.account.conversations(
        category: ConversationCategoryFilter.chats.name,
        circleId: null,
        keyword: '',
        unseenOnly: false,
        limit: 200,
        offset: result.length,
      );
      result.addAll(page.map(ConversationListEntry.fromRust));
      if (page.length < 200) break;
    }
    return result;
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    titlePadding: const EdgeInsets.fromLTRB(20, 16, 8, 8),
    contentPadding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
    title: Row(
      children: [
        Expanded(child: Text(context.l10n.forward)),
        IconButton(
          onPressed: () => Navigator.pop(context),
          icon: const Icon(Icons.close),
        ),
      ],
    ),
    content: SizedBox(
      width: 420,
      height: 520,
      child: Column(
        children: [
          TextField(
            autofocus: true,
            decoration: InputDecoration(
              hintText: context.l10n.searchConversation,
              prefixIcon: const Icon(Icons.search),
            ),
            onChanged: (value) => setState(() => _query = value.trim()),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: FutureBuilder<List<ConversationListEntry>>(
              future: _conversations,
              builder: (context, snapshot) {
                if (snapshot.connectionState != ConnectionState.done) {
                  return const Center(
                    child: CircularProgressIndicator(strokeWidth: 2),
                  );
                }
                if (snapshot.hasError) {
                  return Center(
                    child: Text(
                      snapshot.error.toString(),
                      style: TextStyle(color: context.theme.secondaryText),
                    ),
                  );
                }
                final query = _query.toLowerCase();
                final conversations = (snapshot.data ?? const [])
                    .where(
                      (item) =>
                          query.isEmpty ||
                          item.name.toLowerCase().contains(query) ||
                          item.identityNumber.contains(query),
                    )
                    .toList(growable: false);
                return ListView.builder(
                  itemCount: conversations.length,
                  itemBuilder: (context, index) {
                    final conversation = conversations[index];
                    return ListTile(
                      leading: ConversationAvatarView(
                        conversation: conversation,
                        size: 40,
                      ),
                      title: Text(
                        conversation.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: conversation.identityNumber.isEmpty
                          ? null
                          : Text(conversation.identityNumber),
                      onTap: () => Navigator.pop(context, conversation.id),
                    );
                  },
                );
              },
            ),
          ),
        ],
      ),
    ),
  );
}
