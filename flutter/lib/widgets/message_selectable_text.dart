import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import '../src/rust/desktop_api.dart' show AccountHandle;
import '../theme.dart';
import '../utils/app_logger.dart';
import 'high_light_text.dart';
import 'show_message_user_dialog.dart';

Set<String> messageMentionIdentityNumbers(Iterable<String?> texts) => {
  for (final text in texts)
    if (text != null)
      for (final match in RegExp(r'@(\d+)').allMatches(text)) match.group(1)!,
};

String replaceMessageMentions(String text, Map<String, String> mentionNames) =>
    text.replaceAllMapped(RegExp(r'@(\d+)'), (match) {
      final identityNumber = match.group(1)!;
      return '@${mentionNames[identityNumber] ?? identityNumber}';
    });

String useResolvedMessageMentions(
  AccountHandle? account,
  String content, {
  Object? revision,
}) {
  final resolved = useFuture(
    useMemoized(() async {
      if (account == null || !RegExp(r'@\d{4,}').hasMatch(content)) {
        return content;
      }
      try {
        final contents = await account.user().replaceMentions(
          contents: [content],
        );
        return contents.singleOrNull ?? content;
      } catch (exception, stackTrace) {
        e('Resolve message mentions failed', exception, stackTrace);
        return content;
      }
    }, [account, content, revision]),
  );
  return resolved.data ?? content;
}

Map<String, String> useMessageMentionNames(
  AccountHandle? account,
  Iterable<String?> contents, {
  Object? revision,
}) {
  final values = contents.whereType<String>().toList(growable: false);
  final cacheKey = values.join('\u0000');
  final resolved = useFuture(
    useMemoized(() async {
      if (account == null || !RegExp(r'@\d{4,}').hasMatch(cacheKey)) {
        return const <String, String>{};
      }
      try {
        return await account.user().mentionNames(contents: values);
      } catch (exception, stackTrace) {
        e('Resolve message mention names failed', exception, stackTrace);
        return const <String, String>{};
      }
    }, [account, cacheKey, revision]),
  );
  return resolved.data ?? const {};
}

class SelectableMessageText extends StatelessWidget {
  const SelectableMessageText({
    required this.content,
    required this.style,
    super.key,
    this.mentionNames = const {},
    this.keyword = '',
    this.enableSelection = true,
  });

  final String content;
  final TextStyle style;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool enableSelection;

  @override
  Widget build(BuildContext context) {
    final child = CustomText(
      content,
      style: style,
      textMatchers: [
        UrlTextMatcher(context),
        MailTextMatcher(context),
        MentionTextMatcher(context, mentionNames),
        BotNumberTextMatcher(context),
        EmojiTextMatcher(),
        if (keyword.isNotEmpty)
          KeyWordTextMatcher(
            keyword,
            style: TextStyle(
              backgroundColor: context.theme.highlight,
              color: context.theme.text,
            ),
          ),
      ],
    );
    if (!enableSelection) return child;
    return SelectionArea(
      contextMenuBuilder: (context, selectableState) => const SizedBox.shrink(),
      child: child,
    );
  }
}

class MentionTextMatcher extends TextMatcher {
  MentionTextMatcher(BuildContext context, Map<String, String> mentionNames)
    : super.regExp(
        regExp: RegExp(r'@(\d{4,})'),
        matchBuilder: (_, displayString, linkString) {
          final identityNumber = linkString.substring(1);
          final name = mentionNames[identityNumber];
          if (name == null) return TextSpan(text: linkString);
          return TextSpan(
            text: '@$name',
            style: TextStyle(color: context.theme.accent),
            mouseCursor: SystemMouseCursors.click,
            recognizer: TapGestureRecognizer()
              ..onTap = () => openMessageUserDialog(
                context,
                identityNumber: identityNumber,
              ),
          );
        },
      );
}

class BotNumberTextMatcher extends TextMatcher {
  BotNumberTextMatcher(BuildContext context)
    : super.regExp(
        regExp: RegExp(r'(?<!\d)7000\d{6}(?!\d)'),
        matchBuilder: (_, displayString, linkString) => TextSpan(
          text: displayString,
          style: TextStyle(color: context.theme.accent),
          mouseCursor: SystemMouseCursors.click,
          recognizer: TapGestureRecognizer()
            ..onTap = () => openMessageUserDialog(
              context,
              identityNumber: linkString,
            ),
        ),
      );
}
