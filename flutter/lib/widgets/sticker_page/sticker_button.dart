import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixin_desktop_ui/controllers/sticker_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/widgets/hover_overlay.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/add_sticker_dialog.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_page.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_store.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_data.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/giphy_page.dart';

class StickerButton extends HookWidget {
  const StickerButton({
    required this.textEditingController,
    required this.controller,
    required this.child,
    required this.onStickerSelected,
    required this.onStickerSent,
    required this.onEmojiUsed,
    required this.onGifSelected,
    super.key,
  });

  final TextEditingController textEditingController;
  final StickerController controller;
  final Widget child;
  final Future<bool> Function(String stickerId) onStickerSelected;
  final VoidCallback onStickerSent;
  final ValueChanged<String> onEmojiUsed;
  final GiphySelected onGifSelected;

  @override
  Widget build(BuildContext context) {
    final key = useMemoized(GlobalKey.new);
    final loadedForCurrentOpen = useRef(false);
    useListenable(controller);
    final stickerAlbums = controller.albums
        .map(
          (album) => StickerAlbumData(
            albumId: album.albumId,
            name: album.name,
            iconUrl: album.iconUrl,
            added: album.added,
          ),
        )
        .toList(growable: false);
    final recentStickers = controller.recentStickers
        .map(_stickerData)
        .toList(growable: false);
    final personalStickers = controller.personalStickers
        .map(_stickerData)
        .toList(growable: false);
    final albumStickers = controller.albumStickers.map(
      (albumId, stickers) =>
          MapEntry(albumId, stickers.map(_stickerData).toList(growable: false)),
    );

    final presetStickerGroups = useMemoized(
      () => [
        PresetStickerGroup.store,
        PresetStickerGroup.emoji,
        PresetStickerGroup.recent,
        PresetStickerGroup.favorite,
        if (giphyApiKey.isNotEmpty) PresetStickerGroup.gif,
      ],
    );

    final tabLength = stickerAlbums.length + presetStickerGroups.length;

    return DefaultTabController(
      length: tabLength,
      initialIndex: 1,
      child: HoverOverlay(
        key: key,
        delayDuration: const Duration(milliseconds: 50),
        duration: const Duration(milliseconds: 200),
        closeDuration: const Duration(milliseconds: 200),
        closeWaitDuration: const Duration(milliseconds: 300),
        inCurve: Curves.easeOut,
        outCurve: Curves.easeOut,
        portalBuilder: (context, progress, _, child) {
          if (progress == 0) {
            loadedForCurrentOpen.value = false;
          } else if (!loadedForCurrentOpen.value) {
            loadedForCurrentOpen.value = true;
            scheduleMicrotask(controller.load);
          }

          final renderBox =
              key.currentContext?.findRenderObject() as RenderBox?;
          final offset = renderBox?.localToGlobal(Offset.zero);

          if (renderBox == null || offset == null || !renderBox.hasSize) {
            return const SizedBox();
          }

          final size = renderBox.size;
          return Opacity(
            opacity: progress,
            child: CustomSingleChildLayout(
              delegate: _StickerPagePositionedLayoutDelegate(
                position: offset + Offset(size.width / 2, 0),
              ),
              child: child,
            ),
          );
        },
        portal: Padding(
          padding: const EdgeInsets.all(8),
          child: Builder(
            builder: (context) => StickerPage(
              tabController: DefaultTabController.of(context),
              tabLength: tabLength,
              presetStickerGroups: presetStickerGroups,
              stickerAlbums: stickerAlbums,
              albumStickers: albumStickers,
              recentStickers: recentStickers,
              personalStickers: personalStickers,
              textEditingController: textEditingController,
              recentUsedEmoji: controller.recentEmojis,
              onEmojiUsed: (emoji) {
                unawaited(controller.recordEmoji(emoji));
                onEmojiUsed(emoji);
              },
              onStickerSelected: (sticker) =>
                  onStickerSelected(sticker.stickerId),
              onRemoveSticker: (sticker) async {
                final source = controller.personalStickers.firstWhere(
                  (item) => item.stickerId == sticker.stickerId,
                );
                await controller.removeSticker(source);
              },
              onGifSelected: onGifSelected,
              hasNewAlbum: controller.hasNewAlbum,
              onAddSticker: () => pickAndShowAddStickerDialog(
                context,
                onSave: controller.addStickerFromPath,
              ),
              onOpenStore: () async {
                await controller.markStoreViewed();
                if (!context.mounted) return;
                await showStickerStorePageDialog(
                  context,
                  controller: controller,
                );
              },
              onStickerSent: () {
                HoverOverlay.forceHidden(context);
                onStickerSent();
              },
            ),
          ),
        ),
        child: child,
      ),
    );
  }
}

StickerData _stickerData(rust.StickerItem sticker) => StickerData(
  stickerId: sticker.stickerId,
  albumId: sticker.albumId,
  name: sticker.name,
  assetUrl: sticker.assetUrl,
  assetWidth: sticker.assetWidth,
  assetHeight: sticker.assetHeight,
  assetType: sticker.assetType,
);

class _StickerPagePositionedLayoutDelegate extends SingleChildLayoutDelegate {
  _StickerPagePositionedLayoutDelegate({required this.position});

  final Offset position;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) =>
      BoxConstraints.loose(constraints.biggest);

  @override
  Offset getPositionForChild(Size size, Size childSize) => Offset(
    (position.dx - childSize.width / 2).clamp(0, size.width),
    position.dy - childSize.height,
  );

  @override
  bool shouldRelayout(
    covariant _StickerPagePositionedLayoutDelegate oldDelegate,
  ) => position != oldDelegate.position;
}
