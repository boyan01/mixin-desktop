import 'dart:convert';

import 'package:flutter/widgets.dart';

import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart';

String notificationPreview(BuildContext context, NotificationEvent message) {
  final category = message.category;
  if (category == 'MESSAGE_PIN') {
    return _pinNotificationPreview(context, message);
  }
  final text = _notificationContentPreview(context, category, message.content);
  return message.conversationCategory == 'GROUP'
      ? '${message.senderName}: $text'
      : text;
}

String _pinNotificationPreview(
  BuildContext context,
  NotificationEvent message,
) {
  try {
    final value = jsonDecode(message.content) as Map<String, dynamic>;
    final category = value['category']?.toString();
    if (category == null) throw const FormatException('missing pin category');
    final content = value['content']?.toString() ?? '';
    return context.l10n.chatPinMessage(
      message.senderName,
      _notificationContentPreview(context, category, content),
    );
  } on Object {
    return context.l10n.chatPinMessage(
      message.senderName,
      context.l10n.aMessage,
    );
  }
}

String _notificationContentPreview(
  BuildContext context,
  String category,
  String content,
) {
  if (category.contains('TEXT')) return content.trim();
  if (category.contains('SNAPSHOT')) return '[${context.l10n.transfer}]';
  if (category.contains('STICKER')) return '[${context.l10n.sticker}]';
  if (category.contains('IMAGE')) return '[${context.l10n.image}]';
  if (category.contains('VIDEO')) return '[${context.l10n.video}]';
  if (category.contains('LIVE')) return '[${context.l10n.live}]';
  if (category.contains('DATA')) return '[${context.l10n.file}]';
  if (category.contains('POST')) {
    return content.trim().isEmpty ? context.l10n.post : content.trim();
  }
  if (category.contains('LOCATION')) return '[${context.l10n.location}]';
  if (category.contains('AUDIO')) return '[${context.l10n.audio}]';
  if (category.contains('CONTACT')) return '[${context.l10n.contact}]';
  if (category.contains('TRANSCRIPT')) return '[${context.l10n.transcript}]';
  if (category.contains('INSCRIPTION')) return '[${context.l10n.collectible}]';
  if (category == 'APP_BUTTON_GROUP') return _appButtonGroupPreview(content);
  if (category == 'APP_CARD') return _appCardPreview(context, content);
  return context.l10n.messageNotSupport;
}

String _appButtonGroupPreview(String content) {
  try {
    return (jsonDecode(content) as List<dynamic>)
        .map((item) => '[${(item as Map<String, dynamic>)['label'] ?? ''}]')
        .join();
  } on Object {
    return '';
  }
}

String _appCardPreview(BuildContext context, String content) {
  try {
    final value = jsonDecode(content) as Map<String, dynamic>;
    return '[${value['title']?.toString() ?? context.l10n.card}]';
  } on Object {
    return '[${context.l10n.card}]';
  }
}
