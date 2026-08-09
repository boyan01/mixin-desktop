import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

import '../../constants/assets.dart';
import '../../l10n/generated/app_localizations.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../interactive_decorated_box.dart';
import '../message_navigation_scope.dart';
import '../message_style.dart';
import '../transcript_page.dart';
import 'message_preview_utils.dart';
import 'unknown_special_message.dart';

class TranscriptMessageItem extends StatelessWidget {
  const TranscriptMessageItem({
    required this.message,
    required this.isCurrentUser,
    required this.overlayDateAndStatus,
    super.key,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final Widget overlayDateAndStatus;

  @override
  Widget build(BuildContext context) {
    final lines = _TranscriptLine.parseList(message.content, context.l10n);
    if (lines == null) return UnknownSpecialMessage(category: message.category);
    final navigation = MessageNavigationScope.maybeOf(context);
    final onSelectConversation = navigation?.onSelectConversation;
    return InteractiveDecoratedBox(
      onTap: onSelectConversation == null
          ? null
          : () => unawaited(
              showTranscriptDialog(
                context,
                account: context.read<rust.AccountHandle>(),
                transcriptId: message.id,
                sentByCurrentUser:
                    message.senderRelationship.toUpperCase() == 'ME',
                currentUserId: navigation!.currentUserId,
                onSelectConversation: onSelectConversation,
                onSelectConversationInfo:
                    navigation.onSelectConversationInfo ?? onSelectConversation,
              ),
            ),
      child: SizedBox(
        width: 260,
        child: Stack(
          children: [
            Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.only(left: 4, bottom: 4),
                  child: Row(
                    children: [
                      const SizedBox(width: 4),
                      Text(
                        context.l10n.transcript,
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: context.messageStyle.primaryFontSize,
                        ),
                      ),
                      const Spacer(),
                      Container(
                        decoration: const BoxDecoration(
                          color: Color.fromRGBO(0, 0, 0, 0.2),
                          borderRadius: BorderRadius.all(Radius.circular(8)),
                        ),
                        alignment: Alignment.center,
                        child: SvgPicture.asset(
                          MixinAssets.postDetail,
                          width: 20,
                          height: 20,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 4,
                  ),
                  decoration: const BoxDecoration(
                    color: Color.fromRGBO(0, 0, 0, 0.04),
                    borderRadius: BorderRadius.all(Radius.circular(6)),
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (final line in lines.take(4))
                        Text(
                          Characters('${line.name}: ${line.preview}')
                              .replaceAll(Characters(''), Characters('\u200B'))
                              .toString(),
                          maxLines: 1,
                          style: TextStyle(
                            color: context.theme.secondaryText,
                            fontSize: context.messageStyle.tertiaryFontSize,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            Positioned(
              right: isCurrentUser ? 1 : 2,
              bottom: 2,
              child: DecoratedBox(
                decoration: const ShapeDecoration(
                  color: Color.fromRGBO(0, 0, 0, 0.3),
                  shape: StadiumBorder(),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    vertical: 3,
                    horizontal: 5,
                  ),
                  child: overlayDateAndStatus,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TranscriptLine {
  const _TranscriptLine(this.name, this.preview);
  final String name;
  final String preview;

  static List<_TranscriptLine>? parseList(String raw, AppLocalizations l10n) {
    try {
      final json = jsonDecode(raw) as List<dynamic>;
      return json.map((item) {
        final data = item as Map<String, dynamic>;
        return _TranscriptLine(
          data['name']?.toString() ?? '',
          categoryPreview(
            l10n,
            data['category']?.toString() ?? data['type']?.toString() ?? '',
            data['content']?.toString() ?? '',
            mediaName: data['media_name']?.toString(),
          ),
        );
      }).toList();
    } on Object {
      return null;
    }
  }
}
