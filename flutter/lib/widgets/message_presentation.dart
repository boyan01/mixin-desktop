import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/widgets/message_rows.dart';

class MessagePresentation {
  const MessagePresentation({
    required this.isCurrentUser,
    required this.isGroupOrBotGroupConversation,
    required this.showAvatar,
    required this.showNip,
    required this.showSender,
  });

  factory MessagePresentation.fromRow({
    required MessageRowModel row,
    required String currentUserId,
    required bool isGroupOrBotGroupConversation,
    required bool enableShowAvatar,
  }) {
    final isCurrentUser = row.message.senderId == currentUserId;
    final showAvatar = isGroupOrBotGroupConversation && enableShowAvatar;
    final showNip =
        !(row.sameUserNext && row.sameDayNext) &&
        (!showAvatar || isCurrentUser);
    final showSender =
        isGroupOrBotGroupConversation &&
        !isCurrentUser &&
        (!row.sameUserPrevious || !row.sameDayPrevious);

    return MessagePresentation(
      isCurrentUser: isCurrentUser,
      isGroupOrBotGroupConversation: isGroupOrBotGroupConversation,
      showAvatar: showAvatar,
      showNip: showNip,
      showSender: showSender,
    );
  }

  final bool isCurrentUser;
  final bool isGroupOrBotGroupConversation;
  final bool showAvatar;
  final bool showNip;
  final bool showSender;
}

bool resolveGroupOrBotGroupConversation({
  required MessageListEntry message,
  required bool isGroup,
  required bool isBot,
  required String conversationOwnerId,
}) =>
    isGroup ||
    message.senderId != (message.conversationOwnerId ?? conversationOwnerId) ||
    isBot;
