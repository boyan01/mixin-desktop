import 'package:mixin_desktop_ui/models/message_list_entry.dart';

class MessageRowModel {
  MessageRowModel({required this.message, this.previous, this.next})
    : sameDayPrevious = _isSameDay(previous?.createdAt, message.createdAt),
      sameDayNext = _isSameDay(next?.createdAt, message.createdAt),
      previousIsSystem = previous?.category.startsWith('SYSTEM_') ?? false,
      previousIsPin = previous?.category == 'MESSAGE_PIN',
      sameUserNext = next?.senderId == message.senderId {
    sameUserPrevious =
        !previousIsSystem &&
        !previousIsPin &&
        previous?.senderId == message.senderId;
    dateTime = sameDayPrevious ? null : message.createdAt;
  }

  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final bool sameDayPrevious;
  final bool sameDayNext;
  final bool previousIsSystem;
  final bool previousIsPin;
  late final bool sameUserPrevious;
  final bool sameUserNext;
  late final DateTime? dateTime;
}

bool _isSameDay(DateTime? first, DateTime second) {
  if (first == null) return false;
  final localFirst = first.toLocal();
  final localSecond = second.toLocal();
  return localFirst.year == localSecond.year &&
      localFirst.month == localSecond.month &&
      localFirst.day == localSecond.day;
}
