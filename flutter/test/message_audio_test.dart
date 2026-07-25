import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';
import 'package:provider/provider.dart';

import 'test_settings_store.dart';

void main() {
  testWidgets('dispatches download and cancel actions', (tester) async {
    var downloaded = false;
    var canceled = false;

    await tester.pumpWidget(
      _TestApp(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            AudioMessageWidget(
              message: _audioMessage(id: 'download', mediaStatus: 'CANCELED'),
              onDownloadAttachment: (_) => downloaded = true,
            ),
            AudioMessageWidget(
              message: _audioMessage(id: 'pending', mediaStatus: 'PENDING'),
              onCancelAttachment: (_) => canceled = true,
            ),
          ],
        ),
      ),
    );

    await tester.tap(
      find
          .ancestor(
            of: find.byKey(const Key('message-media-audio-download')),
            matching: find.byType(GestureDetector),
          )
          .first,
    );
    await tester.tap(
      find
          .ancestor(
            of: find.byKey(const Key('message-media-audio-pending')),
            matching: find.byType(GestureDetector),
          )
          .first,
    );
    await tester.pump();
    expect(downloaded, isTrue);
    expect(canceled, isTrue);
  });
}

class _TestApp extends StatelessWidget {
  const _TestApp({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => ChangeNotifierProvider(
    create: (_) => SettingsController(store: TestSettingsStore()),
    child: MaterialApp(
      theme: buildMixinTheme(Brightness.light),
      home: Scaffold(body: Center(child: child)),
    ),
  );
}

MessageListEntry _audioMessage({
  String id = 'audio',
  String mediaStatus = 'READ',
  String mediaDuration = '1000',
  String mediaWaveform = '',
  String senderRelationship = '',
  String? mediaUrl,
}) => MessageListEntry(
  id: id,
  conversationId: 'conversation',
  senderId: 'alice',
  senderName: 'Alice',
  senderAvatarUrl: '',
  senderIsVerified: false,
  category: 'PLAIN_AUDIO',
  content: '',
  status: 'DELIVERED',
  createdAt: DateTime(2026, 7, 16, 12, 30),
  mediaDuration: mediaDuration,
  mediaStatus: mediaStatus,
  mediaWaveform: mediaWaveform,
  senderRelationship: senderRelationship,
  mediaUrl: mediaUrl,
);
