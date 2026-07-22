import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../constants/icon_fonts.dart';
import '../l10n/l10n.dart';
import '../models/message_list_entry.dart';
import 'message_action_policy.dart';

class MessageActionCallbacks {
  const MessageActionCallbacks({
    this.onReply,
    this.onCopyText,
    this.onCopyImage,
    this.onGenerateQr,
    this.onForward,
    this.onSelect,
    this.onTogglePin,
    this.onSaveAs,
    this.onAddSticker,
    this.onRecall,
    this.onDelete,
    this.onLocateToChat,
  });

  final VoidCallback? onReply;
  final ValueChanged<String>? onCopyText;
  final VoidCallback? onCopyImage;
  final ValueChanged<String>? onGenerateQr;
  final VoidCallback? onForward;
  final VoidCallback? onSelect;
  final VoidCallback? onTogglePin;
  final VoidCallback? onSaveAs;
  final VoidCallback? onAddSticker;
  final VoidCallback? onRecall;
  final VoidCallback? onDelete;
  final VoidCallback? onLocateToChat;
}

Menu? buildMessageActionsMenu({
  required BuildContext context,
  required MessageListEntry message,
  required MessageActionPolicy policy,
  required MessageActionCallbacks callbacks,
  String? selectedText,
  bool hasSelectedMessages = false,
}) {
  if (hasSelectedMessages) return null;

  final selectedContent = selectedText?.isNotEmpty == true
      ? selectedText
      : null;
  final groups = <List<MenuElement>>[
    [
      if (policy.canReply && callbacks.onReply != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.reply),
          title: context.l10n.reply,
          callback: callbacks.onReply!,
        ),
    ],
    _copyActions(
      context: context,
      message: message,
      selectedText: selectedContent,
      callbacks: callbacks,
    ),
    [
      if (policy.isPinnedPage && callbacks.onLocateToChat != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.positionToChat),
          title: context.l10n.locateToChat,
          callback: callbacks.onLocateToChat!,
        ),
      if (policy.canForward && callbacks.onForward != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.forward),
          title: context.l10n.forward,
          callback: callbacks.onForward!,
        ),
      if (policy.canSelect && callbacks.onSelect != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.select),
          title: context.l10n.select,
          callback: callbacks.onSelect!,
        ),
      if (policy.canPin && callbacks.onTogglePin != null)
        MenuAction(
          image: MenuImage.icon(
            message.pinned ? IconFonts.unPin : IconFonts.pin,
          ),
          title: message.pinned ? context.l10n.unpin : context.l10n.pinTitle,
          callback: callbacks.onTogglePin!,
        ),
    ],
    [
      if (policy.canSave && callbacks.onSaveAs != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.download),
          title: context.l10n.saveAs,
          callback: callbacks.onSaveAs!,
        ),
    ],
    [
      if ((policy.canAddSticker || policy.canAddImageAsSticker) &&
          callbacks.onAddSticker != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.sticker),
          title: context.l10n.addSticker,
          callback: callbacks.onAddSticker!,
        ),
    ],
    [
      if (policy.canRecall && callbacks.onRecall != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.recall),
          title: context.l10n.deleteForEveryone,
          callback: callbacks.onRecall!,
          attributes: const MenuActionAttributes(destructive: true),
        ),
      if (policy.canDelete && callbacks.onDelete != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.delete),
          title: context.l10n.deleteForMe,
          callback: callbacks.onDelete!,
          attributes: const MenuActionAttributes(destructive: true),
        ),
    ],
    [
      if (!kReleaseMode)
        MenuAction(
          image: MenuImage.icon(IconFonts.copy),
          title: 'Copy message',
          callback: () =>
              Clipboard.setData(ClipboardData(text: message.toString())),
        ),
    ],
  ];

  return Menu(children: _joinGroups(groups));
}

List<MenuElement> _copyActions({
  required BuildContext context,
  required MessageListEntry message,
  required String? selectedText,
  required MessageActionCallbacks callbacks,
}) {
  if (message.isPost && callbacks.onCopyText != null) {
    return [
      _copyTextAction(
        title: context.l10n.copy,
        content: message.content,
        onCopy: callbacks.onCopyText!,
      ),
    ];
  }

  if (message.isImage) {
    final actions = <MenuElement>[
      if (callbacks.onCopyImage != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.copy),
          title: context.l10n.copyImage,
          callback: callbacks.onCopyImage!,
        ),
    ];
    final caption = message.caption;
    if (caption?.trim().isNotEmpty == true && callbacks.onCopyText != null) {
      actions.add(
        _copyTextAction(
          title: selectedText == null
              ? context.l10n.copyText
              : context.l10n.copySelectedText,
          content: selectedText ?? caption!,
          onCopy: callbacks.onCopyText!,
        ),
      );
    }
    return actions;
  }

  if (message.isText) {
    final content = selectedText ?? message.content;
    return [
      if (callbacks.onCopyText != null)
        _copyTextAction(
          title: selectedText == null
              ? context.l10n.copy
              : context.l10n.copySelectedText,
          content: content,
          onCopy: callbacks.onCopyText!,
        ),
      if (callbacks.onGenerateQr != null)
        MenuAction(
          image: MenuImage.icon(Icons.qr_code),
          title: context.l10n.generateQrcode,
          callback: () => callbacks.onGenerateQr!(content),
        ),
    ];
  }

  if (message.category == 'APP_CARD' &&
      selectedText != null &&
      callbacks.onCopyText != null) {
    return [
      _copyTextAction(
        title: context.l10n.copySelectedText,
        content: selectedText,
        onCopy: callbacks.onCopyText!,
      ),
    ];
  }

  return const [];
}

MenuAction _copyTextAction({
  required String title,
  required String content,
  required ValueChanged<String> onCopy,
}) => MenuAction(
  image: MenuImage.icon(IconFonts.copy),
  title: title,
  callback: () => onCopy(content),
);

List<MenuElement> _joinGroups(List<List<MenuElement>> groups) {
  final children = <MenuElement>[];
  for (final group in groups.where((group) => group.isNotEmpty)) {
    if (children.isNotEmpty) children.add(MenuSeparator());
    children.addAll(group);
  }
  return children;
}
