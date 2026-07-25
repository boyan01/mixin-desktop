import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/attachment_status.dart';
import 'package:provider/provider.dart';

void main() {
  testWidgets(
    'given an active transfer, pending status paints the current attachment progress',
    (tester) async {
      final account = _FakeAccount()..progress = 0.42;

      await tester.pumpWidget(
        Provider<AccountHandle>.value(
          value: account,
          child: MaterialApp(
            theme: buildMixinTheme(Brightness.light),
            home: const Scaffold(
              body: AttachmentStatusPending(messageId: 'message-id'),
            ),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pump(const Duration(milliseconds: 100));

      final indicator = tester.widget<CircularProgressIndicator>(
        find.byType(CircularProgressIndicator),
      );
      expect(indicator.value, 0.42);
      expect(account.requestedMessageIds, isNotEmpty);
      expect(account.requestedMessageIds, everyElement('message-id'));
    },
  );
}

class _FakeAccount implements AccountHandle {
  double progress = 0;
  final requestedMessageIds = <String>[];

  @override
  double attachmentProgress({required String messageId}) {
    requestedMessageIds.add(messageId);
    return progress;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) {
    throw UnsupportedError('Unexpected account call: ${invocation.memberName}');
  }
}
