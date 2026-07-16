import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';

class MessageDayTimeChip extends StatelessWidget {
  const MessageDayTimeChip({required this.dateTime, super.key});

  final DateTime dateTime;

  @override
  Widget build(BuildContext context) {
    final localDateTime = dateTime.toLocal();
    final now = DateTime.now();
    final sameDay =
        now.year == localDateTime.year &&
        now.month == localDateTime.month &&
        now.day == localDateTime.day;
    final text = sameDay
        ? AppLocalizations.of(context).today
        : now.year == localDateTime.year
        ? DateFormat.MMMEd().format(localDateTime)
        : DateFormat.yMMMEd().format(localDateTime);

    return Center(
      child: Padding(
        padding: const EdgeInsets.only(top: 16, bottom: 10),
        child: Container(
          decoration: BoxDecoration(
            borderRadius: const BorderRadius.all(Radius.circular(10)),
            color: context.theme.dateTime,
          ),
          padding: const EdgeInsets.symmetric(vertical: 2, horizontal: 10),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minWidth: 64),
            child: Text(
              text,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: context.messageStyle.secondaryFontSize,
                color: Colors.black,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
