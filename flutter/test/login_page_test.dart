import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/pages/login_page.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/qr_login_card.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';

void main() {
  testWidgets('shows the original landing states in a 520 by 418 card', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final desktop = _FakeDesktopHandle();

    await tester.pumpWidget(
      _LocalizedApp(
        child: LoginPage(
          desktop: desktop,
          onAuthenticated: (_) {},
          onFailure: (_, _) {},
        ),
      ),
    );

    expect(find.text('Initializing…'), findsOneWidget);
    expect(tester.getSize(find.byType(QrLoginCard)), const Size(520, 418));

    desktop.loginCompleter.complete(_FakeLoginHandle());
    await tester.pump();

    expect(
      find.text('Log in to Mixin Messenger with a QR code'),
      findsOneWidget,
    );
    expect(find.byType(PrettyQrView), findsOneWidget);
    expect(desktop.login.waitCalls, 1);

    await tester.pumpWidget(const SizedBox.shrink());
    expect(desktop.login.isCancelled, isTrue);
    expect(desktop.login.isDisposed, isTrue);
  });

  testWidgets('requests a new QR code from the retry overlay', (tester) async {
    var retried = false;

    await tester.pumpWidget(
      _LocalizedApp(
        child: Center(
          child: SizedBox(
            width: 520,
            height: 418,
            child: QrLoginCard(
              authUrl: 'mixin://device/auth?id=test&pub_key=test',
              loading: false,
              provisioning: false,
              error: 'expired',
              onRetry: () => retried = true,
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('Click to reload the QR code'));

    expect(retried, isTrue);
  });
}

class _LocalizedApp extends StatelessWidget {
  const _LocalizedApp({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => MaterialApp(
    theme: buildMixinTheme(Brightness.light),
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    home: child,
  );
}

class _FakeDesktopHandle implements DesktopHandle {
  final loginCompleter = Completer<LoginHandle>();
  late final _FakeLoginHandle login;
  var _isDisposed = false;

  @override
  Future<LoginHandle> beginLogin() async {
    final handle = await loginCompleter.future;
    login = handle as _FakeLoginHandle;
    return handle;
  }

  @override
  Future<AccountHandle?> restoreAccount() async => null;

  @override
  void dispose() => _isDisposed = true;

  @override
  bool get isDisposed => _isDisposed;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakeLoginHandle implements LoginHandle {
  final _account = Completer<AccountHandle>();
  var _isDisposed = false;
  bool isCancelled = false;
  int waitCalls = 0;

  @override
  String authUrl() => 'mixin://device/auth?id=test&pub_key=test';

  @override
  void cancel() => isCancelled = true;

  @override
  Future<AccountHandle> wait() {
    waitCalls++;
    return _account.future;
  }

  @override
  void dispose() => _isDisposed = true;

  @override
  bool get isDisposed => _isDisposed;
}
