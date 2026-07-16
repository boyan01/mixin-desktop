import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:lottie/lottie.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/controllers/voice_recorder_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/chat_view.dart';
import 'package:mixin_desktop_ui/widgets/message_action_policy.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_page.dart';
import 'package:provider/provider.dart';
import 'package:visibility_detector/visibility_detector.dart';

void main() {
  setUpAll(() {
    VisibilityDetectorController.instance.updateInterval = Duration.zero;
  });

  testWidgets('matches the Flutter composer layout and mic-send states', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(700, 300));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final controller = TextEditingController();
    final focusNode = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focusNode.dispose);
    var voicePressed = false;

    await tester.pumpWidget(
      _LocalizedApp(
        child: Scaffold(
          body: Align(
            alignment: Alignment.bottomCenter,
            child: ChatInputBar(
              controller: controller,
              focusNode: focusNode,
              quoteMessage: null,
              sending: false,
              onChanged: (_) {},
              onCancelQuote: () {},
              onSend: () async {},
              onVoicePressed: () => voicePressed = true,
            ),
          ),
        ),
      ),
    );

    expect(tester.getSize(find.byKey(const Key('chat-input-bar'))).height, 56);
    expect(
      tester.getSize(find.byKey(const Key('chat-input-surface'))).height,
      40,
    );
    expect(find.byKey(const Key('chat-add')), findsOneWidget);
    expect(find.byKey(const Key('chat-sticker')), findsOneWidget);
    expect(find.byKey(const Key('chat-voice')), findsOneWidget);
    expect(find.byKey(const Key('chat-send')), findsNothing);
    final input = tester.widget<TextField>(find.byKey(const Key('chat-input')));
    expect(input.minLines, 1);
    expect(input.maxLines, 7);
    expect(input.style?.fontSize, 14);

    await tester.tap(find.byKey(const Key('chat-voice')));
    expect(voicePressed, isTrue);
    await tester.enterText(find.byKey(const Key('chat-input')), 'Hello');
    await tester.pump(const Duration(milliseconds: 120));
    expect(find.byKey(const Key('chat-send')), findsOneWidget);
    expect(find.byKey(const Key('chat-voice')), findsNothing);
  });

  testWidgets('anchors the sticker portal in window coordinates', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final account = _FakeAccountHandle([]);
    addTearDown(account.close);

    await tester.pumpWidget(
      _LocalizedApp(
        child: Row(
          children: [
            const SizedBox(width: 300),
            Expanded(
              child: ChatView(
                account: account,
                conversation: _conversation,
                draft: '',
                onDraftChanged: (_) {},
              ),
            ),
          ],
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 100));

    final icon = find.byKey(const Key('chat-sticker'));
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    addTearDown(mouse.removePointer);
    await mouse.addPointer(location: tester.getCenter(icon));
    await tester.pump(const Duration(milliseconds: 60));
    await tester.pump(const Duration(milliseconds: 220));

    final iconRect = tester.getRect(icon);
    final pickerRect = tester.getRect(find.byType(StickerPage));
    expect(pickerRect.center.dx, closeTo(iconRect.center.dx, 1));
    expect(pickerRect.bottom, lessThanOrEqualTo(iconRect.top));
  });

  testWidgets('shows recording controls and recorded voice preview', (
    tester,
  ) async {
    var stopped = false;
    var canceled = false;
    var sent = false;

    Widget bar(VoiceRecorderState state) => _LocalizedApp(
      child: Scaffold(
        body: Align(
          alignment: Alignment.bottomCenter,
          child: VoiceRecorderBar(
            state: state,
            onCancel: () async => canceled = true,
            onStop: () async => stopped = true,
            onRetry: () async {},
            onSend: () async => sent = true,
          ),
        ),
      ),
    );

    await tester.pumpWidget(
      bar(
        const VoiceRecorderState(
          status: VoiceRecorderStatus.recording,
          elapsed: Duration(seconds: 3),
        ),
      ),
    );
    expect(
      tester.getSize(find.byKey(const Key('voice-recorder-bar'))).height,
      56,
    );
    expect(find.text('0:03'), findsOneWidget);
    expect(find.byKey(const Key('voice-stop')), findsOneWidget);
    await tester.tap(find.byKey(const Key('voice-stop')));
    expect(stopped, isTrue);

    await tester.pumpWidget(
      bar(
        const VoiceRecorderState(
          status: VoiceRecorderStatus.recorded,
          elapsed: Duration(seconds: 4),
          recording: VoiceRecording(
            path: '/tmp/voice-preview.ogg',
            duration: Duration(seconds: 4),
            waveform: [10, 30, 20],
          ),
        ),
      ),
    );
    expect(find.byKey(const Key('voice-preview-play')), findsOneWidget);
    expect(find.byKey(const Key('voice-retry')), findsOneWidget);
    await tester.tap(find.byKey(const Key('voice-send')));
    await tester.tap(find.byKey(const Key('voice-cancel')));
    expect(sent, isTrue);
    expect(canceled, isTrue);
  });

  testWidgets('renders Rust messages and sends supported text', (tester) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final account = _FakeAccountHandle([
      _message(
        id: 'incoming-text',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Hello from Rust',
        status: 'READ',
        minute: 30,
      ),
      _message(
        id: 'outgoing-text',
        senderId: 'me',
        senderName: 'Me',
        category: 'PLAIN_TEXT',
        content: 'Hello Alice',
        status: 'READ',
        minute: 31,
      ),
      _message(
        id: 'recall',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'MESSAGE_RECALL',
        content: '',
        status: 'READ',
        minute: 32,
      ),
      _message(
        id: 'image',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_IMAGE',
        content: 'raw image payload',
        status: 'READ',
        minute: 33,
        mediaWidth: 640,
        mediaHeight: 480,
        mediaStatus: 'CANCELED',
      ),
    ]);
    addTearDown(account.close);
    var draft = _conversation.draft;

    await tester.pumpWidget(
      _LocalizedApp(
        child: StatefulBuilder(
          builder: (context, setState) => ChatView(
            account: account,
            conversation: _conversation,
            draft: draft,
            onDraftChanged: (value) => setState(() => draft = value),
          ),
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text('Alice'), findsOneWidget);
    expect(find.text('700010'), findsOneWidget);
    expect(tester.getSize(find.byKey(const Key('chat-header'))).height, 64);
    expect(tester.getSize(find.byKey(const Key('chat-input-bar'))).height, 56);
    expect(find.text('Hello from Rust'), findsOneWidget);
    expect(find.text('Hello Alice'), findsOneWidget);
    expect(find.text('This message was deleted'), findsOneWidget);
    expect(find.byKey(const Key('message-media-image-image')), findsOneWidget);
    expect(
      find.descendant(
        of: find.byKey(const Key('message-media-image-image')),
        matching: find.byType(SvgPicture),
      ),
      findsOneWidget,
    );
    expect(find.text('PLAIN_IMAGE'), findsNothing);
    expect(find.text('raw image payload'), findsNothing);
    expect(
      find.byKey(const Key('message-status-outgoing-text')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      'Draft from Rust',
    );

    await tester.enterText(
      find.byKey(const Key('chat-input')),
      'Sent from test',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('chat-send')));
    await tester.pumpAndSettle();

    expect(account.sentTexts, ['Sent from test']);
    expect(find.text('Sent from test'), findsOneWidget);
    final input = tester.widget<TextField>(find.byKey(const Key('chat-input')));
    expect(input.controller!.text, isEmpty);
    expect(draft, isEmpty);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(milliseconds: 600));
  });

  testWidgets('keeps the draft and shows an error with loaded messages', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final account = _FakeAccountHandle([
      _message(
        id: 'history',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Existing message',
        status: 'READ',
        minute: 30,
      ),
    ])..sendError = StateError('send failed');
    addTearDown(account.close);
    var draft = _conversation.draft;

    await tester.pumpWidget(
      _LocalizedApp(
        child: StatefulBuilder(
          builder: (context, setState) => ChatView(
            account: account,
            conversation: _conversation,
            draft: draft,
            onDraftChanged: (value) => setState(() => draft = value),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Existing message'), findsOneWidget);
    await tester.enterText(
      find.byKey(const Key('chat-input')),
      'Keep this draft',
    );
    await tester.tap(find.byKey(const Key('chat-send')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('chat-message-error')), findsOneWidget);
    expect(find.textContaining('send failed'), findsOneWidget);
    expect(find.text('Existing message'), findsOneWidget);
    expect(draft, 'Keep this draft');
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      'Keep this draft',
    );
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(milliseconds: 600));

    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    'renders the original group header avatar and participant count',
    (tester) async {
      final account = _FakeAccountHandle([]);
      addTearDown(account.close);

      await tester.pumpWidget(
        _LocalizedApp(
          child: ChatView(
            account: account,
            conversation: _groupConversation,
            draft: '',
            onDraftChanged: (_) {},
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Mixin Group'), findsOneWidget);
      expect(find.text('3 PARTICIPANTS'), findsOneWidget);
      expect(find.text('700010'), findsNothing);
      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);
    },
  );

  testWidgets('renders media messages from real fields without fake actions', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 1600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final tempDirectory = Directory.systemTemp.createTempSync(
      'mixin-media-test-',
    );
    addTearDown(() => tempDirectory.deleteSync(recursive: true));
    final imageFile = File('${tempDirectory.path}/preview.png');
    imageFile.writeAsBytesSync(base64Decode(_onePixelPng));

    final messages = [
      _entry(
        id: 'image-media',
        category: 'PLAIN_IMAGE',
        content: 'raw image payload',
        mediaUrl: imageFile.path,
        mediaWidth: 640,
        mediaHeight: 480,
        mediaStatus: 'DONE',
        caption: 'Image caption',
      ),
      _entry(
        id: 'video-media',
        category: 'PLAIN_VIDEO',
        content: 'raw video payload',
        thumbImage: _onePixelPng,
        mediaWidth: 1920,
        mediaHeight: 1080,
        mediaDuration: '65000',
        mediaStatus: 'DONE',
      ),
      _entry(
        id: 'audio-media',
        category: 'PLAIN_AUDIO',
        content: 'raw audio payload',
        mediaDuration: '65000',
        mediaStatus: 'DONE',
      ),
      _entry(
        id: 'sticker-media',
        category: 'PLAIN_STICKER',
        content: 'raw sticker payload',
        mediaUrl: imageFile.path,
        mediaWidth: 128,
        mediaHeight: 128,
        mediaStatus: 'DONE',
      ),
      _entry(
        id: 'file-media',
        category: 'PLAIN_DATA',
        content: '',
        mediaUrl: '/tmp/report.pdf',
        mediaMimeType: 'application/pdf',
        mediaSize: 1536,
        mediaStatus: 'DONE',
      ),
      _entry(
        id: 'post-media',
        category: 'PLAIN_POST',
        content: '# Release notes\nHello',
      ),
      _entry(
        id: 'unknown-media',
        category: 'APP_CARD',
        content: 'invalid app card',
      ),
    ];

    await tester.pumpWidget(
      _LocalizedApp(
        child: Scaffold(
          body: SingleChildScrollView(
            child: Column(
              children: [
                for (final message in messages)
                  MessageContent(
                    message: message,
                    isCurrentUser: false,
                    dateAndStatus: Text('time-${message.id}'),
                    overlayDateAndStatus: Text('overlay-${message.id}'),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(
      find.byKey(const Key('message-media-image-image-media')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('message-media-video-video-media')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('message-media-audio-audio-media')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('message-media-sticker-sticker-media')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('message-media-file-file-media')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('message-media-post-post-media')),
      findsOneWidget,
    );
    expect(find.text('Image caption'), findsOneWidget);
    expect(find.text('1:05'), findsNWidgets(2));
    expect(find.text('report.pdf'), findsOneWidget);
    expect(find.text('1.5 KB'), findsOneWidget);
    expect(
      find.textContaining('Release notes', findRichText: true),
      findsOneWidget,
    );
    expect(find.textContaining('Hello', findRichText: true), findsOneWidget);
    expect(
      find.text(
        'This type of message is not supported, please upgrade Mixin to the '
        'latest version.',
      ),
      findsOneWidget,
    );
    expect(find.text('invalid app card'), findsNothing);

    for (final key in const [
      'message-media-image-image-media',
      'message-media-video-video-media',
      'message-media-audio-audio-media',
      'message-media-sticker-sticker-media',
      'message-media-file-file-media',
      'message-media-post-post-media',
    ]) {
      final media = find.byKey(Key(key));
      expect(
        find.descendant(of: media, matching: find.byType(InkWell)),
        findsNothing,
      );
      expect(
        find.descendant(of: media, matching: find.byType(GestureDetector)),
        findsNothing,
      );
      expect(
        find.descendant(of: media, matching: find.byType(IconButton)),
        findsNothing,
      );
    }

    final localImage = tester.widget<Image>(
      find.descendant(
        of: find.byKey(const Key('message-media-image-image-media')),
        matching: find.byType(Image),
      ),
    );
    expect(localImage.image, isA<FileImage>());
    final thumbnail = tester.widget<Image>(
      find.descendant(
        of: find.byKey(const Key('message-media-video-video-media')),
        matching: find.byType(Image),
      ),
    );
    expect(thumbnail.image, isA<MemoryImage>());
  });

  testWidgets('anchors the first unread message and renders the unread bar', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final account = _FakeAccountHandle([
      _message(
        id: 'older',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Older',
        status: 'READ',
        minute: 20,
      ),
      _message(
        id: 'last-read',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Last read',
        status: 'READ',
        minute: 21,
      ),
      _message(
        id: 'first-unread',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'First unread',
        status: 'DELIVERED',
        minute: 22,
      ),
      _message(
        id: 'latest',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Latest',
        status: 'DELIVERED',
        minute: 23,
      ),
    ]);
    addTearDown(account.close);

    await tester.pumpWidget(
      _LocalizedApp(
        child: ChatView(
          account: account,
          conversation: _unreadConversation,
          draft: '',
          onDraftChanged: (_) {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(account.messagesAroundCalls, 1);
    expect(find.byKey(const Key('unread-message-bar')), findsOneWidget);
    expect(find.text('First unread'), findsOneWidget);
  });

  testWidgets('double click reply sends the selected quote id', (tester) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final account = _FakeAccountHandle([
      _message(
        id: 'reply-target',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Reply target',
        status: 'READ',
        minute: 30,
      ),
    ]);
    addTearDown(account.close);
    await tester.pumpWidget(
      _LocalizedApp(
        child: ChatView(
          account: account,
          conversation: _conversation,
          draft: '',
          onDraftChanged: (_) {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    final bubble = find.byKey(const Key('message-bubble-reply-target'));
    final point = tester.getTopLeft(bubble) + const Offset(4, 4);
    await tester.tapAt(point);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(point);
    await tester.pump();

    expect(find.byKey(const Key('chat-quote-preview')), findsOneWidget);
    expect(find.text('Reply target'), findsNWidgets(2));
    await tester.enterText(find.byKey(const Key('chat-input')), 'Reply body');
    await tester.pump();
    tester.widget<IconButton>(find.byKey(const Key('chat-send'))).onPressed!();
    await tester.pumpAndSettle();
    expect(account.sentTexts.last, 'Reply body');
    expect(account.sentQuoteMessageIds.last, 'reply-target');
  });

  testWidgets('quote preview handles invalid data and locates valid target', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final validQuote = jsonEncode({
      'user_full_name': 'Alice',
      'type': 'PLAIN_TEXT',
      'content': 'Quoted target preview',
    });
    final account = _FakeAccountHandle([
      _message(
        id: 'target',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Original target',
        status: 'READ',
        minute: 20,
      ),
      _message(
        id: 'invalid-quote',
        senderId: 'me',
        senderName: 'Me',
        category: 'PLAIN_TEXT',
        content: 'Invalid quote body',
        status: 'READ',
        minute: 21,
        quoteMessageId: 'missing',
        quoteContent: 'not json',
      ),
      _message(
        id: 'valid-quote',
        senderId: 'me',
        senderName: 'Me',
        category: 'PLAIN_TEXT',
        content: 'Valid quote body',
        status: 'READ',
        minute: 22,
        quoteMessageId: 'target',
        quoteContent: validQuote,
      ),
    ]);
    addTearDown(account.close);
    await tester.pumpWidget(
      _LocalizedApp(
        child: ChatView(
          account: account,
          conversation: _conversation,
          draft: '',
          onDraftChanged: (_) {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Message not found'), findsOneWidget);
    expect(find.text('Quoted target preview'), findsOneWidget);
    final targetPaint = find.descendant(
      of: find.byKey(const Key('message-bubble-target')),
      matching: find.byWidgetPredicate(
        (widget) => widget is CustomPaint && widget.painter is BubblePainter,
      ),
    );
    final before =
        (tester.widget<CustomPaint>(targetPaint).painter! as BubblePainter)
            .color;
    await tester.tap(find.text('Quoted target preview'));
    await tester.pump(const Duration(milliseconds: 300));
    await tester.pump();
    final after =
        (tester.widget<CustomPaint>(targetPaint).painter! as BubblePainter)
            .color;
    expect(after, isNot(before));
    await tester.pump(const Duration(milliseconds: 1200));
  });

  testWidgets('selection copy delete and recall follow message policy', (
    tester,
  ) async {
    String? copiedText;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copiedText =
              (call.arguments as Map<dynamic, dynamic>)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final account = _FakeAccountHandle([
      _message(
        id: 'incoming-select',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'PLAIN_TEXT',
        content: 'Copy me',
        status: 'READ',
        minute: 30,
      ),
      _message(
        id: 'outgoing-select',
        senderId: 'me',
        senderName: 'Me',
        category: 'PLAIN_TEXT',
        content: 'Recall me',
        status: 'READ',
        minute: 31,
        createdAt: DateTime.now().subtract(const Duration(minutes: 5)),
      ),
    ]);
    expect(
      MessageActionPolicy(
        message: MessageListEntry.fromRust(account._messages.last),
        currentUserId: 'me',
        currentUserRole: null,
        now: DateTime.now(),
      ).canRecall,
      isTrue,
    );
    addTearDown(account.close);
    await tester.pumpWidget(
      _LocalizedApp(
        child: ChatView(
          account: account,
          conversation: _conversation,
          draft: '',
          onDraftChanged: (_) {},
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 100));

    final incomingBubble = find.byKey(
      const Key('message-selection-incoming-select'),
    );
    await tester.ensureVisible(incomingBubble);
    await tester.pump();
    await tester.longPressAt(
      tester.getTopLeft(incomingBubble) + const Offset(4, 10),
    );
    await tester.pump(const Duration(milliseconds: 350));
    expect(find.byKey(const Key('chat-selection-bar')), findsOneWidget);
    await tester.tap(find.text('Copy'));
    await tester.pump();
    expect(copiedText, contains('Copy me'));

    await tester.ensureVisible(incomingBubble);
    await tester.longPressAt(
      tester.getTopLeft(incomingBubble) + const Offset(4, 10),
    );
    await tester.pump(const Duration(milliseconds: 350));
    await tester.tap(find.text('Delete'));
    await tester.pump(const Duration(milliseconds: 300));
    expect(find.text('Delete for Everyone'), findsNothing);
    await tester.tap(find.text('Delete for me'));
    await tester.pump(const Duration(seconds: 1));
    expect(account.deletedMessageIds, contains('incoming-select'));
    expect(find.byKey(const Key('chat-selection-bar')), findsNothing);

    final outgoingBubble = find.byKey(
      const Key('message-selection-outgoing-select'),
    );
    await tester.ensureVisible(outgoingBubble);
    await tester.longPressAt(
      tester.getTopLeft(outgoingBubble) + const Offset(4, 10),
    );
    await tester.pump(const Duration(milliseconds: 350));
    expect(find.byKey(const Key('chat-selection-bar')), findsOneWidget);
    await tester.tap(find.text('Delete'));
    await tester.pump(const Duration(milliseconds: 300));
    expect(find.text('Delete for me'), findsOneWidget);
    expect(find.text('Delete for Everyone'), findsOneWidget);
    await tester.tap(find.text('Delete for Everyone'));
    await tester.pump(const Duration(milliseconds: 300));
    expect(account.recalledMessageIds, contains('outgoing-select'));
  });

  testWidgets('renders sticker asset url and Lottie json branches', (
    tester,
  ) async {
    final tempDirectory = Directory.systemTemp.createTempSync(
      'mixin-lottie-test-',
    );
    addTearDown(() => tempDirectory.deleteSync(recursive: true));
    final lottieFile = File('${tempDirectory.path}/sticker.json')
      ..writeAsStringSync(
        '{"v":"5.5.7","fr":30,"ip":0,"op":1,'
        '"w":1,"h":1,"layers":[]}',
      );
    await tester.pumpWidget(
      _LocalizedApp(
        child: Scaffold(
          body: Column(
            children: [
              MessageContent(
                message: _entry(
                  id: 'asset-sticker',
                  category: 'PLAIN_STICKER',
                  content: '',
                  stickerAssetUrl: 'https://example.invalid/sticker.png',
                  stickerAssetWidth: 180,
                  stickerAssetHeight: 120,
                  stickerAssetType: 'png',
                ),
                isCurrentUser: false,
                dateAndStatus: const Text('asset-time'),
                overlayDateAndStatus: const SizedBox.shrink(),
              ),
              MessageContent(
                message: _entry(
                  id: 'lottie-sticker',
                  category: 'PLAIN_STICKER',
                  content: '',
                  stickerAssetUrl: lottieFile.path,
                  stickerAssetWidth: 128,
                  stickerAssetHeight: 128,
                  stickerAssetType: 'json',
                ),
                isCurrentUser: false,
                dateAndStatus: const Text('lottie-time'),
                overlayDateAndStatus: const SizedBox.shrink(),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    final assetImage = tester.widget<Image>(
      find.descendant(
        of: find.byKey(const Key('message-media-sticker-asset-sticker')),
        matching: find.byType(Image),
      ),
    );
    expect(assetImage.image, isA<NetworkImage>());
    expect(
      find.descendant(
        of: find.byKey(const Key('message-media-sticker-lottie-sticker')),
        matching: find.byType(Lottie),
      ),
      findsOneWidget,
    );
  });

  testWidgets('attachment tap downloads canceled and cancels pending media', (
    tester,
  ) async {
    final calls = <String>[];
    await tester.pumpWidget(
      _LocalizedApp(
        child: Scaffold(
          body: Column(
            children: [
              MessageContent(
                message: _entry(
                  id: 'canceled-image',
                  category: 'PLAIN_IMAGE',
                  content: '',
                  mediaStatus: 'CANCELED',
                  mediaWidth: 100,
                  mediaHeight: 100,
                ),
                isCurrentUser: false,
                dateAndStatus: const SizedBox.shrink(),
                overlayDateAndStatus: const SizedBox.shrink(),
                onDownloadAttachment: (_) => calls.add('download'),
              ),
              MessageContent(
                message: _entry(
                  id: 'pending-image',
                  category: 'PLAIN_IMAGE',
                  content: '',
                  mediaStatus: 'PENDING',
                  mediaWidth: 100,
                  mediaHeight: 100,
                ),
                isCurrentUser: false,
                dateAndStatus: const SizedBox.shrink(),
                overlayDateAndStatus: const SizedBox.shrink(),
                onCancelAttachment: (_) => calls.add('cancel'),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('message-media-image-canceled-image')),
    );
    await tester.tap(
      find.byKey(const Key('message-media-image-pending-image')),
    );
    await tester.pump();
    expect(calls, ['download', 'cancel']);
  });

  testWidgets('APP input action sends its text through AccountHandle', (
    tester,
  ) async {
    final account = _FakeAccountHandle([
      _message(
        id: 'app-input',
        senderId: 'alice',
        senderName: 'Alice',
        category: 'APP_BUTTON_GROUP',
        content: '[{"label":"Fill","action":"input: hello"}]',
        status: 'READ',
        minute: 30,
      ),
    ]);
    addTearDown(account.close);
    await tester.pumpWidget(
      _LocalizedApp(
        child: ChatView(
          account: account,
          conversation: _conversation,
          draft: '',
          onDraftChanged: (_) {},
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Fill'));
    await tester.pump();
    expect(account.sentTexts, ['hello']);
    expect(account.sentQuoteMessageIds, [null]);
  });
}

const _onePixelPng =
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=';

final _conversation = ConversationListEntry(
  id: 'conversation',
  ownerId: 'alice',
  name: 'Alice',
  avatarUrl: '',
  category: 'CONTACT',
  draft: 'Draft from Rust',
  status: 2,
  content: '',
  contentType: 'PLAIN_TEXT',
  messageStatus: 'READ',
  senderId: 'alice',
  senderName: 'Alice',
  updatedAt: DateTime(2026, 7, 16, 12),
  unseenCount: 0,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: false,
  isPinned: false,
  relationship: 'FRIEND',
  identityNumber: '700010',
  circleIds: const [],
  groupAvatars: const [],
);

final _groupConversation = ConversationListEntry(
  id: 'group',
  ownerId: 'owner',
  name: 'Mixin Group',
  avatarUrl: '',
  category: 'GROUP',
  draft: '',
  status: 2,
  content: '',
  contentType: 'PLAIN_TEXT',
  messageStatus: 'READ',
  senderId: 'alice',
  senderName: 'Alice',
  updatedAt: DateTime(2026, 7, 16, 12),
  unseenCount: 0,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: false,
  isPinned: false,
  relationship: '',
  identityNumber: '700010',
  circleIds: const [],
  participantCount: 3,
  groupAvatars: const [
    ConversationAvatarEntry(userId: 'alice', name: 'Alice', avatarUrl: ''),
    ConversationAvatarEntry(userId: 'bob', name: 'Bob', avatarUrl: ''),
  ],
);

final _unreadConversation = ConversationListEntry(
  id: 'conversation',
  ownerId: 'alice',
  name: 'Alice',
  avatarUrl: '',
  category: 'CONTACT',
  draft: '',
  status: 2,
  lastReadMessageId: 'last-read',
  content: '',
  contentType: 'PLAIN_TEXT',
  messageStatus: 'DELIVERED',
  senderId: 'alice',
  senderName: 'Alice',
  updatedAt: DateTime(2026, 7, 16, 12),
  unseenCount: 2,
  mentionCount: 0,
  isMuted: false,
  isVerified: false,
  isBot: false,
  isPinned: false,
  relationship: 'FRIEND',
  identityNumber: '700010',
  circleIds: const [],
  groupAvatars: const [],
);

MessageListItem _message({
  required String id,
  required String senderId,
  required String senderName,
  required String category,
  required String content,
  required String status,
  required int minute,
  String? mediaUrl,
  String? mediaMimeType,
  int? mediaSize,
  String mediaDuration = '',
  int? mediaWidth,
  int? mediaHeight,
  String? thumbImage,
  String mediaStatus = '',
  String? caption,
  String? quoteMessageId,
  String? quoteContent,
  String? stickerAssetUrl,
  int? stickerAssetWidth,
  int? stickerAssetHeight,
  String? stickerAssetType,
  DateTime? createdAt,
}) => MessageListItem(
  messageId: id,
  conversationId: 'conversation',
  senderId: senderId,
  senderName: senderName,
  senderAvatarUrl: '',
  senderIsVerified: false,
  senderRelationship: 'FRIEND',
  senderIsScam: false,
  senderIsBot: false,
  category: category,
  content: content,
  status: status,
  createdAtMicros:
      (createdAt ?? DateTime(2026, 7, 16, 12, minute)).microsecondsSinceEpoch,
  mediaUrl: mediaUrl,
  mediaMimeType: mediaMimeType,
  mediaSize: mediaSize,
  mediaDuration: mediaDuration,
  mediaWidth: mediaWidth,
  mediaHeight: mediaHeight,
  thumbImage: thumbImage,
  mediaStatus: mediaStatus,
  caption: caption,
  quoteMessageId: quoteMessageId,
  quoteContent: quoteContent,
  stickerAssetUrl: stickerAssetUrl,
  stickerAssetWidth: stickerAssetWidth,
  stickerAssetHeight: stickerAssetHeight,
  stickerAssetType: stickerAssetType,
  sharedUserIsVerified: false,
  pinned: false,
);

MessageListEntry _entry({
  required String id,
  required String category,
  required String content,
  String? mediaUrl,
  String? mediaMimeType,
  int? mediaSize,
  String mediaDuration = '',
  int? mediaWidth,
  int? mediaHeight,
  String? thumbImage,
  String mediaStatus = '',
  String? caption,
  String? quoteMessageId,
  String? quoteContent,
  String? stickerAssetUrl,
  int? stickerAssetWidth,
  int? stickerAssetHeight,
  String? stickerAssetType,
}) => MessageListEntry.fromRust(
  _message(
    id: id,
    senderId: 'alice',
    senderName: 'Alice',
    category: category,
    content: content,
    status: 'READ',
    minute: 30,
    mediaUrl: mediaUrl,
    mediaMimeType: mediaMimeType,
    mediaSize: mediaSize,
    mediaDuration: mediaDuration,
    mediaWidth: mediaWidth,
    mediaHeight: mediaHeight,
    thumbImage: thumbImage,
    mediaStatus: mediaStatus,
    caption: caption,
    quoteMessageId: quoteMessageId,
    quoteContent: quoteContent,
    stickerAssetUrl: stickerAssetUrl,
    stickerAssetWidth: stickerAssetWidth,
    stickerAssetHeight: stickerAssetHeight,
    stickerAssetType: stickerAssetType,
  ),
);

class _LocalizedApp extends StatelessWidget {
  const _LocalizedApp({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => ChangeNotifierProvider(
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
      home: Portal(child: child),
    ),
  );
}

class _FakeAccountHandle implements AccountHandle {
  _FakeAccountHandle(this._messages);

  final List<MessageListItem> _messages;
  final _changes = StreamController<BigInt>.broadcast();
  final sentTexts = <String>[];
  final sentQuoteMessageIds = <String?>[];
  final downloadedMessageIds = <String>[];
  final canceledMessageIds = <String>[];
  final deletedMessageIds = <String>[];
  final recalledMessageIds = <String>[];
  Object? sendError;
  var _isDisposed = false;
  var messagesAroundCalls = 0;

  @override
  String accountId() => 'me';

  @override
  Stream<BigInt> messageChanges() => _changes.stream;

  @override
  Future<List<MessageListItem>> messages({
    required String conversationId,
    int? beforeCreatedAtMicros,
    String? beforeMessageId,
    required int limit,
  }) async {
    final before = beforeMessageId == null
        ? _messages.length
        : _messages.indexWhere(
            (message) => message.messageId == beforeMessageId,
          );
    final end = before < 0 ? 0 : before;
    return _messages.take(end).toList().reversed.take(limit).toList();
  }

  @override
  Future<String> sendText({
    required String conversationId,
    required String content,
    String? quoteMessageId,
  }) async {
    if (sendError case final error?) throw error;
    sentTexts.add(content);
    sentQuoteMessageIds.add(quoteMessageId);
    final id = 'sent-${sentTexts.length}';
    _messages.add(
      _message(
        id: id,
        senderId: 'me',
        senderName: 'Me',
        category: 'PLAIN_TEXT',
        content: content,
        status: 'SENT',
        minute: 34,
      ),
    );
    return id;
  }

  @override
  Future<List<MessageListItem>> messagesAround({
    required String conversationId,
    required String targetMessageId,
    required int before,
    required int after,
  }) async {
    messagesAroundCalls++;
    return List.of(_messages);
  }

  @override
  Future<String?> currentUserRole({required String conversationId}) async =>
      null;

  @override
  Future<List<MessageListItem>> pinnedMessages({
    required String conversationId,
  }) async => const [];

  @override
  Future<void> downloadAttachment({required String messageId}) async {
    downloadedMessageIds.add(messageId);
  }

  @override
  Future<void> cancelAttachment({required String messageId}) async {
    canceledMessageIds.add(messageId);
  }

  @override
  Future<void> deleteMessages({
    required String conversationId,
    required List<String> messageIds,
  }) async {
    deletedMessageIds.addAll(messageIds);
    _messages.removeWhere((message) => messageIds.contains(message.messageId));
  }

  @override
  Future<void> recallMessages({
    required String conversationId,
    required List<String> messageIds,
  }) async {
    recalledMessageIds.addAll(messageIds);
  }

  @override
  Future<void> markConversationRead({required String conversationId}) async {}

  Future<void> close() => _changes.close();

  @override
  void dispose() => _isDisposed = true;

  @override
  bool get isDisposed => _isDisposed;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
