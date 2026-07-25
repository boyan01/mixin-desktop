import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/image_by_blur_hash.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:provider/provider.dart';

import 'test_settings_store.dart';

void main() {
  test(
    'unresolved media messages use the upstream normal bubble semantics',
    () {
      final message = _message(category: 'SIGNAL_STICKER', status: 'UNKNOWN');

      expect(message.isUnresolvedMessage, isTrue);
      expect(message.showMessageBubble, isTrue);
      expect(message.includeMessageBubbleNip, isFalse);
      expect(message.clipMessageBubble, isFalse);
      expect(message.useOuterMessageDateAndStatus, isFalse);
      expect(message.messageBubblePadding, const EdgeInsets.all(8));
    },
  );

  testWidgets('renders plain text from projected content', (tester) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    Future<void> render(MessageListEntry message) => tester.pumpWidget(
      _TestApp(
        child: MessageContent(
          message: message,
          isCurrentUser: false,
          currentUserId: 'me',
          dateAndStatus: const Text('12:30'),
          overlayDateAndStatus: const Text('12:30 overlay'),
        ),
      ),
    );

    await render(_message(content: 'Visible text'));
    expect(find.text('Visible text', findRichText: true), findsOneWidget);
  });

  testWidgets('renders projected media fields without raw payloads', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    Future<void> render(MessageListEntry message) => tester.pumpWidget(
      _TestApp(
        child: MessageContent(
          message: message,
          isCurrentUser: false,
          currentUserId: 'me',
          dateAndStatus: const Text('12:30'),
          overlayDateAndStatus: const Text('12:30 overlay'),
          onOpenSticker: (_) {},
        ),
      ),
    );

    await render(
      _message(
        id: 'image',
        category: 'PLAIN_IMAGE',
        content: 'RAW_IMAGE_PAYLOAD',
        caption: 'Image caption',
        mediaWidth: 640,
        mediaHeight: 480,
        mediaStatus: 'CANCELED',
      ),
    );
    expect(find.byKey(const Key('message-media-image-image')), findsOneWidget);
    expect(find.text('Image caption', findRichText: true), findsOneWidget);
    expect(find.text('RAW_IMAGE_PAYLOAD', findRichText: true), findsNothing);

    await render(
      _message(
        id: 'video',
        category: 'PLAIN_VIDEO',
        content: 'RAW_VIDEO_PAYLOAD',
        caption: 'Video caption',
        mediaDuration: '65000',
        mediaWidth: 1920,
        mediaHeight: 1080,
        mediaStatus: 'PENDING',
      ),
    );
    expect(find.byKey(const Key('message-media-video-video')), findsOneWidget);
    expect(find.text('RAW_VIDEO_PAYLOAD', findRichText: true), findsNothing);

    await render(
      _message(
        id: 'audio',
        category: 'PLAIN_AUDIO',
        content: 'RAW_AUDIO_PAYLOAD',
        mediaDuration: '65000',
        mediaStatus: 'CANCELED',
      ),
    );
    expect(find.byKey(const Key('message-media-audio-audio')), findsOneWidget);
    expect(find.text('RAW_AUDIO_PAYLOAD', findRichText: true), findsNothing);

    await render(
      _message(
        id: 'sticker',
        category: 'PLAIN_STICKER',
        content: 'RAW_STICKER_PAYLOAD',
        stickerId: 'sticker-id',
        stickerAssetWidth: 128,
        stickerAssetHeight: 128,
      ),
    );
    expect(find.byType(InteractiveDecoratedBox), findsOneWidget);
    expect(find.text('RAW_STICKER_PAYLOAD', findRichText: true), findsNothing);

    await render(
      _message(
        id: 'data',
        category: 'PLAIN_DATA',
        content: '{"name":"report.pdf","secret":"RAW_DATA_PAYLOAD"}',
        mediaName: 'report.pdf',
        mediaMimeType: 'application/pdf',
        mediaSize: 1536,
        mediaStatus: 'CANCELED',
      ),
    );
    expect(find.byKey(const Key('message-media-file-data')), findsOneWidget);
    final file = tester.widget<MessageFile>(find.byType(MessageFile));
    expect(file.message.mediaName, 'report.pdf');
    expect(file.message.mediaSize, 1536);
    expect(find.textContaining('RAW_DATA_PAYLOAD'), findsNothing);
  });

  testWidgets('renders structured content from projected fields', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    Future<void> render(MessageListEntry message) => tester.pumpWidget(
      _TestApp(
        child: MessageContent(
          message: message,
          isCurrentUser: false,
          currentUserId: 'me',
          dateAndStatus: const Text('12:30'),
          overlayDateAndStatus: const Text('12:30 overlay'),
        ),
      ),
    );

    await render(
      _message(
        id: 'post',
        category: 'PLAIN_POST',
        content: '# Release notes\nReady',
      ),
    );
    expect(find.byKey(const Key('message-media-post-post')), findsOneWidget);
    expect(find.textContaining('Release notes'), findsOneWidget);
    expect(find.textContaining('Ready'), findsOneWidget);

    await render(
      _message(
        id: 'contact',
        category: 'PLAIN_CONTACT',
        content: 'RAW_CONTACT_PAYLOAD',
        sharedUserId: 'shared-user',
        sharedUserFullName: 'Shared Alice',
        sharedUserIdentityNumber: '700001',
      ),
    );
    final contact = tester.widget<ContactMessageItem>(
      find.byType(ContactMessageItem),
    );
    expect(contact.message.sharedUserFullName, 'Shared Alice');
    expect(find.text('700001', findRichText: true), findsOneWidget);
    expect(find.text('RAW_CONTACT_PAYLOAD', findRichText: true), findsNothing);

    await render(
      _message(
        id: 'location',
        category: 'PLAIN_LOCATION',
        content:
            '{"latitude":37.7,"longitude":-122.4,'
            '"name":"Office","address":"Mixin Street"}',
      ),
    );
    await tester.pump(const Duration(milliseconds: 20));
    final location = tester.widget<LocationMessageItem>(
      find.byType(LocationMessageItem),
    );
    expect(location.message.content, contains('Mixin Street'));
    expect(find.textContaining('latitude'), findsNothing);

    await render(
      _message(
        id: 'transcript',
        category: 'PLAIN_TRANSCRIPT',
        content:
            '[{"name":"Alice","category":"PLAIN_TEXT",'
            '"content":"Transcript hello","secret":"RAW_TRANSCRIPT"}]',
      ),
    );
    expect(find.text('Transcript', findRichText: true), findsOneWidget);
    final transcript = tester.widget<TranscriptMessageItem>(
      find.byType(TranscriptMessageItem),
    );
    expect(transcript.message.content, contains('Transcript hello'));
    expect(find.textContaining('RAW_TRANSCRIPT'), findsNothing);

    await render(
      _message(
        id: 'button-group',
        category: 'APP_BUTTON_GROUP',
        content:
            '[{"label":"Approve","action":"mixin://approve",'
            '"secret":"RAW_BUTTON_PAYLOAD"}]',
      ),
    );
    expect(find.text('Approve', findRichText: true), findsOneWidget);
    expect(find.textContaining('RAW_BUTTON_PAYLOAD'), findsNothing);

    await render(
      _message(
        id: 'app-card',
        category: 'APP_CARD',
        content:
            '{"title":"Card title","description":"Card description",'
            '"action":"","secret":"RAW_CARD_PAYLOAD"}',
      ),
    );
    expect(find.text('Card title', findRichText: true), findsOneWidget);
    expect(find.text('Card description', findRichText: true), findsOneWidget);
    expect(find.textContaining('RAW_CARD_PAYLOAD'), findsNothing);

    await render(
      _message(
        id: 'snapshot',
        category: 'SYSTEM_ACCOUNT_SNAPSHOT',
        content: '{"amount":"RAW_SNAPSHOT_PAYLOAD","symbol":"RAW"}',
        snapshotId: 'snapshot-id',
        snapshotAmount: '42.5',
        snapshotAssetSymbol: 'XIN',
      ),
    );
    expect(find.text('42.5', findRichText: true), findsOneWidget);
    expect(find.text('XIN', findRichText: true), findsOneWidget);
    expect(find.textContaining('RAW_SNAPSHOT_PAYLOAD'), findsNothing);
  });

  testWidgets('renders system and unsupported message states', (tester) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    Future<void> render(MessageListEntry message) => tester.pumpWidget(
      _TestApp(
        child: MessageContent(
          message: message,
          isCurrentUser: false,
          currentUserId: 'me',
          dateAndStatus: const Text('12:30'),
          overlayDateAndStatus: const Text('12:30 overlay'),
        ),
      ),
    );

    await render(
      _message(
        id: 'system-conversation',
        category: 'SYSTEM_CONVERSATION',
        content: 'RAW_SYSTEM_PAYLOAD',
        action: 'JOIN',
        participantId: 'alice',
        participantFullName: 'Alice Participant',
      ),
    );
    expect(
      find.textContaining('Alice Participant', findRichText: true),
      findsOneWidget,
    );
    expect(find.text('RAW_SYSTEM_PAYLOAD'), findsNothing);

    await render(
      _message(
        id: 'recall',
        category: 'MESSAGE_RECALL',
        content: 'RAW_RECALL_PAYLOAD',
      ),
    );
    expect(
      find.text('This message was deleted', findRichText: true),
      findsOneWidget,
    );
    expect(find.text('RAW_RECALL_PAYLOAD'), findsNothing);

    await render(
      _message(
        id: 'unknown-status',
        content: 'RAW_UNKNOWN_STATUS',
        status: 'UNKNOWN',
      ),
    );
    expect(
      find.byKey(const Key('message-media-unknown-unknown-status')),
      findsOneWidget,
    );
    expect(
      find.textContaining(
        'This type of message is not supported',
        findRichText: true,
      ),
      findsOneWidget,
    );
    expect(
      find.textContaining('Learn More', findRichText: true),
      findsOneWidget,
    );
    expect(find.text('RAW_UNKNOWN_STATUS'), findsNothing);

    await render(
      _message(
        content: 'RAW_FAILED_PAYLOAD',
        status: 'FAILED',
      ),
    );
    expect(
      find.textContaining(
        'Waiting for Alice to get online and establish an encrypted session.',
        findRichText: true,
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('message-media-waiting-message')),
      findsOneWidget,
    );
    expect(find.text('RAW_FAILED_PAYLOAD'), findsNothing);

    await render(
      _message(
        id: 'unknown-category',
        category: 'FUTURE_CATEGORY',
        content: 'RAW_UNKNOWN_CATEGORY',
      ),
    );
    expect(
      find.byKey(const Key('message-media-unknown-unknown-category')),
      findsOneWidget,
    );
    expect(find.text('RAW_UNKNOWN_CATEGORY'), findsNothing);
  });

  testWidgets('opens sticker detail from a sticker message', (tester) async {
    MessageListEntry? opened;
    await tester.pumpWidget(
      _TestApp(
        child: MessageContent(
          message: _message(
            id: 'sticker',
            category: 'PLAIN_STICKER',
            stickerId: 'sticker-id',
            stickerAssetWidth: 128,
            stickerAssetHeight: 128,
          ),
          isCurrentUser: false,
          currentUserId: 'me',
          dateAndStatus: const Text('12:30'),
          overlayDateAndStatus: const Text('12:30 overlay'),
          onOpenSticker: (message) => opened = message,
        ),
      ),
    );

    await tester.tap(find.byType(InteractiveDecoratedBox));

    expect(opened?.stickerId, 'sticker-id');
  });

  testWidgets('routes canceled and pending media taps to their callbacks', (
    tester,
  ) async {
    final downloaded = <String>[];
    final canceled = <String>[];

    await tester.pumpWidget(
      _TestApp(
        child: Column(
          children: [
            MessageContent(
              message: _message(
                id: 'canceled-image',
                category: 'PLAIN_IMAGE',
                content: 'RAW_IMAGE',
                mediaStatus: 'CANCELED',
                mediaWidth: 200,
                mediaHeight: 200,
              ),
              isCurrentUser: false,
              dateAndStatus: const Text('time'),
              overlayDateAndStatus: const Text('overlay'),
              onDownloadAttachment: (message) => downloaded.add(message.id),
            ),
            MessageContent(
              message: _message(
                id: 'pending-video',
                category: 'PLAIN_VIDEO',
                content: 'RAW_VIDEO',
                mediaStatus: 'PENDING',
                mediaWidth: 200,
                mediaHeight: 200,
              ),
              isCurrentUser: false,
              dateAndStatus: const Text('time'),
              overlayDateAndStatus: const Text('overlay'),
              onCancelAttachment: (message) => canceled.add(message.id),
            ),
          ],
        ),
      ),
    );

    await tester.tap(
      find.byKey(const Key('message-media-image-canceled-image')),
    );
    await tester.tap(
      find.byKey(const Key('message-media-video-pending-video')),
    );
    await tester.pump();

    expect(downloaded, ['canceled-image']);
    expect(canceled, ['pending-video']);
  });

  testWidgets('keeps invalid quotes inert and opens valid quote previews', (
    tester,
  ) async {
    final opened = <String>[];

    Future<void> render(MessageListEntry message) => tester.pumpWidget(
      _TestApp(
        child: MessageBubble(
          isCurrentUser: false,
          showNip: false,
          quote: buildMessageQuotePreview(message, onOpenMessage: opened.add),
          child: MessageContent(
            message: message,
            isCurrentUser: false,
            dateAndStatus: const Text('time'),
            overlayDateAndStatus: const Text('overlay'),
            onOpenMessage: opened.add,
          ),
        ),
      ),
    );

    await render(
      _message(
        content: 'Reply body',
        quoteMessageId: 'missing',
        quoteContent: 'not-json',
      ),
    );
    expect(find.byKey(const Key('quote-message-preview')), findsOneWidget);
    expect(find.byType(InkWell), findsNothing);
    await tester.tap(find.byKey(const Key('quote-message-preview')));
    expect(opened, isEmpty);

    await render(
      _message(content: 'Reply body', quoteMessageId: 'unavailable'),
    );
    expect(find.byKey(const Key('quote-message-preview')), findsOneWidget);

    await render(
      _message(
        content: 'Reply body',
        quoteMessageId: 'quoted-message',
        quoteContent:
            '{"user_full_name":"Quoted Alice","type":"PLAIN_DATA",'
            '"content":"RAW_QUOTE_PAYLOAD","media_name":"quote.pdf"}',
      ),
    );
    expect(find.byKey(const Key('quote-message-preview')), findsOneWidget);
    await tester.tap(find.byKey(const Key('quote-message-preview')));
    await tester.pump();
    expect(opened, ['quoted-message']);
  });

  testWidgets('matches Flutter quote preview geometry and typography', (
    tester,
  ) async {
    await tester.pumpWidget(
      _TestApp(
        child: MessageBubble(
          isCurrentUser: false,
          showNip: false,
          quote: buildMessageQuotePreview(
            _message(
              content: 'Reply body',
              quoteMessageId: 'quoted-contact',
              quoteContent:
                  '{"user_full_name":"Quoted Alice","type":"PLAIN_CONTACT",'
                  '"shared_user_id":"bob","shared_user_full_name":"Bob",'
                  '"shared_user_identity_number":"7000"}',
            ),
          ),
          child: MessageContent(
            message: _message(
              content: 'Reply body',
              quoteMessageId: 'quoted-contact',
              quoteContent:
                  '{"user_full_name":"Quoted Alice","type":"PLAIN_CONTACT",'
                  '"shared_user_id":"bob","shared_user_full_name":"Bob",'
                  '"shared_user_identity_number":"7000"}',
            ),
            isCurrentUser: false,
            dateAndStatus: const Text('time'),
            overlayDateAndStatus: const Text('overlay'),
          ),
        ),
      ),
    );

    final preview = find.byKey(const Key('quote-message-preview'));
    final accent = find.byKey(const Key('quote-message-accent'));
    final image = find.byKey(const Key('quote-message-image'));
    expect(tester.getSize(preview).height, greaterThanOrEqualTo(50));
    expect(tester.getSize(accent).width, 6);
    expect(tester.getSize(image), const Size.square(48));
    expect(
      tester.widget<Container>(preview).color,
      const Color.fromRGBO(0, 0, 0, 0.04),
    );
    expect(
      tester.getTopLeft(image).dx,
      greaterThan(tester.getTopLeft(preview).dx),
    );
  });

  testWidgets('keeps Flutter quote media preview widgets by category', (
    tester,
  ) async {
    await tester.pumpWidget(
      const _TestApp(
        child: Column(
          children: [
            QuoteMessagePreview(
              raw:
                  '{"type":"PLAIN_IMAGE","media_url":"data:image/png;base64,'
                  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9WlF9ScAAAAASUVORK5CYII="}',
              messageId: 'image',
              membership: null,
              mentionNames: {},
              onOpenMessage: null,
            ),
            QuoteMessagePreview(
              raw: '{"type":"PLAIN_VIDEO","thumb_image":"invalid"}',
              messageId: 'video',
              membership: null,
              mentionNames: {},
              onOpenMessage: null,
            ),
          ],
        ),
      ),
    );

    expect(find.byType(MixinImage), findsOneWidget);
    expect(find.byType(ImageByBlurHashOrBase64), findsOneWidget);
  });

  testWidgets('lays out an image message with a quote', (tester) async {
    final opened = <String>[];
    final message = _message(
      id: 'reply-image',
      category: 'PLAIN_IMAGE',
      content: 'raw image payload',
      mediaWidth: 800,
      mediaHeight: 600,
      quoteMessageId: 'quoted-message',
      quoteContent:
          '{"user_full_name":"Quoted Alice","type":"PLAIN_TEXT",'
          '"content":"Quoted text"}',
    );

    await tester.pumpWidget(
      _TestApp(
        child: MessageBubble(
          isCurrentUser: false,
          showNip: true,
          quote: buildMessageQuotePreview(message, onOpenMessage: opened.add),
          constrainQuoteWidth: true,
          child: MessageContent(
            message: message,
            isCurrentUser: false,
            dateAndStatus: const Text('time'),
            overlayDateAndStatus: const Text('overlay'),
          ),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    final image = find.byKey(const Key('message-media-image-reply-image'));
    final imageSize = tester.getSize(image);
    expect(imageSize.width, inExclusiveRange(0, 301));
    expect(imageSize.height, closeTo(imageSize.width * 3 / 4, 0.01));
    expect(
      tester.getBottomLeft(find.byKey(const Key('quote-message-preview'))).dy,
      lessThanOrEqualTo(tester.getTopLeft(image).dy),
    );
    await tester.tap(find.byKey(const Key('quote-message-preview')));
    expect(opened, ['quoted-message']);
  });

  testWidgets('does not create a blank sender row for a nameless quote', (
    tester,
  ) async {
    await tester.pumpWidget(
      const _TestApp(
        child: QuoteMessagePreview(
          raw: '{"user_id":"me","type":"PLAIN_TEXT","content":"Quoted"}',
          messageId: 'quoted',
          membership: null,
          mentionNames: {},
          onOpenMessage: null,
        ),
      ),
    );

    expect(find.byType(CustomText), findsOneWidget);
  });

  testWidgets('keeps app card body, actions, and time in separate surfaces', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(800, 600));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await tester.pumpWidget(
      _TestApp(
        child: SizedBox(
          width: 600,
          child: MessageContent(
            message: _message(
              id: 'actions-card',
              category: 'APP_CARD',
              content:
                  '{"title":"Card title","description":"Card description",'
                  '"action":"","actions":[{"label":"Open",'
                  '"action":"https://example.com"}]}',
            ),
            isCurrentUser: false,
            showNip: true,
            dateAndStatus: const Text('12:30', key: Key('card-time')),
            overlayDateAndStatus: const SizedBox.shrink(),
          ),
        ),
      ),
    );
    await tester.pump();

    final card = find.byKey(const Key('app-card-actions-message-actions-card'));
    final body = find.byKey(const Key('app-card-body-actions-card'));
    final actions = find.byKey(const Key('app-card-actions-actions-card'));
    expect(tester.getSize(body).width, 320);
    expect(tester.getSize(actions).width, tester.getSize(body).width);
    expect(tester.getTopLeft(actions).dx, tester.getTopLeft(body).dx);
    expect(tester.getSize(card).height, lessThan(150));
    expect(
      tester.getTopLeft(actions).dy,
      greaterThan(tester.getBottomLeft(body).dy),
    );
    expect(
      tester.getTopLeft(find.byKey(const Key('card-time'))).dy,
      greaterThan(tester.getBottomLeft(actions).dy),
    );

    await tester.pumpWidget(
      _TestApp(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            MessageContent(
              message: _message(
                id: 'compact-card',
                category: 'APP_CARD',
                content:
                    '{"title":"Compact","description":"One line",'
                    '"action":"https://example.com","icon_url":""}',
              ),
              isCurrentUser: false,
              showNip: true,
              dateAndStatus: const Text('12:31'),
              overlayDateAndStatus: const SizedBox.shrink(),
            ),
          ],
        ),
      ),
    );
    await tester.pump();
    expect(
      tester
          .getSize(find.byKey(const Key('app-card-compact-compact-card')))
          .height,
      lessThan(100),
    );
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
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: Center(child: child)),
    ),
  );
}

MessageListEntry _message({
  String id = 'message',
  String category = 'PLAIN_TEXT',
  String content = '',
  String status = 'READ',
  String mediaDuration = '',
  String mediaStatus = '',
  String? mediaName,
  String? mediaMimeType,
  int? mediaSize,
  int? mediaWidth,
  int? mediaHeight,
  String? caption,
  String? action,
  String? participantId,
  String? participantFullName,
  String? snapshotId,
  String? snapshotAmount,
  String? snapshotAssetSymbol,
  String? quoteMessageId,
  String? quoteContent,
  String? sharedUserId,
  String? sharedUserFullName,
  String? sharedUserIdentityNumber,
  int? stickerAssetWidth,
  int? stickerAssetHeight,
  String? stickerId,
}) => MessageListEntry(
  id: id,
  conversationId: 'conversation',
  senderId: 'alice',
  senderName: 'Alice',
  senderAvatarUrl: '',
  senderIsVerified: false,
  category: category,
  content: content,
  status: status,
  createdAt: DateTime(2026, 7, 16, 12, 30),
  mediaDuration: mediaDuration,
  mediaStatus: mediaStatus,
  mediaName: mediaName,
  mediaMimeType: mediaMimeType,
  mediaSize: mediaSize,
  mediaWidth: mediaWidth,
  mediaHeight: mediaHeight,
  caption: caption,
  action: action,
  participantId: participantId,
  participantFullName: participantFullName,
  snapshotId: snapshotId,
  snapshotAmount: snapshotAmount,
  snapshotAssetSymbol: snapshotAssetSymbol,
  quoteMessageId: quoteMessageId,
  quoteContent: quoteContent,
  sharedUserId: sharedUserId,
  sharedUserFullName: sharedUserFullName,
  sharedUserIdentityNumber: sharedUserIdentityNumber,
  stickerAssetWidth: stickerAssetWidth,
  stickerAssetHeight: stickerAssetHeight,
  stickerId: stickerId,
);
