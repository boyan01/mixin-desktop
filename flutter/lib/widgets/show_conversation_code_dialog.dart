import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';

Future<String?> showConversationCodeDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required rust.CodeResult result,
  required String code,
}) => showMixinDialog<String>(
  context: context,
  child: _ConversationCodeDialog(account: account, result: result, code: code),
);

class _ConversationCodeDialog extends StatefulWidget {
  const _ConversationCodeDialog({
    required this.account,
    required this.result,
    required this.code,
  });

  final rust.AccountHandle account;
  final rust.CodeResult result;
  final String code;

  @override
  State<_ConversationCodeDialog> createState() =>
      _ConversationCodeDialogState();
}

class _ConversationCodeDialogState extends State<_ConversationCodeDialog> {
  bool joining = false;

  Future<void> _join() async {
    if (joining) return;
    joining = true;
    String? conversationId;
    final succeeded = await runWithToast(() async {
      conversationId = await widget.account.conversation().joinGroup(
        code: widget.code,
      );
    });
    joining = false;
    if (mounted && succeeded) Navigator.pop(context, conversationId);
  }

  @override
  Widget build(BuildContext context) {
    final avatars = widget.result.participantAvatars
        .map(
          (avatar) => ConversationAvatarEntry(
            userId: avatar.userId,
            name: avatar.name,
            avatarUrl: avatar.avatarUrl,
          ),
        )
        .toList(growable: false);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          width: 340,
          child: Column(
            children: [
              const Row(
                children: [
                  Spacer(),
                  Padding(
                    padding: EdgeInsets.only(right: 12, top: 12),
                    child: MixinCloseButton(),
                  ),
                ],
              ),
              ClipOval(
                child: SizedBox.square(
                  dimension: 90,
                  child: AvatarPuzzlesView(avatars: avatars, size: 90),
                ),
              ),
              const SizedBox(height: 8),
              CustomSelectableText(
                widget.result.conversationName ?? '',
                style: TextStyle(
                  color: context.theme.text,
                  fontSize: 16,
                  height: 1,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 4),
              CustomSelectableText(
                context.l10n.participantsCount(
                  widget.result.participantCount.toInt(),
                ),
                style: TextStyle(
                  color: context.theme.secondaryText,
                  fontSize: 12,
                ),
              ),
              const SizedBox(height: 8),
              DialogAddOrJoinButton(
                onTap: _join,
                title: Text(context.l10n.joinGroupWithPlus),
              ),
              const SizedBox(height: 56),
            ],
          ),
        ),
      ],
    );
  }
}
