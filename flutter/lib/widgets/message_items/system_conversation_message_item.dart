import 'package:flutter/material.dart';

import '../../l10n/generated/app_localizations.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../high_light_text.dart';
import '../message_style.dart';

class SystemConversationMessageItem extends StatelessWidget {
  const SystemConversationMessageItem({
    required this.message,
    required this.currentUserId,
    super.key,
  });

  final MessageListEntry message;
  final String currentUserId;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: context.dynamicColor(const Color.fromRGBO(202, 234, 201, 1)),
            borderRadius: const BorderRadius.all(Radius.circular(10)),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 5, horizontal: 10),
            child: CustomText(
              generateSystemMessageText(
                context,
                action: message.action,
                participantId: message.participantId,
                participantName: message.participantFullName,
                senderId: message.senderId,
                senderName: message.senderName,
                currentUserId: currentUserId,
                expireIn: message.expireIn,
              ),
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

String generateSystemMessageText(
  BuildContext context, {
  required String? action,
  required String? participantId,
  required String? participantName,
  required String? senderId,
  required String? senderName,
  required String currentUserId,
  required int? expireIn,
}) {
  final participant = participantId == currentUserId
      ? context.l10n.you
      : participantName ?? '';
  final sender = senderId == currentUserId
      ? context.l10n.you
      : senderName ?? '';
  return switch (action?.toUpperCase()) {
    'JOIN' => context.l10n.chatGroupJoin(participant),
    'EXIT' => context.l10n.chatGroupExit(participant),
    'ADD' => context.l10n.chatGroupAdd(sender, participant),
    'REMOVE' => context.l10n.chatGroupRemove(sender, participant),
    'CREATE' => context.l10n.createdThisGroup(sender),
    'ROLE' => context.l10n.nowAnAddmin(participant),
    'EXPIRE' => _expireSystemText(context.l10n, sender, expireIn),
    'UPDATE' => context.l10n.messageNotSupport,
    _ => context.l10n.messageNotSupport,
  };
}

String _expireSystemText(AppLocalizations l10n, String sender, int? expireIn) {
  if (expireIn == null) {
    return l10n.changedDisappearingMessageSettings(sender);
  }
  if (expireIn <= 0) return l10n.disableDisappearingMessage(sender);
  return l10n.setDisappearingMessageTimeTo(
    sender,
    _formatConversationExpireIn(l10n, expireIn),
  );
}

String _formatConversationExpireIn(AppLocalizations l10n, int seconds) {
  final duration = Duration(seconds: seconds);
  if (seconds < 60) return '$seconds ${l10n.unitSecond(seconds)}';
  final minutes = duration.inMinutes;
  if (minutes < 60) return '$minutes ${l10n.unitMinute(minutes)}';
  final hours = duration.inHours;
  if (hours < 24) return '$hours ${l10n.unitHour(hours)}';
  final days = duration.inDays;
  if (days < 7) return '$days ${l10n.unitDay(days)}';
  final weeks = days ~/ 7;
  return '$weeks ${l10n.unitWeek(weeks)}';
}
