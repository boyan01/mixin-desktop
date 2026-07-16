import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/pages/home_page.dart';
import 'package:mixin_desktop_ui/pages/login_page.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:provider/provider.dart';

class MixinDesktopApp extends StatelessWidget {
  const MixinDesktopApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Mixin Messenger',
    debugShowCheckedModeBanner: false,
    theme: buildMixinTheme(Brightness.light),
    darkTheme: buildMixinTheme(Brightness.dark),
    themeMode: ThemeMode.system,
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    home: const _AppBody(),
  );
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
      AppStage.signedIn => Provider<AccountHandle>.value(
        value: controller.account,
        child: const HomePage(),
      ),
      AppStage.failed => _FailurePage(
        message: controller.error ?? 'Unable to start Mixin Messenger.',
        onRetry: controller.initialize,
      ),
    };
  }
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
