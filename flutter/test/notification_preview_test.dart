import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/pages/home_page.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

void main() {
  testWidgets('formats app card and pin notification previews', (tester) async {
    late BuildContext context;
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (value) {
            context = value;
            return const SizedBox();
          },
        ),
      ),
    );

    NotificationEvent event({
      required String category,
      required String content,
    }) => NotificationEvent(
      messageId: 'message',
      conversationId: 'conversation',
      senderName: 'Alice',
      category: category,
      content: content,
      createdAtMicros: 0,
      conversationName: 'Conversation',
      conversationCategory: 'GROUP',
    );

    expect(
      notificationPreview(
        context,
        event(category: 'APP_CARD', content: '{"title":"PIN Updated"}'),
      ),
      'Alice: [PIN Updated]',
    );
    expect(
      notificationPreview(
        context,
        event(
          category: 'APP_BUTTON_GROUP',
          content: '[{"label":"Confirm"},{"label":"Cancel"}]',
        ),
      ),
      'Alice: [Confirm][Cancel]',
    );
    expect(
      notificationPreview(
        context,
        event(
          category: 'MESSAGE_PIN',
          content:
              '{"category":"APP_CARD","content":"{\\"title\\":\\"PIN Updated\\"}"}',
        ),
      ),
      'Alice pinned [PIN Updated]',
    );
  });
}
