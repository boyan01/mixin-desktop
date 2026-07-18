import 'package:flutter/material.dart';

const _nameColors = [
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

Color? nameColorForId(String? value) {
  final components = value?.trim().split('-');
  if (components == null || components.length != 5) return null;
  try {
    final mostSignificant =
        (int.parse(components[0], radix: 16) << 32) |
        (int.parse(components[1], radix: 16) << 16) |
        int.parse(components[2], radix: 16);
    final leastSignificant =
        (int.parse(components[3], radix: 16) << 48) |
        int.parse(components[4], radix: 16);
    final highLow = mostSignificant ^ leastSignificant;
    final hash = ((highLow >> 32) ^ highLow).toSigned(32);
    return _nameColors[hash.abs() % _nameColors.length];
  } on FormatException {
    return null;
  }
}
