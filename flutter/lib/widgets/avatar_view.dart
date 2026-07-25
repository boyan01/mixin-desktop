import 'package:flutter/material.dart';

import '../models/conversation_list_entry.dart';
import 'mixin_image.dart';

class ConversationAvatarView extends StatelessWidget {
  const ConversationAvatarView({
    required this.conversation,
    required this.size,
    super.key,
  });

  final ConversationListEntry conversation;
  final double size;

  @override
  Widget build(BuildContext context) {
    if (!conversation.isGroup) {
      return AvatarView(
        userId: conversation.ownerId,
        name: conversation.name,
        avatarUrl: conversation.avatarUrl,
        size: size,
      );
    }
    return SizedBox.square(
      dimension: size,
      child: ClipOval(
        child: AvatarPuzzlesView(
          avatars: conversation.groupAvatars.take(4).toList(growable: false),
          size: size,
        ),
      ),
    );
  }
}

class AvatarPuzzlesView extends StatelessWidget {
  const AvatarPuzzlesView({
    required this.avatars,
    required this.size,
    super.key,
  });

  final List<ConversationAvatarEntry> avatars;
  final double size;

  @override
  Widget build(BuildContext context) {
    if (avatars.isEmpty) return SizedBox.square(dimension: size);
    return switch (avatars.length) {
      1 => AvatarView(
        userId: avatars.single.userId,
        name: avatars.single.name,
        avatarUrl: avatars.single.avatarUrl,
        size: size,
        clipOval: false,
      ),
      2 => Row(children: avatars.map(_buildAvatar).toList()),
      3 => Row(
        children: [
          Expanded(
            child: AvatarView(
              userId: avatars.first.userId,
              name: avatars.first.name,
              avatarUrl: avatars.first.avatarUrl,
              size: size,
              clipOval: false,
            ),
          ),
          Expanded(
            child: Column(
              children: avatars.sublist(1).map(_buildAvatar).toList(),
            ),
          ),
        ],
      ),
      _ => Row(
        children: [avatars.sublist(0, 2), avatars.sublist(2)]
            .map(
              (column) => Expanded(
                child: Column(children: column.map(_buildAvatar).toList()),
              ),
            )
            .toList(),
      ),
    };
  }

  Widget _buildAvatar(ConversationAvatarEntry avatar) => Expanded(
    child: AvatarView(
      userId: avatar.userId,
      name: avatar.name,
      avatarUrl: avatar.avatarUrl,
      size: size,
      clipOval: false,
    ),
  );
}

class AvatarView extends StatelessWidget {
  const AvatarView({
    required this.userId,
    required this.name,
    required this.avatarUrl,
    required this.size,
    super.key,
    this.clipOval = true,
  });

  final String userId;
  final String name;
  final String avatarUrl;
  final double size;
  final bool clipOval;

  @override
  Widget build(BuildContext context) {
    final placeholder = SizedBox.fromSize(
      size: Size.square(size),
      child: DecoratedBox(
        decoration: BoxDecoration(color: _avatarColor(userId)),
        child: Center(
          child: Text(
            name.isEmpty ? '' : name.characters.first.toUpperCase(),
            style: const TextStyle(
              color: Colors.white,
              fontSize: 18,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );
    final image = avatarUrl.isEmpty
        ? placeholder
        : MixinImage.network(
            avatarUrl,
            width: size,
            height: size,
            placeholder: () => placeholder,
            errorBuilder: (_, _, _) => placeholder,
          );
    return clipOval ? ClipOval(child: image) : image;
  }
}

Color _avatarColor(String id) {
  const colors = [
    Color(0xFFFFD659),
    Color(0xFFFFC168),
    Color(0xFFF58268),
    Color(0xFFF4979C),
    Color(0xFFEC7F87),
    Color(0xFFFF78CB),
    Color(0xFFC377E0),
    Color(0xFF8BAAFF),
    Color(0xFF78DCFA),
    Color(0xFF88E5B9),
    Color(0xFFBFF199),
    Color(0xFFC5E1A5),
    Color(0xFFCD907D),
    Color(0xFFBE938E),
    Color(0xFFB68F91),
    Color(0xFFBC987B),
    Color(0xFFA69E8E),
    Color(0xFFD4C99E),
    Color(0xFF93C2E6),
    Color(0xFF92C3D9),
    Color(0xFF8FBFC5),
    Color(0xFF80CBC4),
    Color(0xFFA4DBDB),
    Color(0xFFB2C8BD),
    Color(0xFFF7C8C9),
    Color(0xFFDCC6E4),
    Color(0xFFBABAE8),
    Color(0xFFBABCD5),
    Color(0xFFAD98DA),
    Color(0xFFC097D9),
  ];
  return colors[_uuidHashCode(id).abs() % colors.length];
}

int _uuidHashCode(String id) {
  final components = id.trim().split('-');
  if (components.length != 5) return id.hashCode;
  try {
    final mostSignificantBits =
        (int.parse(components[0], radix: 16) << 32) |
        (int.parse(components[1], radix: 16) << 16) |
        int.parse(components[2], radix: 16);
    final leastSignificantBits =
        (int.parse(components[3], radix: 16) << 48) |
        int.parse(components[4], radix: 16);
    final hilo = mostSignificantBits ^ leastSignificantBits;
    return (hilo >> 32) ^ hilo.toSigned(32);
  } on FormatException {
    return id.hashCode;
  }
}
