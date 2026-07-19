import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';

void main() {
  final l10n = lookupAppLocalizations(const Locale('en'));

  test('uses Flutter app category field for pinned message preview', () {
    expect(
      pinMessagePreview(
        l10n,
        '{"message_id":"message-id","category":"PLAIN_TEXT",'
        '"content":"Pinned text"}',
      ),
      'Pinned text',
    );
  });
}
