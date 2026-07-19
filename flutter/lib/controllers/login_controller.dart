import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

enum LoginStatus { loading, ready, provisioning, failed }

class LoginController extends ChangeNotifier {
  LoginController(this.desktop, this.onAuthenticated, this.onFailure) {
    refresh();
  }

  final DesktopHandle desktop;
  final ValueChanged<AccountHandle> onAuthenticated;
  final void Function(Object error, StackTrace stackTrace) onFailure;

  LoginStatus status = LoginStatus.loading;
  String? authUrl;
  String? error;
  LoginHandle? _login;
  var _requestVersion = 0;

  Future<void> refresh() async {
    final requestVersion = ++_requestVersion;
    _login?.cancel();
    _login?.dispose();
    _login = null;
    authUrl = null;
    error = null;
    status = LoginStatus.loading;
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
      unawaited(_waitForAuthentication(login, requestVersion));
    } catch (exception) {
      if (requestVersion != _requestVersion) return;
      status = LoginStatus.failed;
      error = exception.toString();
      notifyListeners();
    }
  }

  Future<void> _waitForAuthentication(
    LoginHandle login,
    int requestVersion,
  ) async {
    try {
      final account = await login.wait();
      if (requestVersion != _requestVersion || !identical(_login, login)) {
        unawaited(account.shutdown());
        account.dispose();
        return;
      }
      _login = null;
      login.dispose();
      onAuthenticated(account);
    } catch (exception, stackTrace) {
      if (requestVersion != _requestVersion || !identical(_login, login)) {
        return;
      }
      if (exception.toString().contains('authorization timed out')) {
        unawaited(refresh());
        return;
      }
      if (exception.toString().contains('login_provisioning_error:')) {
        onFailure(exception, stackTrace);
        return;
      }
      status = LoginStatus.failed;
      error = exception.toString();
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _requestVersion += 1;
    _login?.cancel();
    _login?.dispose();
    super.dispose();
  }
}
