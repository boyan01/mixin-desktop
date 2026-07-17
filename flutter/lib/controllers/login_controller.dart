import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

enum LoginStatus { loading, ready, provisioning, failed }

class LoginController extends ChangeNotifier {
  LoginController(this.desktop, this.onAuthenticated) {
    refresh();
  }

  final DesktopHandle desktop;
  final ValueChanged<AccountHandle> onAuthenticated;

  LoginStatus status = LoginStatus.loading;
  String? authUrl;
  String? error;
  LoginHandle? _login;
  Timer? _timer;
  var _polling = false;
  var _ticks = 0;
  var _pollFailures = 0;
  var _requestVersion = 0;

  Future<void> refresh() async {
    final requestVersion = ++_requestVersion;
    _timer?.cancel();
    _login?.dispose();
    _login = null;
    authUrl = null;
    error = null;
    status = LoginStatus.loading;
    _ticks = 0;
    _pollFailures = 0;
    notifyListeners();

    try {
      final login = await desktop.beginLogin();
      if (requestVersion != _requestVersion) {
        login.dispose();
        return;
      }
      _login = login;
      authUrl = login.authUrl();
      status = LoginStatus.ready;
      notifyListeners();
      _timer = Timer.periodic(const Duration(seconds: 1), (_) => _poll());
    } catch (exception) {
      if (requestVersion != _requestVersion) return;
      status = LoginStatus.failed;
      error = exception.toString();
      notifyListeners();
    }
  }

  Future<void> _poll() async {
    if (_polling || _login == null) return;
    if (++_ticks > 60) {
      unawaited(refresh());
      return;
    }
    _polling = true;
    try {
      final account = await _login!.poll();
      _pollFailures = 0;
      if (account == null) return;
      _timer?.cancel();
      status = LoginStatus.provisioning;
      notifyListeners();
      onAuthenticated(account);
    } catch (exception) {
      _pollFailures += 1;
      if (_pollFailures >= 3) {
        _timer?.cancel();
        status = LoginStatus.failed;
        error = exception.toString();
        notifyListeners();
      }
    } finally {
      _polling = false;
    }
  }

  @override
  void dispose() {
    _requestVersion += 1;
    _timer?.cancel();
    _login?.dispose();
    super.dispose();
  }
}
