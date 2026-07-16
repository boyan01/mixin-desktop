import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';
import 'package:provider/provider.dart';

void main() {
  testWidgets('matches Flutter audio message geometry and unread waveform', (
    tester,
  ) async {
    await tester.pumpWidget(
      _TestApp(
        child: AudioMessageWidget(
          message: _audioMessage(
            mediaStatus: 'DONE',
            mediaDuration: '65000',
            mediaWaveform: base64Encode([10, 100, 255, 40]),
          ),
        ),
      ),
    );

    final audio = find.byKey(const Key('message-media-audio-audio'));
    final waveform = find.byKey(const Key('audio-waveform-audio'));
    expect(tester.getSize(audio), const Size(284, 38));
    expect(tester.getSize(waveform), const Size(238, 12));
    expect(find.text('1:05'), findsOneWidget);
    expect(find.byKey(const Key('audio-status-audio-play')), findsOneWidget);

    final painter = tester.widget<CustomPaint>(
      find.descendant(of: waveform, matching: find.byType(CustomPaint)),
    );
    final waveformPainter = painter.painter! as AudioWaveformPainter;
    expect(waveformPainter.backgroundColor, lightMixinColors.accent);
    expect(waveformPainter.foregroundColor, lightMixinColors.accent);
  });

  testWidgets(
    'uses read colors and distinguishes download upload and pending',
    (tester) async {
      var downloaded = false;
      var canceled = false;

      await tester.pumpWidget(
        _TestApp(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              AudioMessageWidget(
                message: _audioMessage(id: 'read', mediaStatus: 'READ'),
              ),
              AudioMessageWidget(
                message: _audioMessage(id: 'download', mediaStatus: 'CANCELED'),
                onDownloadAttachment: (_) => downloaded = true,
              ),
              AudioMessageWidget(
                message: _audioMessage(
                  id: 'upload',
                  mediaStatus: 'CANCELED',
                  senderRelationship: 'ME',
                  mediaUrl: '/tmp/audio.ogg',
                ),
                onDownloadAttachment: (_) {},
              ),
              AudioMessageWidget(
                message: _audioMessage(id: 'pending', mediaStatus: 'PENDING'),
                onCancelAttachment: (_) => canceled = true,
              ),
            ],
          ),
        ),
      );

      final readWaveform = find.byKey(const Key('audio-waveform-read'));
      final painter = tester.widget<CustomPaint>(
        find.descendant(of: readWaveform, matching: find.byType(CustomPaint)),
      );
      final waveformPainter = painter.painter! as AudioWaveformPainter;
      expect(
        waveformPainter.backgroundColor,
        lightMixinColors.waveformBackground,
      );
      expect(
        waveformPainter.foregroundColor,
        lightMixinColors.waveformForeground,
      );
      expect(
        find.byKey(const Key('audio-status-download-download')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('audio-status-upload-upload')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('audio-status-pending-pending')),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('audio-status-download-download')));
      await tester.tap(find.byKey(const Key('audio-status-pending-pending')));
      await tester.pump();
      expect(downloaded, isTrue);
      expect(canceled, isTrue);
    },
  );
}

class _TestApp extends StatelessWidget {
  const _TestApp({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => ChangeNotifierProvider(
    create: (_) => SettingsController(),
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
