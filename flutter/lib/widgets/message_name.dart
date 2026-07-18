import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';

class MessageName extends StatelessWidget {
  const MessageName({
    required this.userName,
    required this.userId,
    required this.userIdentityNumber,
    required this.verified,
    required this.isBot,
    required this.membership,
    required this.showIdentityNumber,
    required this.onTap,
    super.key,
  });

  final String userName;
  final String userId;
  final String userIdentityNumber;
  final bool verified;
  final bool isBot;
  final String? membership;
  final bool showIdentityNumber;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.centerLeft,
    child: InteractiveDecoratedBox(
      onTap: onTap,
      cursor: SystemMouseCursors.click,
      child: Padding(
        padding: const EdgeInsets.only(left: 10, bottom: 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          mainAxisSize: MainAxisSize.min,
          children: [
            CustomText(
              userName,
              style: TextStyle(
                fontSize: context.messageStyle.secondaryFontSize,
                color: messageNameColor(userId),
              ),
            ),
            if (showIdentityNumber && userIdentityNumber != '0') ...[
              const SizedBox(width: 2),
              Padding(
                padding: const EdgeInsets.only(bottom: 2),
                child: Text(
                  '@$userIdentityNumber',
                  style: TextStyle(
                    fontSize: context.messageStyle.statusFontSize,
                    color: context.theme.text.withValues(alpha: 0.5),
                  ),
                ),
              ),
            ],
            Padding(
              padding: const EdgeInsets.only(bottom: 3),
              child: BadgesWidget(
                verified: verified,
                isBot: isBot,
                membership: membership,
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

Color messageNameColor(String userId) {
  const colors = [
    Color(0xFF8C8DFF),
    Color(0xFF7983C2),
    Color(0xFF6D8DDE),
    Color(0xFF5979F0),
    Color(0xFF6695DF),
    Color(0xFF8F7AC5),
    Color(0xFF9D77A5),
    Color(0xFF8A64D0),
    Color(0xFFAA66C3),
    Color(0xFFA75C96),
    Color(0xFFC8697D),
    Color(0xFFB74D62),
    Color(0xFFBD637C),
    Color(0xFFB3798E),
    Color(0xFF9B6D77),
    Color(0xFFB87F7F),
    Color(0xFFC5595A),
    Color(0xFFAA4848),
    Color(0xFFB0665E),
    Color(0xFFB76753),
    Color(0xFFBB5334),
    Color(0xFFC97B46),
    Color(0xFFBE6C2C),
    Color(0xFFCB7F40),
    Color(0xFFA47758),
    Color(0xFFB69370),
    Color(0xFFA49373),
    Color(0xFFAA8A46),
    Color(0xFFAA8220),
    Color(0xFF76A048),
    Color(0xFF9CAD23),
    Color(0xFFA19431),
    Color(0xFFAA9100),
    Color(0xFFA09555),
    Color(0xFFC49B4B),
    Color(0xFF5FB05F),
    Color(0xFF6AB48F),
    Color(0xFF71B15C),
    Color(0xFFB3B357),
    Color(0xFFA3B561),
    Color(0xFF909F45),
    Color(0xFF93B289),
    Color(0xFF3D98D0),
    Color(0xFF429AB6),
    Color(0xFF4EABAA),
    Color(0xFF6BC0CE),
    Color(0xFF64B5D9),
    Color(0xFF3E9CCB),
    Color(0xFF2887C4),
    Color(0xFF52A98B),
  ];
  return colors[_uuidHashCode(userId.trim()).abs() % colors.length];
}

int _uuidHashCode(String id) {
  final components = id.split('-');
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
