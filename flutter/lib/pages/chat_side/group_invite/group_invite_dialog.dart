import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import '../../../constants/assets.dart';
import '../../../l10n/l10n.dart';
import '../../../models/conversation_list_entry.dart';
import '../../../src/rust/desktop_api.dart' as rust;
import '../../../theme.dart';
import '../../../widgets/avatar_view.dart';
import '../../../widgets/buttons.dart';
import '../../../widgets/high_light_text.dart';
import '../../../widgets/interactive_decorated_box.dart';
import '../../../widgets/mixin_dialog.dart';
import '../../../widgets/show_forward_conversation_selector.dart';
import '../../../widgets/toast.dart';

Future<void> showGroupInviteByLinkDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required ConversationListEntry conversation,
}) => showMixinDialog<void>(
  context: context,
  child: _GroupInviteByLinkDialog(account: account, conversation: conversation),
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

  Future<rust.ConversationDetailItem> _load() => widget.account
      .conversation()
      .conversationDetail(conversationId: widget.conversation.id);

  Future<void> _run(
    Future<void> Function() action, {
    bool refresh = false,
  }) async {
    if (acting) return;
    setState(() => acting = true);
    final succeeded = await runWithToast(action);
    if (!mounted) return;
    setState(() {
      acting = false;
      if (succeeded && refresh) detail = _load();
    });
  }

  Future<void> _share(String codeUrl) async {
    final target = await showConversationSelector(
      context,
      account: widget.account,
      title: context.l10n.forward,
    );
    if (target == null) return;
    await _run(
      () => widget.account
          .message()
          .sendText(
            conversationId: target.id,
            content: codeUrl,
            silent: false,
          )
          .then((_) {}),
    );
  }

  @override
  Widget build(BuildContext context) => Material(
    color: context.theme.popUp,
    child: SizedBox(
      width: 480,
      height: 600,
      child: Stack(
        fit: StackFit.passthrough,
        children: [
          FutureBuilder<rust.ConversationDetailItem>(
            future: detail,
            builder: (context, snapshot) {
              if (!snapshot.hasData) return const SizedBox();
              final detail = snapshot.data!;
              return Column(
                children: [
                  const SizedBox(height: 120),
                  ConversationAvatarView(
                    conversation: widget.conversation,
                    size: 90,
                  ),
                  const SizedBox(height: 16),
                  Text(
                    detail.name,
                    style: TextStyle(
                      fontSize: 18,
                      color: context.theme.text,
                      fontWeight: FontWeight.w600,
                      height: 1.5,
                    ),
                  ),
                  const SizedBox(height: 12),
                  SizedBox(
                    width: 320,
                    child: CustomSelectableText(
                      detail.codeUrl,
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        fontSize: 14,
                        color: context.theme.text,
                        fontWeight: FontWeight.w400,
                        height: 1.5,
                      ),
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
                        iconAssetName: MixinAssets.inviteShare,
                        label: context.l10n.shareLink,
                        onTap: detail.codeUrl.isEmpty
                            ? null
                            : () => _share(detail.codeUrl),
                      ),
                      _ActionButton(
                        iconAssetName: MixinAssets.inviteCopy,
                        label: context.l10n.copyInvite,
                        onTap: detail.codeUrl.isEmpty
                            ? null
                            : () async {
                                await Clipboard.setData(
                                  ClipboardData(text: detail.codeUrl),
                                );
                                showToastSuccessful();
                              },
                      ),
                      _ActionButton(
                        iconAssetName: MixinAssets.inviteRefresh,
                        label: context.l10n.resetLink,
                        onTap: () => _run(
                          () => widget.account.conversation().rotateGroupInvite(
                            conversationId: widget.conversation.id,
                          ),
                          refresh: true,
                        ),
                      ),
                    ],
                  ),
                ],
              );
            },
          ),
          Align(
            alignment: Alignment.topCenter,
            child: Padding(
              padding: const EdgeInsets.only(top: 30),
              child: Text(
                context.l10n.inviteToGroupViaLink,
                style: TextStyle(
                  fontSize: 18,
                  color: context.theme.text,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          const Align(
            alignment: Alignment.topRight,
            child: Padding(
              padding: EdgeInsets.only(right: 30, top: 20),
              child: MixinCloseButton(),
            ),
          ),
        ],
      ),
    ),
  );
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.iconAssetName,
    required this.label,
    required this.onTap,
  });

  final String iconAssetName;
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox.color(
    onTap: onTap,
    decoration: const BoxDecoration(),
    hoveringColor: Theme.of(context).brightness == Brightness.light
        ? const Color.fromRGBO(0, 0, 0, 0.03)
        : const Color.fromRGBO(255, 255, 255, 0.2),
    child: Padding(
      padding: const EdgeInsets.all(8),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SvgPicture.asset(
            iconAssetName,
            width: 24,
            height: 24,
            colorFilter: ColorFilter.mode(context.theme.icon, BlendMode.srcIn),
          ),
          const SizedBox(height: 15),
          Text(
            label,
            style: TextStyle(color: context.theme.text, fontSize: 14),
          ),
        ],
      ),
    ),
  );
}
