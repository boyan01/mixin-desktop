import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/pages/settings_page.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:provider/provider.dart';

const _profile = AccountProfile(
  userId: 'ea91421a-98bb-41d2-abcf-af013d8b874b',
  fullName: 'Mixin User',
  avatarUrl: '',
  identityNumber: '7000',
  biography: '',
  phone: '',
  createdAt: '',
);

void main() {
  testWidgets('shows account profile and opens migrated settings pages', (
    tester,
  ) async {
    await _pumpSettings(tester, onSignOut: () async {}, onClose: () {});

    expect(find.text('Mixin User'), findsOneWidget);
    expect(find.text('Mixin ID: 7000'), findsOneWidget);
    expect(tester.widget<AvatarView>(find.byType(AvatarView)).size, 90);
    expect(find.text('Edit Profile'), findsOneWidget);
    expect(find.text('Appearance'), findsOneWidget);

    final editProfileInkWell = tester.widget<InkWell>(
      find.descendant(
        of: find.byKey(const ValueKey('settings-edit-profile')),
        matching: find.byType(InkWell),
      ),
    );
    expect(editProfileInkWell.onTap, isNotNull);

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
    await tester.tap(signOut);
    await tester.pump();

    expect(signOutCount, 1);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(
      tester
          .widget<InkWell>(
            find.descendant(of: signOut, matching: find.byType(InkWell)),
          )
          .onTap,
      isNull,
    );

    completer.complete();
    await tester.pumpAndSettle();
    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(signOutCount, 1);
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
    await tester.pump();

    expect(signOutCount, 1);
    expect(find.text('Failed'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(
      tester
          .widget<InkWell>(
            find.descendant(of: signOut, matching: find.byType(InkWell)),
          )
          .onTap,
      isNotNull,
    );
  });
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
            onClose: onClose,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}
