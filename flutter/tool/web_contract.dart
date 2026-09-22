// Non-UI integration contract, run only in a temporary browser profile.
import 'dart:js_interop';

import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/src/rust/error.dart';
import 'package:mixin_desktop_ui/src/rust/frb_generated.dart';

@JS('mixinContractResult')
external set result(JSAny value);

Future<void> main() async {
  try {
    await RustLib.init().timeout(const Duration(seconds: 30));
    final desktops = await Future.wait(List.generate(3, (_) => openDesktop()));
    try {
      await desktops.first.restoreAccount();
      throw StateError('A fresh browser profile must be signed out');
    } on CoreError_NotFound {
      // The normal application can now reach its signed-out state.
    }
    final settings = desktops.first.settings;
    const key = 'flutter.web.runtime.contract';
    final previous = await settings.setting(key: key);
    if (previous != null && previous != 'persistent') {
      throw StateError('Unexpected persisted value');
    }
    final observed = settings
        .subscribeSetting(key: key)
        .firstWhere((value) => value == 'persistent');
    await settings.setSetting(key: key, value: 'persistent');
    await observed.timeout(const Duration(seconds: 10));
    if (await settings.setting(key: key) != 'persistent') {
      throw StateError('Settings write was lost');
    }
    try {
      await settings.mcpSettings();
      throw StateError('Browser MCP must reject the operation');
    } on CoreError_Other catch (error) {
      if (!error.message.contains('not supported')) rethrow;
    }
    try {
      desktops.first.media.audioPlaybackSnapshot();
      throw StateError('Browser native media must reject the operation');
    } on CoreError_Other catch (error) {
      if (!error.message.contains('not supported')) rethrow;
    }
    result = {'ok': true, 'restored': previous == 'persistent'}.jsify()!;
  } catch (error, stack) {
    result = {'ok': false, 'error': '$error', 'stack': '$stack'}.jsify()!;
  }
}
