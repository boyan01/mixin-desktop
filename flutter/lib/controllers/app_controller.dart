import 'dart:async';

import 'package:flutter/foundation.dart';

import '../src/rust/desktop_api.dart';
import '../utils/app_logger.dart';
import 'settings_store.dart';
import 'sticker_controller.dart';

enum AppStage { starting, signedOut, signedIn, failed }

class AppController extends ChangeNotifier {
  AppController({this.settings});

  final SettingsStore? settings;
  AppStage stage = AppStage.starting;
  DesktopHandle? _desktop;
  AccountHandle? _account;
  String? error;
  String? errorStackTrace;
  DatabaseOpenFailure? databaseOpenFailure;
  bool showLoginFailure = false;
  var _requestVersion = 0;

  DesktopHandle get desktop => _desktop!;
  AccountHandle get account => _account!;

  Future<void> initialize() async {
    final requestVersion = ++_requestVersion;
    stage = AppStage.starting;
    error = null;
    errorStackTrace = null;
    databaseOpenFailure = null;
    showLoginFailure = false;
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
      if (account != null && settings != null) {
        unawaited(_preloadStickers(account, settings!));
      }
    } catch (exception, stackTrace) {
      if (requestVersion != _requestVersion) return;
      final message = exception.toString();
      databaseOpenFailure = DatabaseOpenFailure.tryParse(message);
      error = message;
      errorStackTrace = stackTrace.toString();
      showLoginFailure = _desktop != null && databaseOpenFailure == null;
      stage = AppStage.failed;
    }
    notifyListeners();
  }

  void setAccount(AccountHandle account) {
    _account = account;
    stage = AppStage.signedIn;
    error = null;
    errorStackTrace = null;
    databaseOpenFailure = null;
    showLoginFailure = false;
    notifyListeners();
    if (settings != null) unawaited(_preloadStickers(account, settings!));
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

  Future<void> recreateAccountDatabase() async {
    await desktop.recreateAccountDatabase();
    await initialize();
  }

  void setLoginFailure(Object exception, StackTrace stackTrace) {
    error = exception.toString();
    errorStackTrace = stackTrace.toString();
    databaseOpenFailure = DatabaseOpenFailure.tryParse(error!);
    showLoginFailure = databaseOpenFailure == null;
    stage = AppStage.failed;
    notifyListeners();
  }

  Future<void> abortFailedLogin() async {
    await desktop.abortSavedLogin();
    _account?.dispose();
    _account = null;
    error = null;
    errorStackTrace = null;
    databaseOpenFailure = null;
    showLoginFailure = false;
    stage = AppStage.signedOut;
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

class DatabaseOpenFailure {
  const DatabaseOpenFailure({
    required this.resultCode,
    required this.explanation,
  });

  static const _marker = 'mixin_database_open_error:';

  final int resultCode;
  final String explanation;

  static DatabaseOpenFailure? tryParse(String value) {
    final markerIndex = value.indexOf(_marker);
    if (markerIndex == -1) return null;
    final payload = value.substring(markerIndex + _marker.length);
    final separator = payload.indexOf(':');
    if (separator == -1) return null;
    final resultCode = int.tryParse(payload.substring(0, separator));
    if (resultCode == null) return null;
    return DatabaseOpenFailure(
      resultCode: resultCode,
      explanation: payload.substring(separator + 1),
    );
  }
}

Future<void> _preloadStickers(
  AccountHandle account,
  SettingsStore settings,
) async {
  try {
    await StickerController.refreshRemote(account, settings: settings);
  } on Object catch (error, stackTrace) {
    e('Preload stickers failed', error, stackTrace);
    // Sticker data is optional during account startup and can retry on picker open.
  }
}
