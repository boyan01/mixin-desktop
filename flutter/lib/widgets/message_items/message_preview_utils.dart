import 'dart:convert';

import 'package:markdown/markdown.dart' as markdown;

import '../../l10n/generated/app_localizations.dart';

String postOptimizeMarkdown(String content) {
  var optimized = const LineSplitter().convert(content).take(10).join('\r\n');
  if (optimized.length > 1024) optimized = optimized.substring(0, 1024);
  return markdown.Document()
      .parseLines(const LineSplitter().convert(optimized))
      .map((node) => node.textContent)
      .join()
      .replaceAll(RegExp(r'\s+'), ' ');
}

String? nonEmpty(Object? value) {
  final text = value?.toString().trim();
  return text == null || text.isEmpty ? null : text;
}

String categoryPreview(
  AppLocalizations l10n,
  String category,
  String content, {
  String? mediaName,
  String? sharedName,
}) {
  if (category.endsWith('_TEXT')) return content.trim();
  if (category.endsWith('_POST')) {
    return content.trim().isEmpty ? l10n.post : content.trim();
  }
  if (category == 'SYSTEM_ACCOUNT_SNAPSHOT' ||
      category == 'SYSTEM_SAFE_SNAPSHOT') {
    return '[${l10n.transfer}]';
  }
  if (category.endsWith('_IMAGE')) return '[${l10n.image}]';
  if (category.endsWith('_VIDEO')) return '[${l10n.video}]';
  if (category.endsWith('_LIVE')) return '[${l10n.live}]';
  if (category.endsWith('_AUDIO')) return '[${l10n.audio}]';
  if (category.endsWith('_DATA')) {
    return mediaName?.isNotEmpty == true ? mediaName! : '[${l10n.file}]';
  }
  if (category.endsWith('_STICKER')) {
    return '[${l10n.sticker}]';
  }
  if (category.endsWith('_CONTACT')) {
    return sharedName?.isNotEmpty == true ? sharedName! : '[${l10n.contact}]';
  }
  if (category.endsWith('_LOCATION')) {
    return '[${l10n.location}]';
  }
  if (category.endsWith('_TRANSCRIPT')) {
    return '[${l10n.transcript}]';
  }
  if (category == 'SYSTEM_SAFE_INSCRIPTION') {
    return '[${l10n.collectible}]';
  }
  return l10n.messageNotSupport;
}
