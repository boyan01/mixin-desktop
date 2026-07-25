import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';

import 'test_settings_store.dart';

void main() {
  test('loads injected UI preferences without opening the desktop runtime', () async {
    final controller = SettingsController(
      store: TestSettingsStore({
        'brightness': 1,
        'messageShowAvatar': false,
        'messageShowIdentityNumber': true,
        'messagePreview': false,
        'chatFontSizeDelta': 2,
      }),
    );

    await controller.initialize();

    expect(controller.themeMode, ThemeMode.dark);
    expect(controller.messageShowAvatar, isFalse);
    expect(controller.messageShowIdentityNumber, isTrue);
    expect(controller.messagePreview, isFalse);
    expect(controller.chatFontSizeDelta, 2);
  });
}
