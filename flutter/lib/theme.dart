import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/utils/system_fonts.dart';

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
    required this.popUp,
    required this.red,
    required this.green,
    required this.warning,
    required this.highlight,
    required this.dateTime,
    required this.encrypt,
    required this.statusBackground,
    required this.stickerPlaceholderColor,
    required this.waveformBackground,
    required this.waveformForeground,
    required this.settingCellBackgroundColor,
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
  final Color popUp;
  final Color red;
  final Color green;
  final Color warning;
  final Color highlight;
  final Color dateTime;
  final Color encrypt;
  final Color statusBackground;
  final Color stickerPlaceholderColor;
  final Color waveformBackground;
  final Color waveformForeground;
  final Color settingCellBackgroundColor;

  @override
  MixinThemeColors copyWith({
    Color? primary,
    Color? accent,
    Color? text,
    Color? icon,
    Color? secondaryText,
    Color? sidebarSelected,
    Color? listSelected,
    Color? chatBackground,
    Color? background,
    Color? divider,
    Color? popUp,
    Color? red,
    Color? green,
    Color? warning,
    Color? highlight,
    Color? dateTime,
    Color? encrypt,
    Color? statusBackground,
    Color? stickerPlaceholderColor,
    Color? waveformBackground,
    Color? waveformForeground,
    Color? settingCellBackgroundColor,
  }) => MixinThemeColors(
    primary: primary ?? this.primary,
    accent: accent ?? this.accent,
    text: text ?? this.text,
    icon: icon ?? this.icon,
    secondaryText: secondaryText ?? this.secondaryText,
    sidebarSelected: sidebarSelected ?? this.sidebarSelected,
    listSelected: listSelected ?? this.listSelected,
    chatBackground: chatBackground ?? this.chatBackground,
    background: background ?? this.background,
    divider: divider ?? this.divider,
    popUp: popUp ?? this.popUp,
    red: red ?? this.red,
    green: green ?? this.green,
    warning: warning ?? this.warning,
    highlight: highlight ?? this.highlight,
    dateTime: dateTime ?? this.dateTime,
    encrypt: encrypt ?? this.encrypt,
    statusBackground: statusBackground ?? this.statusBackground,
    stickerPlaceholderColor:
        stickerPlaceholderColor ?? this.stickerPlaceholderColor,
    waveformBackground: waveformBackground ?? this.waveformBackground,
    waveformForeground: waveformForeground ?? this.waveformForeground,
    settingCellBackgroundColor:
        settingCellBackgroundColor ?? this.settingCellBackgroundColor,
  );

  @override
  MixinThemeColors lerp(covariant MixinThemeColors? other, double t) =>
      other == null
      ? this
      : MixinThemeColors(
          primary: Color.lerp(primary, other.primary, t)!,
          accent: Color.lerp(accent, other.accent, t)!,
          text: Color.lerp(text, other.text, t)!,
          icon: Color.lerp(icon, other.icon, t)!,
          secondaryText: Color.lerp(secondaryText, other.secondaryText, t)!,
          sidebarSelected: Color.lerp(
            sidebarSelected,
            other.sidebarSelected,
            t,
          )!,
          listSelected: Color.lerp(listSelected, other.listSelected, t)!,
          chatBackground: Color.lerp(chatBackground, other.chatBackground, t)!,
          background: Color.lerp(background, other.background, t)!,
          divider: Color.lerp(divider, other.divider, t)!,
          popUp: Color.lerp(popUp, other.popUp, t)!,
          red: Color.lerp(red, other.red, t)!,
          green: Color.lerp(green, other.green, t)!,
          warning: Color.lerp(warning, other.warning, t)!,
          highlight: Color.lerp(highlight, other.highlight, t)!,
          dateTime: Color.lerp(dateTime, other.dateTime, t)!,
          encrypt: Color.lerp(encrypt, other.encrypt, t)!,
          statusBackground: Color.lerp(
            statusBackground,
            other.statusBackground,
            t,
          )!,
          stickerPlaceholderColor: Color.lerp(
            stickerPlaceholderColor,
            other.stickerPlaceholderColor,
            t,
          )!,
          waveformBackground: Color.lerp(
            waveformBackground,
            other.waveformBackground,
            t,
          )!,
          waveformForeground: Color.lerp(
            waveformForeground,
            other.waveformForeground,
            t,
          )!,
          settingCellBackgroundColor: Color.lerp(
            settingCellBackgroundColor,
            other.settingCellBackgroundColor,
            t,
          )!,
        );
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
  popUp: Colors.white,
  red: Color.fromRGBO(229, 120, 116, 1),
  green: Color.fromRGBO(80, 189, 92, 1),
  warning: Color.fromRGBO(244, 171, 45, 1),
  highlight: Color.fromRGBO(167, 242, 89, 1),
  dateTime: Color.fromRGBO(213, 211, 243, 1),
  encrypt: Color.fromRGBO(255, 247, 173, 1),
  statusBackground: Color.fromRGBO(245, 247, 250, 1),
  stickerPlaceholderColor: Color.fromRGBO(236, 236, 236, 1),
  waveformBackground: Color.fromRGBO(221, 221, 221, 1),
  waveformForeground: Color.fromRGBO(155, 155, 155, 1),
  settingCellBackgroundColor: Colors.white,
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
  popUp: Color.fromRGBO(62, 65, 72, 1),
  red: Color.fromRGBO(246, 112, 112, 1),
  green: Color.fromRGBO(96, 209, 108, 1),
  warning: Color.fromRGBO(243, 177, 64, 1),
  highlight: Color.fromRGBO(134, 184, 82, 1),
  dateTime: Color.fromRGBO(213, 211, 243, 1),
  encrypt: Color.fromRGBO(255, 247, 173, 1),
  statusBackground: Color.fromRGBO(245, 247, 250, 1),
  stickerPlaceholderColor: Color.fromRGBO(40, 44, 48, 1),
  waveformBackground: Color.fromRGBO(255, 255, 255, 0.4),
  waveformForeground: Colors.white,
  settingCellBackgroundColor: Color.fromRGBO(255, 255, 255, 0.06),
);

ThemeData buildMixinTheme(Brightness brightness) {
  final colors = brightness == Brightness.dark
      ? darkMixinColors
      : lightMixinColors;
  return ThemeData(
    colorScheme: brightness == Brightness.dark
        ? ColorScheme.dark(primary: colors.text)
        : ColorScheme.light(primary: colors.text),
    textSelectionTheme: TextSelectionThemeData(cursorColor: colors.accent),
    useMaterial3: true,
    extensions: [colors],
  ).withFallbackFonts();
}

extension MixinThemeContext on BuildContext {
  MixinThemeColors get mixinTheme =>
      Theme.of(this).extension<MixinThemeColors>() ?? lightMixinColors;

  // Compatibility property for widgets copied from flutter-app.
  MixinThemeColors get theme => mixinTheme;

  Color dynamicColor(Color color, {Color? darkColor}) {
    if (darkColor == null) return color;
    final brightness = Theme.of(this).brightness;
    return brightness == Brightness.dark ? darkColor : color;
  }
}
