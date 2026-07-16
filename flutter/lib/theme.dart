import 'package:flutter/material.dart';

const mixinAccent = Color.fromRGBO(61, 117, 227, 1);
const mixinSecondaryText = Color.fromRGBO(184, 189, 199, 1);

@immutable
class MixinThemeColors extends ThemeExtension<MixinThemeColors> {
  const MixinThemeColors({
    required this.primary,
    required this.accent,
    required this.text,
    required this.icon,
    required this.secondaryText,
    required this.sidebarSelected,
    required this.listSelected,
    required this.chatBackground,
    required this.background,
    required this.divider,
    required this.red,
  });

  final Color primary;
  final Color accent;
  final Color text;
  final Color icon;
  final Color secondaryText;
  final Color sidebarSelected;
  final Color listSelected;
  final Color chatBackground;
  final Color background;
  final Color divider;
  final Color red;

  @override
  MixinThemeColors copyWith() => this;

  @override
  MixinThemeColors lerp(covariant MixinThemeColors? other, double t) =>
      other ?? this;
}

const lightMixinColors = MixinThemeColors(
  primary: Colors.white,
  accent: Color.fromRGBO(61, 117, 227, 1),
  text: Color.fromRGBO(51, 51, 51, 1),
  icon: Color.fromRGBO(47, 48, 50, 1),
  secondaryText: Color.fromRGBO(184, 189, 199, 1),
  sidebarSelected: Color.fromRGBO(0, 0, 0, 0.08),
  listSelected: Color.fromRGBO(246, 247, 250, 1),
  chatBackground: Color.fromRGBO(237, 238, 238, 1),
  background: Color.fromRGBO(246, 247, 250, 1),
  divider: Color.fromRGBO(229, 231, 235, 1),
  red: Color.fromRGBO(229, 120, 116, 1),
);

const darkMixinColors = MixinThemeColors(
  primary: Color.fromRGBO(44, 49, 54, 1),
  accent: Color.fromRGBO(65, 145, 255, 1),
  text: Color.fromRGBO(255, 255, 255, 0.9),
  icon: Color.fromRGBO(255, 255, 255, 0.9),
  secondaryText: Color.fromRGBO(255, 255, 255, 0.4),
  sidebarSelected: Color.fromRGBO(255, 255, 255, 0.06),
  listSelected: Color.fromRGBO(255, 255, 255, 0.06),
  chatBackground: Color.fromRGBO(35, 39, 43, 1),
  background: Color.fromRGBO(40, 44, 48, 1),
  divider: Color.fromRGBO(0, 0, 0, 0.16),
  red: Color.fromRGBO(246, 112, 112, 1),
);

ThemeData buildMixinTheme(Brightness brightness) {
  final colors = brightness == Brightness.dark
      ? darkMixinColors
      : lightMixinColors;
  return ThemeData(
    brightness: brightness,
    colorScheme: ColorScheme.fromSeed(
      seedColor: colors.accent,
      brightness: brightness,
      primary: colors.accent,
      surface: colors.primary,
    ),
    scaffoldBackgroundColor: colors.background,
    fontFamilyFallback: const [
      '-apple-system',
      'BlinkMacSystemFont',
      'Segoe UI',
    ],
    splashFactory: NoSplash.splashFactory,
    dividerColor: colors.divider,
    extensions: [colors],
  );
}

extension MixinThemeContext on BuildContext {
  MixinThemeColors get mixinTheme =>
      Theme.of(this).extension<MixinThemeColors>() ?? lightMixinColors;
}
