import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/controllers/sticker_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

enum AppStage { starting, signedOut, signedIn, failed }

class AppController extends ChangeNotifier {
  AppStage stage = AppStage.starting;
  DesktopHandle? _desktop;
  AccountHandle? _account;
  String? error;
  var _requestVersion = 0;

  DesktopHandle get desktop => _desktop!;
  AccountHandle get account => _account!;

  Future<void> initialize() async {
    final requestVersion = ++_requestVersion;
    stage = AppStage.starting;
    error = null;
    notifyListeners();
    try {
      _desktop ??= await openDesktop();
      final account = await _desktop!.restoreAccount();
      if (requestVersion != _requestVersion) {
        account?.dispose();
        return;
      }
      _account = account;
      stage = account == null ? AppStage.signedOut : AppStage.signedIn;
      if (account != null) unawaited(_preloadStickers(account));
    } catch (exception) {
      if (requestVersion != _requestVersion) return;
      // A stale or revoked saved session should return to QR login.
      if (_desktop != null) {
        stage = AppStage.signedOut;
      } else {
        error = exception.toString();
        stage = AppStage.failed;
      }
    }
    notifyListeners();
  }

  void setAccount(AccountHandle account) {
    _account = account;
    stage = AppStage.signedIn;
    error = null;
    notifyListeners();
    unawaited(_preloadStickers(account));
  }

  Future<void> signOut() async {
    final account = _account;
    if (account == null) return;
    final requestVersion = ++_requestVersion;
    error = null;
    notifyListeners();
    try {
      await account.signOut();
      if (requestVersion != _requestVersion) return;
      account.dispose();
      _account = null;
      stage = AppStage.signedOut;
    } catch (exception, stackTrace) {
      if (requestVersion != _requestVersion) return;
      account.dispose();
      _account = null;
      error = exception.toString();
      stage = AppStage.signedOut;
      notifyListeners();
      Error.throwWithStackTrace(exception, stackTrace);
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _account?.shutdown();
    _account?.dispose();
    _desktop?.dispose();
    super.dispose();
  }
}

Future<void> _preloadStickers(AccountHandle account) async {
  try {
    await StickerController.refreshRemote(account);
  } on Object {
    // Sticker data is optional during account startup and can retry on picker open.
  }
}
