import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/pages/settings_page.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart' as app_toast;
import 'package:overlay_support/overlay_support.dart';
import 'package:provider/provider.dart';

const _profile = AccountProfile(
  userId: 'ea91421a-98bb-41d2-abcf-af013d8b874b',
  fullName: 'Mixin User',
  avatarUrl: '',
  identityNumber: '7000',
  biography: '',
  phone: '',
  createdAt: '',
  isVerified: false,
  fiatCurrency: 'USD',
);

void main() {
  testWidgets('shows account profile and opens migrated settings pages', (
    tester,
  ) async {
    await _pumpSettings(tester, onSignOut: () async {}, onClose: () {});

    expect(find.text('Mixin User', findRichText: true), findsOneWidget);
    expect(find.text('Mixin ID: 7000', findRichText: true), findsOneWidget);
    expect(find.text('Edit Profile', findRichText: true), findsOneWidget);
    expect(find.text('Appearance', findRichText: true), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('settings-edit-profile')));
    await tester.pumpAndSettle();
    expect(find.text('Save'), findsOneWidget);
    expect(find.text('Biography'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('settings-back')));
    await tester.pumpAndSettle();
    await tester.ensureVisible(
      find.byKey(const ValueKey('settings-appearance')),
    );
    await tester.tap(find.byKey(const ValueKey('settings-appearance')));
    await tester.pumpAndSettle();
    expect(find.text('Follow System'), findsOneWidget);
    expect(find.text('Show avatar'), findsOneWidget);
  });

  testWidgets('runs only one sign out while the action is pending', (
    tester,
  ) async {
    final completer = Completer<void>();
    var signOutCount = 0;
    await _pumpSettings(
      tester,
      onSignOut: () {
        signOutCount++;
        return completer.future;
      },
      onClose: () {},
    );

    final signOut = find.byKey(const ValueKey('settings-sign-out'));
    await tester.ensureVisible(signOut);
    await tester.tapAt(tester.getCenter(signOut));
    await tester.pump();
    await tester.tapAt(tester.getCenter(signOut));
    await tester.pump();

    expect(signOutCount, 1);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    completer.complete();
    await _pumpUntil(
      tester,
      () => find.byType(CircularProgressIndicator).evaluate().isEmpty,
    );
    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(signOutCount, 1);
    app_toast.Toast.dismiss();
    await tester.pump(const Duration(seconds: 2));
  });

  testWidgets('shows a failure without completing sign out', (tester) async {
    var signOutCount = 0;
    await _pumpSettings(
      tester,
      onSignOut: () async {
        signOutCount++;
        throw StateError('sign out failed');
      },
      onClose: () {},
    );

    final signOut = find.byKey(const ValueKey('settings-sign-out'));
    await tester.ensureVisible(signOut);
    await tester.tap(signOut);
    await _pumpUntil(
      tester,
      () => find.textContaining('sign out failed').evaluate().isNotEmpty,
    );

    expect(signOutCount, 1);
    expect(find.textContaining('sign out failed'), findsWidgets);

    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    await tester.tap(signOut);
    await tester.pump();
    expect(signOutCount, 2);
    app_toast.Toast.dismiss();
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
  });
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() condition) async {
  for (var attempt = 0; attempt < 20; attempt++) {
    if (condition()) return;
    await tester.pump(const Duration(milliseconds: 10));
  }
  fail('condition was not reached');
}

Future<void> _pumpSettings(
  WidgetTester tester, {
  required Future<void> Function() onSignOut,
  required VoidCallback onClose,
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = const Size(360, 900);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  await tester.pumpWidget(
    ChangeNotifierProvider(
      create: (_) => SettingsController(),
      child: OverlaySupport.global(
        child: MaterialApp(
          theme: buildMixinTheme(Brightness.light),
          localizationsDelegates: const [
            AppLocalizations.delegate,
            GlobalMaterialLocalizations.delegate,
            GlobalWidgetsLocalizations.delegate,
            GlobalCupertinoLocalizations.delegate,
          ],
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SettingsPage(
              profile: _profile,
              onSignOut: onSignOut,
              onProfileUpdated: (_, _) async {},
              onProfileRefresh: () async => _profile,
              onClose: onClose,
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}
