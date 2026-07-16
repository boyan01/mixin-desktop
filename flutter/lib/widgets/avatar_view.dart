import 'package:flutter/material.dart';

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
    final placeholder = ColoredBox(
      color: _avatarColor(userId),
      child: Center(
        child: Text(
          name.isEmpty ? '' : name.characters.first.toUpperCase(),
          style: TextStyle(
            color: Colors.white,
            fontSize: size * 0.36,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
    final image = avatarUrl.isEmpty
        ? placeholder
        : Image.network(
            avatarUrl,
            width: size,
            height: size,
            fit: BoxFit.cover,
            errorBuilder: (_, _, _) => placeholder,
          );
    final child = SizedBox.square(dimension: size, child: image);
    return clipOval ? ClipOval(child: child) : child;
  }
}

Color _avatarColor(String id) {
  const colors = [
    Color(0xFFFFD659),
    Color(0xFFF58268),
    Color(0xFFEC7F87),
    Color(0xFFC377E0),
    Color(0xFF8BAAFF),
    Color(0xFF78DCFA),
    Color(0xFF88E5B9),
    Color(0xFF80CBC4),
  ];
  return colors[id.hashCode.abs() % colors.length];
}
