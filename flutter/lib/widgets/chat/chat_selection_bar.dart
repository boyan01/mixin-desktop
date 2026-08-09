import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../../constants/assets.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../interactive_decorated_box.dart';
import '../message_action_policy.dart';
import '../mixin_dialog.dart';

class ChatSelectionBottomBar extends StatelessWidget {
  const ChatSelectionBottomBar({
    required this.selectedMessages,
    required this.currentUserId,
    required this.onCombineForward,
    required this.onForward,
    required this.onCopy,
    required this.onDelete,
    required this.onRecall,
    super.key,
  });

  final List<MessageListEntry> selectedMessages;
  final String currentUserId;
  final Future<void> Function(List<MessageListEntry>) onCombineForward;
  final Future<void> Function(List<MessageListEntry>) onForward;
  final Future<void> Function() onCopy;
  final Future<void> Function(List<MessageListEntry>) onDelete;
  final Future<void> Function(List<MessageListEntry>) onRecall;

  @override
  Widget build(BuildContext context) {
    final policies = selectedMessages
        .map(
          (message) => MessageActionPolicy(
            message: message,
            currentUserId: currentUserId,
            currentUserRole: null,
            now: DateTime.now(),
          ),
        )
        .toList(growable: false);
    final canForward =
        selectedMessages.length < 100 &&
        policies.every((item) => item.canForward);
    final canCombineForward =
        selectedMessages.length >= 2 &&
        selectedMessages.length < 100 &&
        policies.every((item) => item.canCombineForward);
    final canRecall =
        selectedMessages.length < 100 &&
        policies.every((item) => item.canRecall);

    return SizedBox(
      key: const Key('chat-selection-bar'),
      height: 80,
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: [
          _SelectionAction(
            label: context.l10n.combineAndForward,
            iconAssetName: MixinAssets.messageTranscriptForward,
            enabled: canCombineForward,
            onTap: canCombineForward
                ? () => onCombineForward(selectedMessages)
                : null,
          ),
          _SelectionAction(
            label: context.l10n.oneByOneForward,
            iconAssetName: MixinAssets.contextMenuForward,
            enabled: canForward,
            onTap: canForward ? () => onForward(selectedMessages) : null,
          ),
          _SelectionAction(
            label: context.l10n.copy,
            iconAssetName: MixinAssets.previewCopy,
            onTap: onCopy,
          ),
          _SelectionAction(
            label: context.l10n.delete,
            iconAssetName: MixinAssets.contextMenuDelete,
            onTap: () => _showSelectionDeleteDialog(
              context,
              selectedMessages: selectedMessages,
              canRecall: canRecall,
              onDelete: onDelete,
              onRecall: onRecall,
            ),
          ),
        ],
      ),
    );
  }
}

class _SelectionAction extends StatelessWidget {
  const _SelectionAction({
    required this.label,
    required this.iconAssetName,
    this.enabled = true,
    this.onTap,
  });

  final String label;
  final String iconAssetName;
  final bool enabled;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final active = enabled && onTap != null;
    Widget child = Center(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SvgPicture.asset(
              iconAssetName,
              width: 24,
              height: 24,
              colorFilter: ColorFilter.mode(
                context.theme.icon,
                BlendMode.srcIn,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: context.theme.text, fontSize: 14),
            ),
          ],
        ),
      ),
    );
    if (active) {
      child = InteractiveDecoratedBox.color(
        decoration: const BoxDecoration(),
        hoveringColor: context.dynamicColor(
          const Color.fromRGBO(0, 0, 0, 0.03),
          darkColor: const Color.fromRGBO(255, 255, 255, 0.2),
        ),
        onTap: onTap,
        child: child,
      );
    } else {
      child = Opacity(opacity: 0.5, child: child);
    }
    return Expanded(child: child);
  }
}

Future<void> _showSelectionDeleteDialog(
  BuildContext context, {
  required List<MessageListEntry> selectedMessages,
  required bool canRecall,
  required Future<void> Function(List<MessageListEntry>) onDelete,
  required Future<void> Function(List<MessageListEntry>) onRecall,
}) async {
  final action = await showConfirmMixinDialog(
    context,
    context.l10n.chatDeleteMessage(
      selectedMessages.length,
      selectedMessages.length,
    ),
    positiveText: context.l10n.delete,
    neutralText: canRecall ? context.l10n.deleteForEveryone : null,
  );
  switch (action) {
    case DialogEvent.positive:
      await onDelete(selectedMessages);
    case DialogEvent.neutral:
      await onRecall(selectedMessages);
    case null:
      return;
  }
}
