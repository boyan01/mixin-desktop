import 'dart:async';
import 'dart:math' as math;
import 'dart:ui';

import 'package:desktop_webview_window/desktop_webview_window.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:overlay_support/overlay_support.dart';
import 'package:protocol_handler/protocol_handler.dart';
import 'package:provider/provider.dart';
import 'package:screen_retriever/screen_retriever.dart';
import 'package:window_manager/window_manager.dart';

import 'app.dart';
import 'controllers/app_controller.dart';
import 'controllers/settings_controller.dart';
import 'src/rust/frb_generated.dart';
import 'theme.dart';
import 'utils/app_logger.dart';
import 'utils/local_notification_center.dart';
import 'utils/system_fonts.dart';
import 'widgets/web_view_navigation_bar.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await initializeLogUtil();
  i('Application started');
  await loadFallbackFonts();
  if (runWebViewTitleBarWidget(
    args,
    builder: (context) => Theme(
      data: buildMixinTheme(Brightness.light),
      child: const Material(
        color: Color(0xFFF0E7EA),
        child: WebViewNavigationBar(),
      ),
    ),
    backgroundColor: const Color(0xFFF0E7EA),
  )) {
    return;
  }
  FlutterError.onError = (details) {
    e('FlutterError', details.exception, details.stack);
    FlutterError.presentError(details);
  };
  PlatformDispatcher.instance.onError = (error, stackTrace) {
    e('Unhandled error', error, stackTrace);
    return true;
  };

  unawaited(initNotificationListener());

  await _initializeDesktopWindow();
  if (defaultTargetPlatform == TargetPlatform.macOS ||
      defaultTargetPlatform == TargetPlatform.windows) {
    await protocolHandler.register('mixin');
  }
  final initialProtocolUrl = defaultTargetPlatform == TargetPlatform.linux
      ? args.firstOrNull
      : await protocolHandler.getInitialUrl();
  final controller = AppController();
  unawaited(controller.initialize());
  final settingsController = SettingsController();
  await settingsController.initialize();
  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: controller),
        ChangeNotifierProvider.value(value: settingsController),
      ],
      child: OverlaySupport.global(
        child: MixinDesktopApp(initialProtocolUrl: initialProtocolUrl),
      ),
    ),
  );

  if (_isDesktop) {
    Size? windowSize;
    if (defaultTargetPlatform == TargetPlatform.windows) {
      const defaultWindowSize = Size(1280, 750);
      try {
        final screen = await screenRetriever.getPrimaryDisplay();
        final visibleSize = screen.visibleSize ?? screen.size;
        windowSize = Size(
          math.min(visibleSize.width, defaultWindowSize.width),
          math.min(visibleSize.height, defaultWindowSize.height),
        );
      } on Object {
        windowSize = defaultWindowSize;
      }
    }
    await windowManager.waitUntilReadyToShow(
      WindowOptions(
        titleBarStyle: defaultTargetPlatform == TargetPlatform.macOS
            ? TitleBarStyle.hidden
            : null,
        minimumSize: const Size(384, 480),
        size: windowSize,
        center: defaultTargetPlatform == TargetPlatform.windows ? true : null,
      ),
      () async {
        await windowManager.show();
        await windowManager.focus();
      },
    );
  }
}

bool get _isDesktop =>
    !kIsWeb &&
    const {
      TargetPlatform.linux,
      TargetPlatform.macOS,
      TargetPlatform.windows,
    }.contains(defaultTargetPlatform);

Future<void> _initializeDesktopWindow() async {
  if (!_isDesktop) return;
  await windowManager.ensureInitialized();
}
