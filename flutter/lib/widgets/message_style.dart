import 'package:flutter/widgets.dart';
import 'package:provider/provider.dart';

import '../controllers/settings_controller.dart';

extension MessageStyleExtension on BuildContext {
  MessageStyle get messageStyle =>
      MessageStyle.defaultStyle + watch<SettingsController>().chatFontSizeDelta;
}

class MessageStyle {
  const MessageStyle({
    required this.primaryFontSize,
    required this.secondaryFontSize,
    required this.tertiaryFontSize,
    required this.statusFontSize,
  });

  static const defaultStyle = MessageStyle(
    primaryFontSize: 16,
    secondaryFontSize: 14,
    tertiaryFontSize: 12,
    statusFontSize: 10,
  );

  final double primaryFontSize;
  final double secondaryFontSize;
  final double tertiaryFontSize;
  final double statusFontSize;

  MessageStyle operator +(double delta) => MessageStyle(
    primaryFontSize: primaryFontSize + delta,
    secondaryFontSize: secondaryFontSize + delta,
    tertiaryFontSize: tertiaryFontSize + delta,
    statusFontSize: statusFontSize,
  );
}
