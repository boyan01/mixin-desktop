import 'dart:async';

import 'package:mixin_desktop_ui/controllers/settings_store.dart';

class TestSettingsStore implements SettingsStore {
  TestSettingsStore([Map<String, Object?> values = const {}])
    : _values = Map.of(values);

  final Map<String, Object?> _values;
  final Map<String, StreamController<Object?>> _subscriptions = {};

  @override
  Future<Object?> get(String key) async => _values[key];

  @override
  Future<void> set(String key, Object? value) async {
    if (value == null) {
      _values.remove(key);
    } else {
      _values[key] = value;
    }
    _subscriptions[key]?.add(value);
  }

  @override
  Stream<Object?> subscribe(String key) async* {
    yield _values[key];
    yield* (_subscriptions[key] ??= StreamController.broadcast()).stream;
  }
}
