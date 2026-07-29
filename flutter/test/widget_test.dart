import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/widgets/qr_login_card.dart';

void main() {
  testWidgets('shows the QR login instructions', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: QrLoginCard(
            authUrl: 'mixin://device/auth?id=test&pub_key=test',
            loading: false,
            provisioning: false,
            onRetry: () {},
          ),
        ),
      ),
    );

    expect(
      find.text('Log in to Mixin Messenger with a QR code'),
      findsOneWidget,
    );
  });
}
