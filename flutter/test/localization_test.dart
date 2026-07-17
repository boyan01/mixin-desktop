import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/app.dart';

void main() {
  test('uses simplified Chinese for a zh-Hans system locale', () {
    final locale = basicLocaleListResolution(const [
      Locale.fromSubtags(
        languageCode: 'zh',
        scriptCode: 'Hans',
        countryCode: 'CN',
      ),
    ], MixinDesktopApp.supportedLocales);

    expect(locale, const Locale('zh'));
  });
}
