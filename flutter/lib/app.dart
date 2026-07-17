import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/controllers/network_controller.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/network/core_http_scope.dart';
import 'package:mixin_desktop_ui/pages/home_page.dart';
import 'package:mixin_desktop_ui/pages/login_page.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:provider/provider.dart';

class MixinDesktopApp extends StatelessWidget {
  const MixinDesktopApp({super.key});

  static const supportedLocales = <Locale>[
    Locale.fromSubtags(languageCode: 'en'),
    Locale.fromSubtags(languageCode: 'es'),
    Locale.fromSubtags(languageCode: 'id'),
    Locale.fromSubtags(languageCode: 'ja'),
    Locale.fromSubtags(languageCode: 'ms'),
    Locale.fromSubtags(languageCode: 'ru'),
    Locale.fromSubtags(languageCode: 'zh-HK'),
    Locale.fromSubtags(languageCode: 'zh-TW'),
    Locale.fromSubtags(languageCode: 'zh'),
  ];

  @override
  Widget build(BuildContext context) {
    final settings = context.watch<SettingsController>();
    return MaterialApp(
      title: 'Mixin Messenger',
      debugShowCheckedModeBanner: false,
      theme: buildMixinTheme(Brightness.light),
      darkTheme: buildMixinTheme(Brightness.dark),
      themeMode: settings.themeMode,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: supportedLocales,
      home: const _AppBody(),
    );
  }
}

class _AppBody extends StatelessWidget {
  const _AppBody();

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<AppController>();
    return switch (controller.stage) {
      AppStage.starting => const _StartupPage(),
      AppStage.signedOut => LoginPage(
        desktop: controller.desktop,
        onAuthenticated: controller.setAccount,
      ),
      AppStage.signedIn => _SignedInBody(
        desktop: controller.desktop,
        account: controller.account,
      ),
      AppStage.failed => _FailurePage(
        message: controller.error ?? 'Unable to start Mixin Messenger.',
        onRetry: controller.initialize,
      ),
    };
  }
}

class _SignedInBody extends StatefulWidget {
  const _SignedInBody({required this.desktop, required this.account});

  final DesktopHandle desktop;
  final AccountHandle account;

  @override
  State<_SignedInBody> createState() => _SignedInBodyState();
}

class _SignedInBodyState extends State<_SignedInBody> {
  late final NetworkController _networkController = NetworkController(
    widget.desktop,
  )..initialize();

  @override
  void dispose() {
    _networkController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Provider<AccountHandle>.value(
    value: widget.account,
    child: ChangeNotifierProvider<NetworkController>.value(
      value: _networkController,
      child: Consumer<NetworkController>(
        builder: (context, network, child) => CoreHttpScope(
          client: network.httpClient,
          revision: network.revision,
          child: child!,
        ),
        child: const HomePage(),
      ),
    ),
  );
}

class _StartupPage extends StatelessWidget {
  const _StartupPage();

  @override
  Widget build(BuildContext context) => const Scaffold(
    body: Center(child: CircularProgressIndicator(strokeWidth: 2)),
  );
}

class _FailurePage extends StatelessWidget {
  const _FailurePage({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => Scaffold(
    body: Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.error_outline, size: 40),
              const SizedBox(height: 16),
              Text(message, textAlign: TextAlign.center),
              const SizedBox(height: 20),
              FilledButton(onPressed: onRetry, child: const Text('Retry')),
            ],
          ),
        ),
      ),
    ),
  );
}
