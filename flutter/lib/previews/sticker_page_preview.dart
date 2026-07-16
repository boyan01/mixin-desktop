import 'package:flutter/material.dart';
import 'package:flutter/widget_previews.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_data.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_page.dart';

@Preview(name: 'Sticker Page', group: 'Chat', size: Size(480, 430))
Widget stickerPagePreview() => MaterialApp(
  theme: buildMixinTheme(Brightness.light),
  home: Scaffold(
    backgroundColor: lightMixinColors.chatBackground,
    body: Center(
      child: DefaultTabController(
        length: 5,
        initialIndex: 1,
        child: Builder(
          builder: (context) => StickerPage(
            tabController: DefaultTabController.of(context),
            tabLength: 5,
            presetStickerGroups: const [
              PresetStickerGroup.store,
              PresetStickerGroup.emoji,
              PresetStickerGroup.recent,
              PresetStickerGroup.favorite,
            ],
            stickerAlbums: const [_album],
            albumStickers: const {'album': _stickers},
            recentStickers: _stickers,
            personalStickers: _stickers,
            textEditingController: TextEditingController(),
            recentUsedEmoji: const ['😀', '👍', '🎉'],
            onEmojiUsed: _ignoreEmoji,
            onStickerSelected: _ignoreSticker,
            onRemoveSticker: _ignoreSticker,
          ),
        ),
      ),
    ),
  ),
);

const _album = StickerAlbumData(
  albumId: 'album',
  name: 'Mixin Friends',
  iconUrl: '',
  added: true,
);

const _stickers = [
  StickerData(
    stickerId: 'one',
    name: 'One',
    assetUrl: '',
    assetWidth: 128,
    assetHeight: 128,
    assetType: 'png',
  ),
  StickerData(
    stickerId: 'two',
    name: 'Two',
    assetUrl: '',
    assetWidth: 128,
    assetHeight: 128,
    assetType: 'webp',
  ),
];

void _ignoreEmoji(String _) {}
Future<bool> _ignoreSticker(StickerData _) async => true;
