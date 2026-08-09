import 'package:flutter/material.dart';

import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../avatar_view.dart';
import '../badges_widget.dart';
import '../interactive_decorated_box.dart';
import '../message_style.dart';
import '../show_message_user_dialog.dart';

class ContactMessageItem extends StatelessWidget {
  const ContactMessageItem({
    required this.message,
    super.key,
  });

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final userId = message.sharedUserId ?? '';
    return InteractiveDecoratedBox(
      onTap: userId.isEmpty
          ? null
          : () => openMessageUserDialog(context, userId: userId),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          AvatarView(
            userId: userId,
            name: message.sharedUserFullName ?? '',
            avatarUrl: message.sharedUserAvatarUrl ?? '',
            size: 40,
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Flexible(
                      child: Text(
                        Characters(message.sharedUserFullName ?? '')
                            .replaceAll(Characters(''), Characters('\u200B'))
                            .toString(),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: context.messageStyle.primaryFontSize,
                        ),
                      ),
                    ),
                    BadgesWidget(
                      verified: message.sharedUserIsVerified,
                      isBot: message.sharedUserAppId != null,
                      membership: message.sharedUserMembership,
                    ),
                  ],
                ),
                Text(
                  message.sharedUserIdentityNumber ?? '',
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: context.messageStyle.secondaryFontSize,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
