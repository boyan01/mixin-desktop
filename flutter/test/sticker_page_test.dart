import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_data.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_detail_page.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_page.dart';

void main() {
  testWidgets('keeps upstream emoji insertion and sticker send behavior', (
    tester,
  ) async {
    final textController = TextEditingController(text: 'AB')
      ..selection = const TextSelection.collapsed(offset: 1);
    String? sentStickerId;
    await tester.pumpWidget(
      _testApp(
        DefaultTabController(
          length: 4,
          initialIndex: 1,
          child: Builder(
            builder: (context) => StickerPage(
              tabController: DefaultTabController.of(context),
              tabLength: 4,
              presetStickerGroups: const [
                PresetStickerGroup.store,
                PresetStickerGroup.emoji,
                PresetStickerGroup.recent,
                PresetStickerGroup.favorite,
              ],
              stickerAlbums: const [],
              albumStickers: const {},
              recentStickers: const [_sticker],
              personalStickers: const [],
              textEditingController: textController,
              recentUsedEmoji: const ['RECENT'],
              onEmojiUsed: (_) {},
              onStickerSelected: (sticker) async {
                sentStickerId = sticker.stickerId;
                return true;
              },
              onRemoveSticker: (_) async {},
            ),
          ),
        ),
      ),
    );

    expect(find.byKey(const Key('sticker-picker-panel')), findsNothing);
    expect(find.byType(StickerPage), findsOneWidget);
    await tester.tap(find.text('RECENT'));
    await tester.pump();
    expect(textController.text, 'ARECENTB');

    await tester.tap(find.byKey(const Key('sticker-tab-2')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('sticker-one')));
    await tester.pump();
    expect(sentStickerId, 'one');
  });

  testWidgets('shows upstream sticker detail and switches album state', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(800, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    bool? added;
    await tester.pumpWidget(
      _testApp(
        Center(
          child: SizedBox(
            width: 480,
            child: StickerDetailPage(
              detail: const rust.StickerDetailItem(
                sticker: _rustSticker,
                album: _rustAlbum,
                albumStickers: [_rustSticker],
                isPersonal: false,
              ),
              onAlbumAddedChanged: (value) async => added = value,
            ),
          ),
        ),
      ),
    );

    expect(find.text('Album'), findsOneWidget);
    expect(find.text('Add Stickers'), findsOneWidget);
    await tester.tap(find.text('Add Stickers'));
    await tester.pump();
    expect(added, isTrue);
    expect(find.text('Remove Stickers'), findsOneWidget);
  });

  testWidgets('keeps favorite add tile and only closes after a sent sticker', (
    tester,
  ) async {
    var addCalls = 0;
    var sentCalls = 0;
    var sendSucceeds = false;
    await tester.pumpWidget(
      _testApp(
        DefaultTabController(
          length: 4,
          initialIndex: 3,
          child: Builder(
            builder: (context) => StickerPage(
              tabController: DefaultTabController.of(context),
              tabLength: 4,
              presetStickerGroups: const [
                PresetStickerGroup.store,
                PresetStickerGroup.emoji,
                PresetStickerGroup.recent,
                PresetStickerGroup.favorite,
              ],
              stickerAlbums: const [],
              albumStickers: const {},
              recentStickers: const [],
              personalStickers: const [_sticker],
              textEditingController: TextEditingController(),
              recentUsedEmoji: const [],
              onEmojiUsed: (_) {},
              onStickerSelected: (_) async => sendSucceeds,
              onRemoveSticker: (_) async {},
              onAddSticker: () async => addCalls++,
              onStickerSent: () => sentCalls++,
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key('add-sticker')));
    expect(addCalls, 1);
    await tester.tap(find.byKey(const Key('sticker-one')));
    expect(sentCalls, 0);
    sendSucceeds = true;
    await tester.tap(find.byKey(const Key('sticker-one')));
    expect(sentCalls, 1);
  });
}

Widget _testApp(Widget child) => MaterialApp(
  theme: buildMixinTheme(Brightness.light),
  localizationsDelegates: const [
    AppLocalizations.delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
  ],
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: Center(child: child)),
);

const _sticker = StickerData(
  stickerId: 'one',
  name: 'One',
  assetUrl: '',
  assetWidth: 128,
  assetHeight: 128,
  assetType: 'png',
);

const _rustSticker = rust.StickerItem(
  stickerId: 'one',
  name: 'One',
  assetUrl: '',
  assetWidth: 128,
  assetHeight: 128,
  assetType: 'png',
  createdAtMillis: 0,
);

const _rustAlbum = rust.StickerAlbumItem(
  albumId: 'album',
  name: 'Album',
  iconUrl: '',
  category: 'SYSTEM',
  description: '',
  added: false,
  isVerified: false,
);
