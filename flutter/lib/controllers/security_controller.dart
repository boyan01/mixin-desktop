import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:local_auth/local_auth.dart';
import 'package:shared_preferences/shared_preferences.dart';

class SecurityController extends ChangeNotifier {
  SecurityController(this.accountId);

  final String accountId;
  final LocalAuthentication _authentication = LocalAuthentication();

  SharedPreferences? _preferences;
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
    final preferences = await SharedPreferences.getInstance();
    _preferences = preferences;
    _passcode = preferences.getString('${_prefix}passcode');
    _biometric = preferences.getBool('${_prefix}biometric') ?? false;
    _lockDuration = Duration(
      minutes: preferences.getInt('${_prefix}lockDuration') ?? 1,
    );
    notifyListeners();
  }

  Future<void> setPasscode(String? value) async {
    if (value != null && (value.length != 6 || int.tryParse(value) == null)) {
      throw ArgumentError('Passcode must be 6 digits');
    }
    _passcode = value;
    final preferences = _preferences ?? await SharedPreferences.getInstance();
    _preferences = preferences;
    if (value == null) {
      await preferences.remove('${_prefix}passcode');
      _biometric = false;
      _lockDuration = const Duration(minutes: 1);
      await preferences.setBool('${_prefix}biometric', false);
      await preferences.remove('${_prefix}lockDuration');
    } else {
      await preferences.setString('${_prefix}passcode', value);
    }
    notifyListeners();
  }

  Future<void> setBiometric(bool value) async {
    _biometric = value;
    final preferences = _preferences ?? await SharedPreferences.getInstance();
    _preferences = preferences;
    await preferences.setBool('${_prefix}biometric', value);
    notifyListeners();
  }

  Future<void> setLockDuration(Duration value) async {
    _lockDuration = value;
    final preferences = _preferences ?? await SharedPreferences.getInstance();
    _preferences = preferences;
    await preferences.setInt('${_prefix}lockDuration', value.inMinutes);
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
