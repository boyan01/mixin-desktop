import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';

Future<void> showGroupInviteByLinkDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required ConversationListEntry conversation,
}) => showDialog<void>(
  context: context,
  builder: (context) =>
      _GroupInviteByLinkDialog(account: account, conversation: conversation),
);

class _GroupInviteByLinkDialog extends StatefulWidget {
  const _GroupInviteByLinkDialog({
    required this.account,
    required this.conversation,
  });

  final rust.AccountHandle account;
  final ConversationListEntry conversation;

  @override
  State<_GroupInviteByLinkDialog> createState() =>
      _GroupInviteByLinkDialogState();
}

class _GroupInviteByLinkDialogState extends State<_GroupInviteByLinkDialog> {
  late Future<rust.ConversationDetailItem> detail = _load();
  bool acting = false;
  Object? error;

  Future<rust.ConversationDetailItem> _load() =>
      widget.account.conversationDetail(conversationId: widget.conversation.id);

  Future<void> _run(Future<void> Function() action) async {
    if (acting) return;
    setState(() {
      acting = true;
      error = null;
    });
    try {
      await action();
      if (mounted) setState(() => detail = _load());
    } on Object catch (exception) {
      if (mounted) setState(() => error = exception);
    } finally {
      if (mounted) setState(() => acting = false);
    }
  }

  Future<void> _share(String codeUrl) async {
    final target = await showForwardConversationSelector(
      context,
      account: widget.account,
    );
    if (target == null) return;
    await _run(
      () => widget.account
          .sendText(
            conversationId: target,
            content: codeUrl,
            quoteMessageId: null,
          )
          .then((_) {}),
    );
  }

  @override
  Widget build(BuildContext context) => Dialog(
    clipBehavior: Clip.antiAlias,
    child: Material(
      color: context.theme.popUp,
      child: SizedBox(
        width: 480,
        height: 600,
        child: FutureBuilder<rust.ConversationDetailItem>(
          future: detail,
          builder: (context, snapshot) {
            if (!snapshot.hasData) {
              return snapshot.hasError
                  ? Center(child: Text(snapshot.error.toString()))
                  : const Center(child: CircularProgressIndicator());
            }
            final detail = snapshot.data!;
            return Stack(
              children: [
                Column(
                  children: [
                    const SizedBox(height: 30),
                    Text(
                      context.l10n.inviteToGroupViaLink,
                      style: const TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 70),
                    ConversationAvatarView(
                      conversation: widget.conversation,
                      size: 90,
                    ),
                    const SizedBox(height: 16),
                    Text(
                      detail.name,
                      style: const TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 12),
                    SizedBox(
                      width: 320,
                      child: SelectableText(
                        detail.codeUrl,
                        textAlign: TextAlign.center,
                      ),
                    ),
                    const SizedBox(height: 8),
                    SizedBox(
                      width: 338,
                      child: Text(
                        context.l10n.inviteInfo,
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: context.theme.secondaryText,
                          fontSize: 12,
                          height: 1.5,
                        ),
                      ),
                    ),
                    const SizedBox(height: 61),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                      children: [
                        _ActionButton(
                          icon: Icons.share_outlined,
                          label: context.l10n.shareLink,
                          onTap: detail.codeUrl.isEmpty
                              ? null
                              : () => _share(detail.codeUrl),
                        ),
                        _ActionButton(
                          icon: Icons.copy_outlined,
                          label: context.l10n.copyInvite,
                          onTap: detail.codeUrl.isEmpty
                              ? null
                              : () => Clipboard.setData(
                                  ClipboardData(text: detail.codeUrl),
                                ),
                        ),
                        _ActionButton(
                          icon: Icons.refresh,
                          label: context.l10n.resetLink,
                          onTap: () => _run(
                            () => widget.account.rotateGroupInvite(
                              conversationId: widget.conversation.id,
                            ),
                          ),
                        ),
                      ],
                    ),
                    if (error != null)
                      Padding(
                        padding: const EdgeInsets.all(16),
                        child: Text(
                          error.toString(),
                          style: const TextStyle(color: Colors.red),
                        ),
                      ),
                  ],
                ),
                Positioned(
                  right: 20,
                  top: 20,
                  child: IconButton(
                    onPressed: () => Navigator.pop(context),
                    icon: const Icon(Icons.close),
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
            );
          },
        ),
      ),
    ),
  );
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => InkWell(
    onTap: onTap,
    borderRadius: BorderRadius.circular(8),
    child: Padding(
      padding: const EdgeInsets.all(8),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, color: context.theme.icon),
          const SizedBox(height: 15),
          Text(label),
        ],
      ),
    ),
  );
}
