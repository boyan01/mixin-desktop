import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../l10n/l10n.dart';
import '../../theme.dart';
import '../app_protocol_handler.dart';
import '../message_layout.dart';
import '../message_style.dart';

class WaitingMessageItem extends StatelessWidget {
  const WaitingMessageItem({
    required this.messageId,
    required this.subject,
    required this.dateAndStatus,
    super.key,
  });

  final String messageId;
  final String subject;
  final Widget dateAndStatus;

  @override
  Widget build(BuildContext context) {
    final helpUri = Uri.tryParse(context.l10n.chatNotSupportUrl);
    return MessageLayout(
      spacing: 6,
      content: RichText(
        key: Key('message-media-waiting-$messageId'),
        text: TextSpan(
          text: context.l10n.chatDecryptionFailedHint(subject),
          style: TextStyle(
            fontSize: context.messageStyle.primaryFontSize,
            color: context.theme.text,
          ),
          children: [
            TextSpan(
              mouseCursor: SystemMouseCursors.click,
              text: context.l10n.learnMore,
              style: TextStyle(
                fontSize: context.messageStyle.primaryFontSize,
                color: context.theme.accent,
              ),
              recognizer: TapGestureRecognizer()
                ..onTap = helpUri == null
                    ? null
                    : () => context.openUri(helpUri),
            ),
          ],
        ),
      ),
      dateAndStatus: dateAndStatus,
    );
  }
}
