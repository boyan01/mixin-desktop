import 'package:flutter/foundation.dart';

final kEmojiFontFamily = switch (defaultTargetPlatform) {
  TargetPlatform.iOS || TargetPlatform.macOS => 'Apple Color Emoji',
  TargetPlatform.windows => 'Segoe UI Emoji',
  _ => 'NotoColorEmoji',
};
