import 'dart:convert';

import '../src/rust/desktop_api.dart';

abstract interface class SettingsStore {
  Future<Object?> get(String key);

  Future<void> set(String key, Object? value);

  Stream<Object?> subscribe(String key);

  static Future<SettingsStore> open() async {
    final desktop = await openDesktop();
    return _RustSettingsStore(desktop.settings);
  }
}

class _RustSettingsStore implements SettingsStore {
  const _RustSettingsStore(this._settings);

  final SettingsHandle _settings;

  @override
  Future<Object?> get(String key) async {
    final value = await _settings.setting(key: key);
    return value == null ? null : jsonDecode(value);
  }

  @override
  Future<void> set(String key, Object? value) => _settings.setSetting(
    key: key,
    value: value == null ? null : jsonEncode(value),
  );

  @override
  Stream<Object?> subscribe(String key) => _settings
      .subscribeSetting(key: key)
      .map((value) => value == null ? null : jsonDecode(value));
}
