import '../models/message_list_entry.dart';

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

class MessageRows {
  const MessageRows({required this.top, required this.bottom, this.center});

  factory MessageRows.from({
    required List<MessageListEntry> top,
    required List<MessageListEntry> bottom,
    MessageListEntry? center,
  }) {
    final messages = [...top, ?center, ...bottom];
    final rows = <MessageRowModel>[
      for (var index = 0; index < messages.length; index++)
        MessageRowModel(
          message: messages[index],
          previous: index == 0 ? null : messages[index - 1],
          next: index + 1 == messages.length ? null : messages[index + 1],
        ),
    ];
    return MessageRows(
      top: rows.take(top.length).toList(growable: false),
      center: center == null ? null : rows[top.length],
      bottom: rows
          .skip(top.length + (center == null ? 0 : 1))
          .toList(growable: false),
    );
  }

  final List<MessageRowModel> top;
  final MessageRowModel? center;
  final List<MessageRowModel> bottom;
}

bool _isSameDay(DateTime? first, DateTime second) {
  if (first == null) return false;
  final localFirst = first.toLocal();
  final localSecond = second.toLocal();
  return localFirst.year == localSecond.year &&
      localFirst.month == localSecond.month &&
      localFirst.day == localSecond.day;
}
