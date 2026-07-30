import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:provider/provider.dart';

import '../controllers/conversation_list_controller.dart';
import '../controllers/home_navigation_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import '../utils/app_logger.dart';
import '../widgets/avatar_view.dart';
import '../widgets/mixin_dialog.dart';
import '../widgets/show_forward_conversation_selector.dart';
import '../widgets/show_message_user_dialog.dart';
import '../widgets/toast.dart';
import 'conversation_info_destination.dart';

Future<void> showSearchContactDialog(BuildContext context) async {
  final result = await showMixinDialog<MessageUserDialogResult>(
    context: context,
    child: const _SearchUserDialog(),
  );
  if (!context.mounted || result == null) return;
  await handleConversationUserDialogResult(context, result);
}

Future<void> showConversationUser(
  BuildContext context, {
  required String userId,
}) async {
  final result = await showMessageUserDialog(
    context,
    account: context.read<AccountHandle>(),
    userId: userId,
  );
  if (!context.mounted || result == null) return;
  await handleConversationUserDialogResult(context, result);
}

Future<void> handleConversationUserDialogResult(
  BuildContext context,
  MessageUserDialogResult result,
) => handleMessageUserDialogResult(
  context,
  account: context.read<AccountHandle>(),
  result: result,
  onSelectConversation: context.read<HomeNavigationController>().select,
  onSelectConversationInfo: (conversation) {
    context.read<HomeNavigationController>()
      ..select(conversation)
      ..chatSideController.openDestination(
        ConversationInfoDestination.infoPage,
      );
  },
);

Future<void> showCreateGroupDialog(BuildContext context) async {
  final account = context.read<AccountHandle>();
  final selected = await showConversationMultiSelector(
    context,
    account: account,
    title: context.l10n.createGroup,
  );
  if (!context.mounted || selected == null || selected.isEmpty) return;
  final name = await showMixinDialog<String>(
    context: context,
    child: _NewGroupConfirm(
      profile: account.profile(),
      selected: selected,
    ),
  );
  if (!context.mounted || name == null || name.isEmpty) return;
  try {
    await account.conversation().createGroup(
      name: name.trim(),
      userIds: selected.map((conversation) => conversation.ownerId).toList(),
    );
    if (context.mounted) {
      await context.read<ConversationListController>().refresh();
    }
  } on Object catch (error, stackTrace) {
    e('Create group failed', error, stackTrace);
    if (context.mounted) showToastFailed(null);
  }
}

Future<void> showCreateCircleDialog(BuildContext context) async {
  final account = context.read<AccountHandle>();
  final name = await showMixinDialog<String>(
    context: context,
    child: EditDialog(
      title: Text(context.l10n.circles),
      hintText: context.l10n.editCircleName,
      maxLength: 64,
    ),
  );
  if (!context.mounted || name == null || name.isEmpty) return;
  final selected = await showConversationMultiSelector(
    context,
    account: account,
    title: context.l10n.createCircle,
    category: ConversationCategoryFilter.chats,
    allowEmpty: true,
  );
  if (!context.mounted || selected == null) return;
  try {
    final circle = await account.conversation().createCircle(name: name.trim());
    for (final conversation in selected) {
      await account.conversation().editCircleConversation(
        circleId: circle.circleId,
        conversationId: conversation.id,
        ownerId: conversation.ownerId,
        isGroup: conversation.isGroup,
        add: true,
      );
    }
    if (context.mounted) {
      await context.read<ConversationListController>().refresh();
    }
  } on Object catch (error, stackTrace) {
    e('Create circle failed', error, stackTrace);
    if (context.mounted) showToastFailed(null);
  }
}

class _SearchIntent extends Intent {
  const _SearchIntent();
}

class _SearchUserDialog extends HookWidget {
  const _SearchUserDialog();

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();
    useListenable(controller);
    final profile = useState<UserProfileItem?>(null);
    final loading = useState(false);
    final account = context.read<AccountHandle>();
    final searchable = controller.text.trim().length > 3;
    final currentIdentityNumber = account.profile().identityNumber;

    Future<void> search() async {
      if (!searchable || loading.value) return;
      loading.value = true;
      try {
        profile.value = await account.user().searchUser(query: controller.text);
      } on Object catch (error, stackTrace) {
        e('Search user dialog failed', error, stackTrace);
        if (context.mounted) {
          showToastFailed(ToastError(context.l10n.userNotFound));
        }
      } finally {
        if (context.mounted) loading.value = false;
      }
    }

    return AnimatedSize(
      duration: const Duration(milliseconds: 200),
      child: profile.value != null
          ? MessageUserDialog(
              account: account,
              profile: profile.value!,
            )
          : Stack(
              children: [
                Visibility(
                  visible: !loading.value,
                  maintainSize: true,
                  maintainAnimation: true,
                  maintainState: true,
                  child: AlertDialogLayout(
                    title: Text(context.l10n.addContact),
                    content: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        FocusableActionDetector(
                          shortcuts: {
                            if (searchable)
                              const SingleActivator(LogicalKeyboardKey.enter):
                                  const _SearchIntent(),
                          },
                          actions: {
                            _SearchIntent: CallbackAction<_SearchIntent>(
                              onInvoke: (_) => search(),
                            ),
                          },
                          child: DialogTextField(
                            textEditingController: controller,
                            hintText: context.l10n.addPeopleSearchHint,
                            inputFormatters: [
                              FilteringTextInputFormatter.allow(
                                RegExp('[0-9+]'),
                              ),
                              LengthLimitingTextInputFormatter(128),
                            ],
                          ),
                        ),
                        if (currentIdentityNumber.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(top: 8),
                            child: Text(
                              context.l10n.myMixinId(currentIdentityNumber),
                              style: TextStyle(
                                color: context.theme.secondaryText,
                                fontSize: 12,
                              ),
                            ),
                          ),
                      ],
                    ),
                    actions: [
                      MixinButton(
                        backgroundTransparent: true,
                        onTap: () => Navigator.pop(context),
                        child: Text(context.l10n.cancel),
                      ),
                      MixinButton(
                        disable: !searchable,
                        onTap: search,
                        child: Text(context.l10n.search),
                      ),
                    ],
                  ),
                ),
                if (loading.value)
                  Positioned.fill(
                    child: Center(
                      child: CircularProgressIndicator(
                        color: context.theme.accent,
                      ),
                    ),
                  ),
              ],
            ),
    );
  }
}

class _NewGroupConfirm extends HookWidget {
  const _NewGroupConfirm({required this.profile, required this.selected});

  final AccountProfile profile;
  final List<ConversationListEntry> selected;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();
    useListenable(controller);
    final avatars = [
      ConversationAvatarEntry(
        userId: profile.userId,
        name: profile.fullName,
        avatarUrl: profile.avatarUrl,
      ),
      ...selected.map(
        (conversation) => ConversationAvatarEntry(
          userId: conversation.ownerId,
          name: conversation.name,
          avatarUrl: conversation.avatarUrl,
        ),
      ),
    ].take(4).toList(growable: false);
    return AlertDialogLayout(
      title: Text(context.l10n.groups),
      titleMarginBottom: 24,
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          ClipOval(child: AvatarPuzzlesView(avatars: avatars, size: 60)),
          const SizedBox(height: 8),
          Text(
            context.l10n.participantsCount(selected.length + 1),
            style: TextStyle(fontSize: 14, color: context.theme.secondaryText),
          ),
          const SizedBox(height: 48),
          DialogTextField(
            textEditingController: controller,
            hintText: context.l10n.groupName,
            maxLength: 40,
          ),
        ],
      ),
      actions: [
        MixinButton(
          backgroundTransparent: true,
          onTap: () => Navigator.pop(context),
          child: Text(context.l10n.cancel),
        ),
        MixinButton(
          disable: controller.text.isEmpty,
          onTap: () => Navigator.pop(context, controller.text),
          child: Text(context.l10n.create),
        ),
      ],
    );
  }
}
