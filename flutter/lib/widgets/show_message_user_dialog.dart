import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../controllers/conversation_list_controller.dart';
import '../l10n/generated/app_localizations.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import '../utils/app_logger.dart';
import 'action_button.dart';
import 'avatar_view.dart';
import 'badges_widget.dart';
import 'buttons.dart';
import 'custom_popup_menu.dart';
import 'high_light_text.dart';
import 'mixin_dialog.dart';
import 'more_extended_text.dart';
import 'show_forward_conversation_selector.dart';
import 'toast.dart';

enum MessageUserAction { chat, information }

void openMessageUserDialog(
  BuildContext context, {
  String? userId,
  String? identityNumber,
}) {
  unawaited(
    showMessageUserDialog(
      context,
      account: context.read<rust.AccountHandle>(),
      userId: userId,
      identityNumber: identityNumber,
    ),
  );
}

class MessageUserDialogResult {
  const MessageUserDialogResult({required this.profile, required this.action});

  final rust.UserProfileItem profile;
  final MessageUserAction action;
}

Future<void> handleMessageUserDialogResult(
  BuildContext context, {
  required rust.AccountHandle account,
  required MessageUserDialogResult result,
  required ValueChanged<ConversationListEntry> onSelectConversation,
  ValueChanged<ConversationListEntry>? onSelectConversationInfo,
}) async {
  final conversationId = await account.conversation().openUserConversation(
    userId: result.profile.userId,
  );
  if (!context.mounted) return;
  final controller = context.read<ConversationListController>();
  await controller.refresh();
  final conversation = await controller.findConversation(conversationId);
  if (!context.mounted || conversation == null) return;
  if (result.action == MessageUserAction.information &&
      onSelectConversationInfo != null) {
    onSelectConversationInfo(conversation);
  } else {
    onSelectConversation(conversation);
  }
}

Future<MessageUserDialogResult?> showMessageUserDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  String? userId,
  String? identityNumber,
}) async {
  assert(userId != null || identityNumber != null);
  if (userId == null && identityNumber == null) return null;
  rust.UserProfileItem? profile;
  try {
    profile = await account.user().userProfile(
      userId: userId,
      identityNumber: identityNumber,
    );
    if (profile == null) {
      showToastLoading();
      profile = userId != null
          ? await account.user().refreshUserProfile(userId: userId)
          : await account.user().searchUser(query: identityNumber!);
      Toast.dismiss();
    }
  } on Object catch (error, stackTrace) {
    e('Load message user profile failed', error, stackTrace);
    showToastFailed(error);
    return null;
  }
  if (profile == null) {
    showToastFailed(ToastError.builder((context) => context.l10n.userNotFound));
    return null;
  }
  if (!context.mounted) return null;
  return showMixinDialog<MessageUserDialogResult>(
    context: context,
    child: MessageUserDialog(account: account, profile: profile),
  );
}

class MessageUserDialog extends HookWidget {
  const MessageUserDialog({
    required this.account,
    required this.profile,
    super.key,
  });

  final rust.AccountHandle account;
  final rust.UserProfileItem profile;

  @override
  Widget build(BuildContext context) {
    final currentProfile = useState(profile);

    useEffect(() {
      var disposed = false;
      unawaited(() async {
        try {
          final refreshed = await account.user().refreshUserProfile(
            userId: profile.userId,
          );
          if (!disposed && refreshed != null) currentProfile.value = refreshed;
        } on Object catch (error, stackTrace) {
          e('Refresh message user profile failed', error, stackTrace);
        }
      }());
      return () => disposed = true;
    }, [account, profile.userId]);

    return SizedBox(
      width: 340,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Align(
            alignment: Alignment.centerRight,
            child: Padding(
              padding: EdgeInsets.only(right: 8, top: 8),
              child: MixinCloseButton(
                key: Key('message-user-dialog-close'),
              ),
            ),
          ),
          _ProfileBody(account: account, profile: currentProfile.value),
        ],
      ),
    );
  }
}

class _ProfileBody extends StatefulWidget {
  const _ProfileBody({required this.account, required this.profile});

  final rust.AccountHandle account;
  final rust.UserProfileItem profile;

  @override
  State<_ProfileBody> createState() => _ProfileBodyState();
}

class _ProfileBodyState extends State<_ProfileBody> {
  late String relationship = widget.profile.relationship.toUpperCase();
  bool updating = false;

  Future<void> _updateRelationship() async {
    if (updating) return;
    updating = true;
    final succeeded = await runWithToast(() async {
      if (relationship == 'FRIEND') {
        await widget.account.user().removeContact(
          userId: widget.profile.userId,
        );
        relationship = 'STRANGER';
      } else if (relationship == 'BLOCKED') {
        await widget.account.user().unblockUser(userId: widget.profile.userId);
        relationship = 'STRANGER';
      } else {
        await widget.account.user().addContact(
          userId: widget.profile.userId,
          fullName: widget.profile.fullName,
        );
        relationship = 'FRIEND';
      }
    });
    updating = false;
    if (mounted && succeeded) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final profile = widget.profile;
    final anonymous = profile.identityNumber == '0';
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return AnimatedSize(
      duration: const Duration(milliseconds: 150),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          GestureDetector(
            onTap: () {
              final keys = HardwareKeyboard.instance.logicalKeysPressed;
              if (keys.contains(LogicalKeyboardKey.altLeft) ||
                  keys.contains(LogicalKeyboardKey.altRight)) {
                Clipboard.setData(
                  ClipboardData(text: 'mixin://users/${profile.userId}'),
                );
              }
            },
            child: AvatarView(
              userId: profile.userId,
              name: profile.fullName,
              avatarUrl: profile.avatarUrl,
              size: 90,
            ),
          ),
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 32),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Flexible(
                  child: CustomSelectableArea(
                    child: CustomText(
                      profile.fullName,
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        color: context.theme.text,
                        fontSize: 16,
                        height: 1,
                      ),
                    ),
                  ),
                ),
                if (!anonymous)
                  BadgesWidget(
                    verified: profile.isVerified,
                    isBot: profile.isBot,
                    membership: profile.membership,
                  ),
              ],
            ),
          ),
          if (!anonymous)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: CustomSelectableText(
                l10n?.contactMixinId(profile.identityNumber) ??
                    'Mixin ID: ${profile.identityNumber}',
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: context.theme.secondaryText,
                  fontSize: 12,
                ),
              ),
            ),
          if (!anonymous && relationship == 'STRANGER')
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: DialogAddOrJoinButton(
                onTap: _updateRelationship,
                title: Text(
                  profile.isBot
                      ? context.l10n.addBotWithPlus
                      : context.l10n.addContactWithPlus,
                ),
              ),
            ),
          if (profile.biography.trim().isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 20),
              child: ConstrainedBox(
                constraints: const BoxConstraints(
                  maxHeight: 120,
                  minWidth: 160,
                ),
                child: SingleChildScrollView(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 36),
                    child: MoreExtendedText(
                      profile.biography,
                      style: TextStyle(
                        color: context.theme.text,
                        fontSize: 14,
                        height: 1.5,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          if (!anonymous)
            Padding(
              padding: const EdgeInsets.only(top: 24),
              child: _UserProfileButtonBar(
                account: widget.account,
                profile: profile,
              ),
            ),
          const SizedBox(height: 56),
        ],
      ),
    );
  }
}

class _UserProfileButtonBar extends StatelessWidget {
  const _UserProfileButtonBar({required this.account, required this.profile});

  final rust.AccountHandle account;
  final rust.UserProfileItem profile;

  Future<void> _share(BuildContext context) async {
    final conversation = await showConversationSelector(
      context,
      account: account,
      title: context.l10n.shareContact,
      action: CustomPopupMenuButton<Object?>(
        alignment: Alignment.bottomCenter,
        color: context.theme.icon,
        icon: MixinAssets.inviteShare,
        onSelected: (_) =>
            Clipboard.setData(ClipboardData(text: profile.codeUrl)),
        itemBuilder: (context) => [
          CustomPopupMenuItem(
            icon: MixinAssets.contextMenuCopy,
            title: context.l10n.copyLink,
            value: null,
          ),
        ],
      ),
    );
    if (conversation == null) return;
    await runFutureWithToast(
      account.message().sendContact(
        conversationId: conversation.id,
        sharedUserId: profile.userId,
        silent: false,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final isSelf = profile.userId == account.accountId();
    final children = [
      ActionButton(
        name: MixinAssets.inviteShare,
        size: 30,
        color: context.theme.icon,
        onTap: () => _share(context),
      ),
      if (!isSelf)
        ActionButton(
          name: MixinAssets.chatSmall,
          size: 30,
          color: context.theme.icon,
          onTap: () => Navigator.pop(
            context,
            MessageUserDialogResult(
              profile: profile,
              action: MessageUserAction.chat,
            ),
          ),
        ),
      if (!isSelf)
        ActionButton(
          name: MixinAssets.information,
          size: 30,
          color: context.theme.icon,
          onTap: () => Navigator.pop(
            context,
            MessageUserDialogResult(
              profile: profile,
              action: MessageUserAction.information,
            ),
          ),
        ),
    ];
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 45),
      child: Row(
        mainAxisAlignment: children.length == 1
            ? MainAxisAlignment.center
            : MainAxisAlignment.spaceBetween,
        children: children,
      ),
    );
  }
}
