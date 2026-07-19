import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';

void main() {
  test('uses simplified Chinese for a zh-Hans system locale', () {
    final locale = basicLocaleListResolution(const [
      Locale.fromSubtags(
        languageCode: 'zh',
        scriptCode: 'Hans',
        countryCode: 'CN',
      ),
    ], AppLocalizations.supportedLocales);

    expect(locale, const Locale('zh'));
  });
}
