import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../constants/icon_fonts.dart';
import '../l10n/l10n.dart';
import '../models/message_list_entry.dart';
import '../utils/system_clipboard.dart';
import 'message_action_policy.dart';
import 'message_media_preview_pages.dart';
import 'message_qr_dialog.dart';

Menu? buildMessageActionsMenu({
  required BuildContext context,
  required MessageListEntry message,
  required MessageActionPolicy policy,
  VoidCallback? onReply,
  VoidCallback? onForward,
  VoidCallback? onSelect,
  VoidCallback? onTogglePin,
  VoidCallback? onAddSticker,
  VoidCallback? onRecall,
  VoidCallback? onDelete,
  VoidCallback? onLocateToChat,
  String? selectedText,
  bool hasSelectedMessages = false,
}) {
  if (hasSelectedMessages) return null;

  final selectedContent = selectedText?.isNotEmpty == true
      ? selectedText
      : null;
  final groups = <List<MenuElement>>[
    [
      if (policy.canReply && onReply != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.reply),
          title: context.l10n.reply,
          callback: onReply,
        ),
    ],
    _copyActions(
      context: context,
      message: message,
      policy: policy,
      selectedText: selectedContent,
    ),
    [
      if (policy.isPinnedPage && onLocateToChat != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.positionToChat),
          title: context.l10n.locateToChat,
          callback: onLocateToChat,
        ),
      if (policy.canForward && onForward != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.forward),
          title: context.l10n.forward,
          callback: onForward,
        ),
      if (policy.canSelect && onSelect != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.select),
          title: context.l10n.select,
          callback: onSelect,
        ),
      if (policy.canPin && onTogglePin != null)
        MenuAction(
          image: MenuImage.icon(
            message.pinned ? IconFonts.unPin : IconFonts.pin,
          ),
          title: message.pinned ? context.l10n.unpin : context.l10n.pinTitle,
          callback: onTogglePin,
        ),
    ],
    [
      if (policy.canSave && message.mediaUrl?.trim().isNotEmpty == true)
        MenuAction(
          image: MenuImage.icon(IconFonts.download),
          title: context.l10n.saveAs,
          callback: () => unawaited(
            saveMessageFileAs(
              message.mediaUrl!,
              suggestedName: message.mediaName,
            ),
          ),
        ),
    ],
    [
      if ((policy.canAddSticker || policy.canAddImageAsSticker) &&
          onAddSticker != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.sticker),
          title: context.l10n.addSticker,
          callback: onAddSticker,
        ),
    ],
    [
      if (policy.canRecall && onRecall != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.recall),
          title: context.l10n.deleteForEveryone,
          callback: onRecall,
          attributes: const MenuActionAttributes(destructive: true),
        ),
      if (policy.canDelete && onDelete != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.delete),
          title: context.l10n.deleteForMe,
          callback: onDelete,
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
  required MessageActionPolicy policy,
  required String? selectedText,
}) {
  if (message.isPost) {
    return [
      _copyTextAction(
        title: context.l10n.copy,
        content: message.content,
      ),
    ];
  }

  if (message.isImage) {
    final copyableImage =
        (policy.isPinnedPage ||
            policy.isTranscriptPage ||
            const {
              'DONE',
              'READ',
            }.contains(message.mediaStatus.toUpperCase()))
        ? existingLocalFile(message.mediaUrl)
        : null;
    final actions = <MenuElement>[
      if (copyableImage != null)
        MenuAction(
          image: MenuImage.icon(IconFonts.copy),
          title: context.l10n.copyImage,
          callback: () => unawaited(copyLocalFileToClipboard(copyableImage)),
        ),
    ];
    final caption = message.caption;
    if (caption?.trim().isNotEmpty == true) {
      actions.add(
        _copyTextAction(
          title: selectedText == null
              ? context.l10n.copyText
              : context.l10n.copySelectedText,
          content: selectedText ?? caption!,
        ),
      );
    }
    return actions;
  }

  if (message.isText) {
    final content = selectedText ?? message.content;
    return [
      _copyTextAction(
        title: selectedText == null
            ? context.l10n.copy
            : context.l10n.copySelectedText,
        content: content,
      ),
      MenuAction(
        image: MenuImage.icon(Icons.qr_code),
        title: context.l10n.generateQrcode,
        callback: () => showMessageQrDialog(context, content: content),
      ),
    ];
  }

  if (message.category == 'APP_CARD' && selectedText != null) {
    return [
      _copyTextAction(
        title: context.l10n.copySelectedText,
        content: selectedText,
      ),
    ];
  }

  return const [];
}

MenuAction _copyTextAction({
  required String title,
  required String content,
}) => MenuAction(
  image: MenuImage.icon(IconFonts.copy),
  title: title,
  callback: () => Clipboard.setData(ClipboardData(text: content)),
);

List<MenuElement> _joinGroups(List<List<MenuElement>> groups) {
  final children = <MenuElement>[];
  for (final group in groups.where((group) => group.isNotEmpty)) {
    if (children.isNotEmpty) children.add(MenuSeparator());
    children.addAll(group);
  }
  return children;
}
