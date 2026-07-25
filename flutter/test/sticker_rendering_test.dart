import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:lottie/lottie.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/utils/emoji.dart';
import 'package:mixin_desktop_ui/utils/image.dart';
import 'package:mixin_desktop_ui/widgets/media_image_pipeline.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/emoji_page.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_item.dart';

void main() {
  testWidgets('uses the upstream platform emoji font', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [
          AppLocalizations.delegate,
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        supportedLocales: AppLocalizations.supportedLocales,
        home: SizedBox(
          width: 464,
          height: 357,
          child: EmojiPage(
            textEditingController: TextEditingController(),
            recentUsedEmoji: const [],
            onEmojiUsed: (_) {},
          ),
        ),
      ),
    );

    final emoji = tester.widget<Text>(find.text('😀'));
    expect(emoji.style?.fontFamily, kEmojiFontFamily);
  });

  testWidgets('routes gif stickers through MixinImage normalization', (
    tester,
  ) async {
    await tester.pumpWidget(
      const StickerItem(
        assetUrl: 'https://images.mixin.one/sticker',
        assetType: 'gif',
        width: 128,
        height: 128,
      ),
    );

    final image = tester.widget<MixinImage>(find.byType(MixinImage));
    expect(image.normalizeGif, isTrue);
    expect(
      resolveMediaImageProvider(
        image: const NetworkImage('https://images.mixin.one/sticker'),
        normalizeGif: true,
      ),
      isA<NormalizedNetworkImage>(),
    );
  });

  testWidgets('renders a sticker group icon Lottie URL with query parameters', (
    tester,
  ) async {
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: StickerGroupIcon(
          iconUrl: 'https://images.mixin.one/sticker-group.json?version=1',
          size: 48,
        ),
      ),
    );

    expect(find.byType(LottieBuilder), findsOneWidget);
    expect(find.byType(MixinImage), findsNothing);
  });

  test('normalizes zero-delay gif frames like flutter-app', () {
    final bytes = normalizeGifBytesIfNeeded(
      Uint8List.fromList([
        0x47,
        0x49,
        0x46,
        0x38,
        0x39,
        0x61,
        0x21,
        0xF9,
        0x04,
        0,
        0,
        0,
        0,
        0,
      ]),
    );

    expect(bytes[10], 10);
    expect(bytes[11], 0);
  });
}
