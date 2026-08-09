import 'dart:convert';

import 'package:flutter/material.dart';

import '../../l10n/generated/app_localizations.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../high_light_text.dart';
import '../message_style.dart';
import 'app_button_group_message_item.dart';
import 'app_card_message_item.dart';
import 'message_preview_utils.dart';

class PinMessageItem extends StatelessWidget {
  const PinMessageItem({
    required this.message,
    this.mentionNames = const {},
    super.key,
  });

  final MessageListEntry message;
  final Map<String, String> mentionNames;

  @override
  Widget build(BuildContext context) {
    final pin = _PinData.parse(message.content);
    var preview = pin?.preview(context.l10n) ?? context.l10n.aMessage;
    for (final entry in mentionNames.entries) {
      preview = preview.replaceAll('@${entry.key}', '@${entry.value}');
    }
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 400),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: context.dynamicColor(
                const Color.fromRGBO(202, 234, 201, 1),
              ),
              borderRadius: const BorderRadius.all(Radius.circular(10)),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 5, horizontal: 10),
              child: CustomText(
                context.l10n.chatPinMessage(message.senderName, preview),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: context.messageStyle.secondaryFontSize,
                  color: context.dynamicColor(const Color.fromRGBO(0, 0, 0, 1)),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _PinData {
  const _PinData(this.messageId, this.category, this.content);
  final String? messageId;
  final String category;
  final String content;

  String preview(AppLocalizations l10n) {
    if (category.endsWith('_TEXT')) return content.trim();
    final preview = _pinCategoryPreview(l10n, category, content);
    return preview.isEmpty ? preview : ': $preview';
  }

  static _PinData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _PinData(
        nonEmpty(json['message_id']),
        json['category']?.toString() ?? '',
        json['content']?.toString() ?? '',
      );
    } on Object {
      return null;
    }
  }
}

String _pinCategoryPreview(
  AppLocalizations l10n,
  String category,
  String content,
) {
  if (category.endsWith('_POST')) {
    return content.trim().isEmpty
        ? l10n.post
        : postOptimizeMarkdown(content.trim());
  }
  if (category == 'APP_BUTTON_GROUP') {
    try {
      return (jsonDecode(content) as List<dynamic>)
          .map(
            (item) =>
                '[${ActionData.fromJson(item as Map<String, dynamic>).label}]',
          )
          .join();
    } on Object {
      return '';
    }
  }
  if (category == 'APP_CARD') {
    final title = AppCardData.parse(content)?.title;
    return '[${title ?? l10n.card}]';
  }
  if (category.startsWith('WEBRTC') || category.startsWith('KRAKEN')) {
    return l10n.contentVoice;
  }
  if (category == 'MESSAGE_RECALL') {
    return '[${l10n.thisMessageWasDeleted}]';
  }
  return categoryPreview(l10n, category, content);
}

String pinMessagePreview(AppLocalizations l10n, String raw) =>
    _PinData.parse(raw)?.preview(l10n) ?? l10n.aMessage;
