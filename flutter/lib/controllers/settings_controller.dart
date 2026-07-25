import 'package:flutter/material.dart';

import 'settings_store.dart';

class SettingsController extends ChangeNotifier {
  SettingsController({SettingsStore? store}) : _providedStore = store;

  final SettingsStore? _providedStore;
  late final SettingsStore store;

  ThemeMode themeMode = ThemeMode.system;
  bool messageShowAvatar = true;
  bool messageShowIdentityNumber = false;
  bool messagePreview = true;
  double chatFontSizeDelta = 0;

  Future<void> initialize() async {
    store = _providedStore ?? await SettingsStore.open();
    themeMode = switch (await store.get('brightness')) {
      1 => ThemeMode.dark,
      2 => ThemeMode.light,
      _ => ThemeMode.system,
    };
    messageShowAvatar =
        await store.get('messageShowAvatar') as bool? ?? messageShowAvatar;
    messageShowIdentityNumber =
        await store.get('messageShowIdentityNumber') as bool? ??
        messageShowIdentityNumber;
    messagePreview =
        await store.get('messagePreview') as bool? ?? messagePreview;
    chatFontSizeDelta =
        (await store.get('chatFontSizeDelta') as num?)?.toDouble() ??
        chatFontSizeDelta;
  }

  Future<void> setThemeMode(ThemeMode value) async {
    if (themeMode == value) return;
    themeMode = value;
    notifyListeners();
    await store.set(
      'brightness',
      switch (value) {
        ThemeMode.system => 0,
        ThemeMode.dark => 1,
        ThemeMode.light => 2,
      },
    );
  }

  Future<void> setMessageShowAvatar(bool value) async {
    if (messageShowAvatar == value) return;
    messageShowAvatar = value;
    notifyListeners();
    await store.set('messageShowAvatar', value);
  }

  Future<void> setMessageShowIdentityNumber(bool value) async {
    if (messageShowIdentityNumber == value) return;
    messageShowIdentityNumber = value;
    notifyListeners();
    await store.set('messageShowIdentityNumber', value);
  }

  Future<void> setMessagePreview(bool value) async {
    if (messagePreview == value) return;
    messagePreview = value;
    notifyListeners();
    await store.set('messagePreview', value);
  }

  Future<void> setChatFontSizeDelta(double value) async {
    if (chatFontSizeDelta == value) return;
    chatFontSizeDelta = value;
    notifyListeners();
    await store.set('chatFontSizeDelta', value);
  }
}
