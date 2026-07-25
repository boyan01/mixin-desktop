import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:local_auth/local_auth.dart';

import 'settings_store.dart';

class SecurityController extends ChangeNotifier {
  SecurityController(this.accountId, this._settings);

  final String accountId;
  final SettingsStore _settings;
  final LocalAuthentication _authentication = LocalAuthentication();

  String? _passcode;
  bool _biometric = false;
  Duration _lockDuration = const Duration(minutes: 1);
  int _lockRevision = 0;

  bool get hasPasscode => _passcode != null;
  bool get biometric => _biometric;
  Duration get lockDuration => _lockDuration;
  int get lockRevision => _lockRevision;

  String get _prefix => 'security.$accountId.';

  Future<void> initialize() async {
    _passcode = await _settings.get('${_prefix}passcode') as String?;
    _biometric = await _settings.get('${_prefix}biometric') as bool? ?? false;
    _lockDuration = Duration(
      minutes:
          (await _settings.get('${_prefix}lockDuration') as num?)?.toInt() ?? 1,
    );
    notifyListeners();
  }

  Future<void> setPasscode(String? value) async {
    if (value != null && (value.length != 6 || int.tryParse(value) == null)) {
      throw ArgumentError('Passcode must be 6 digits');
    }
    _passcode = value;
    if (value == null) {
      await _settings.set('${_prefix}passcode', null);
      _biometric = false;
      _lockDuration = const Duration(minutes: 1);
      await _settings.set('${_prefix}biometric', false);
      await _settings.set('${_prefix}lockDuration', null);
    } else {
      await _settings.set('${_prefix}passcode', value);
    }
    notifyListeners();
  }

  Future<void> setBiometric(bool value) async {
    _biometric = value;
    await _settings.set('${_prefix}biometric', value);
    notifyListeners();
  }

  Future<void> setLockDuration(Duration value) async {
    _lockDuration = value;
    await _settings.set('${_prefix}lockDuration', value.inMinutes);
    notifyListeners();
  }

  bool verify(String value) => _passcode == value;

  void lockNow() {
    if (!hasPasscode) return;
    _lockRevision++;
    notifyListeners();
  }

  Future<bool> canAuthenticate() async {
    if (kIsWeb || Platform.isWindows || Platform.isLinux) return false;
    if (!await _authentication.canCheckBiometrics) return false;
    if (!await _authentication.isDeviceSupported()) return false;
    return (await _authentication.getAvailableBiometrics()).isNotEmpty;
  }

  Future<bool> authenticate(String reason) async {
    try {
      return await _authentication.authenticate(
        localizedReason: reason,
        biometricOnly: true,
      );
    } on LocalAuthException catch (error) {
      if (error.code == LocalAuthExceptionCode.authInProgress) {
        await _authentication.stopAuthentication();
        return authenticate(reason);
      }
      return false;
    } on Object {
      return false;
    }
  }
}
