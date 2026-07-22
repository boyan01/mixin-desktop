import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../constants/assets.dart';
import '../../controllers/sticker_controller.dart';
import '../../l10n/l10n.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../action_button.dart';
import '../buttons.dart';
import '../interactive_decorated_box.dart';
import '../mixin_dialog.dart';
import '../settings_widgets.dart';
import 'sticker_item.dart';

final _navigatorKey = GlobalKey<NavigatorState>();

Future<void> showStickerStorePageDialog(
  BuildContext context, {
  required StickerController controller,
}) async {
  unawaited(controller.loadStore());
  await showMixinDialog<void>(
    context: context,
    routeSettings: const RouteSettings(name: 'StickerStore'),
    child: ConstrainedBox(
      constraints: BoxConstraints.loose(const Size(480, 600)),
      child: Navigator(
        key: _navigatorKey,
        onGenerateRoute: (_) => MaterialPageRoute<void>(
          builder: (_) => _StickerStorePage(controller: controller),
        ),
      ),
    ),
  );
}

class _StickerStorePage extends StatelessWidget {
  const _StickerStorePage({required this.controller});

  final StickerController controller;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      _StoreHeader(
        title: context.l10n.stickerStore,
        leading: Center(
          child: ActionButton(
            name: MixinAssets.setting,
            color: context.theme.icon,
            onTap: () => Navigator.push<void>(
              context,
              MaterialPageRoute(
                builder: (_) => _StickerAlbumManagePage(controller: controller),
              ),
            ),
          ),
        ),
      ),
      Expanded(
        child: AnimatedBuilder(
          animation: controller,
          builder: (context, _) => ListView.builder(
            padding: const EdgeInsets.symmetric(vertical: 16),
            itemCount: controller.storeAlbums.length,
            itemBuilder: (context, index) {
              final album = controller.storeAlbums[index];
              return _StoreAlbumItem(
                album: album,
                stickers:
                    controller.storeAlbumStickers[album.albumId] ?? const [],
                onToggle: (added) =>
                    controller.setAlbumAdded(album.albumId, added),
              );
            },
          ),
        ),
      ),
    ],
  );
}

class _StoreAlbumItem extends StatelessWidget {
  const _StoreAlbumItem({
    required this.album,
    required this.stickers,
    required this.onToggle,
  });

  final rust.StickerAlbumItem album;
  final List<rust.StickerItem> stickers;
  final Future<void> Function(bool added) onToggle;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 104,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            album.name,
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: context.theme.text,
            ),
          ),
          Expanded(
            child: Row(
              children: [
                for (final sticker in stickers.take(4))
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: InteractiveDecoratedBox(
                      onTap: () => Navigator.push<void>(
                        context,
                        MaterialPageRoute(
                          builder: (_) => _StickerAlbumPage(
                            album: album,
                            stickers: stickers,
                            onToggle: onToggle,
                          ),
                        ),
                      ),
                      child: SizedBox.square(
                        dimension: 72,
                        child: StickerItem(
                          stickerId: sticker.stickerId,
                          assetUrl: sticker.assetUrl,
                          assetType: sticker.assetType,
                        ),
                      ),
                    ),
                  ),
                const Spacer(),
                AnimatedOpacity(
                  opacity: album.added ? 0.4 : 1,
                  duration: const Duration(milliseconds: 200),
                  child: MixinButton(
                    onTap: () => onToggle(!album.added),
                    child: Text(
                      album.added ? context.l10n.added : context.l10n.add,
                      style: const TextStyle(fontSize: 14),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class _StickerAlbumPage extends StatefulWidget {
  const _StickerAlbumPage({
    required this.album,
    required this.stickers,
    required this.onToggle,
  });

  final rust.StickerAlbumItem album;
  final List<rust.StickerItem> stickers;
  final Future<void> Function(bool added) onToggle;

  @override
  State<_StickerAlbumPage> createState() => _StickerAlbumPageState();
}

class _StickerAlbumPageState extends State<_StickerAlbumPage> {
  late bool _added = widget.album.added;

  Future<void> _toggle() async {
    final added = !_added;
    await widget.onToggle(added);
    if (mounted) setState(() => _added = added);
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      _StoreHeader(title: widget.album.name, showBack: true),
      Expanded(
        child: Stack(
          fit: StackFit.expand,
          children: [
            CustomScrollView(
              slivers: [
                SliverPadding(
                  padding: const EdgeInsets.symmetric(horizontal: 24),
                  sliver: SliverGrid(
                    gridDelegate:
                        const SliverGridDelegateWithFixedCrossAxisCount(
                          crossAxisCount: 4,
                          mainAxisSpacing: 10,
                          crossAxisSpacing: 8,
                        ),
                    delegate: SliverChildBuilderDelegate(
                      (context, index) => StickerItem(
                        stickerId: widget.stickers[index].stickerId,
                        assetUrl: widget.stickers[index].assetUrl,
                        assetType: widget.stickers[index].assetType,
                      ),
                      childCount: widget.stickers.length,
                    ),
                  ),
                ),
                const SliverPadding(padding: EdgeInsets.only(bottom: 112)),
              ],
            ),
            Positioned(
              left: 0,
              right: 0,
              bottom: 0,
              height: 93,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topCenter,
                    end: Alignment.bottomCenter,
                    colors: [
                      context.theme.popUp.withValues(alpha: 0),
                      context.theme.popUp.withValues(alpha: 0.36),
                      context.theme.popUp,
                    ],
                  ),
                ),
                child: Column(
                  children: [
                    const Spacer(),
                    MixinButton(
                      backgroundColor: _added
                          ? context.theme.red
                          : context.theme.accent,
                      onTap: _toggle,
                      child: Text(
                        _added
                            ? context.l10n.removeStickers
                            : context.l10n.addStickers,
                      ),
                    ),
                    const SizedBox(height: 40),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    ],
  );
}

class _StickerAlbumManagePage extends StatefulWidget {
  const _StickerAlbumManagePage({required this.controller});

  final StickerController controller;

  @override
  State<_StickerAlbumManagePage> createState() =>
      _StickerAlbumManagePageState();
}

class _StickerAlbumManagePageState extends State<_StickerAlbumManagePage> {
  late List<rust.StickerAlbumItem> _albums;
  final _scrollController = ScrollController();

  @override
  void initState() {
    super.initState();
    _albums = widget.controller.albums.toList();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      _StoreHeader(title: context.l10n.myStickers, showBack: true),
      Expanded(
        child: ReorderableList(
          controller: _scrollController,
          itemCount: _albums.length,
          onReorderItem: (oldIndex, newIndex) async {
            setState(() {
              final album = _albums.removeAt(oldIndex);
              _albums.insert(newIndex, album);
            });
            await widget.controller.setAlbumOrder(
              _albums.map((album) => album.albumId).toList(),
            );
          },
          itemBuilder: (context, index) {
            final album = _albums[index];
            return Listener(
              key: ValueKey(album.albumId),
              onPointerDown: (event) {
                if (event.buttons != kPrimaryButton) return;
                ReorderableList.maybeOf(context)?.startItemDragReorder(
                  index: index,
                  event: event,
                  recognizer: ImmediateMultiDragGestureRecognizer(
                    supportedDevices: {
                      PointerDeviceKind.touch,
                      PointerDeviceKind.mouse,
                    },
                  ),
                );
              },
              child: Container(
                margin: const EdgeInsets.symmetric(
                  vertical: 16,
                  horizontal: 24,
                ),
                height: 72,
                child: Row(
                  children: [
                    StickerGroupIcon(iconUrl: album.iconUrl, size: 72),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        album.name,
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    ActionButton(
                      name: MixinAssets.delete,
                      color: context.theme.secondaryText,
                      onTap: () async {
                        await widget.controller.setAlbumAdded(
                          album.albumId,
                          false,
                        );
                        if (!mounted) return;
                        setState(() => _albums.remove(album));
                      },
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    ],
  );
}

class _StoreHeader extends StatelessWidget {
  const _StoreHeader({
    required this.title,
    this.leading,
    this.showBack = false,
  });

  final String title;
  final Widget? leading;
  final bool showBack;

  @override
  Widget build(BuildContext context) => MixinAppBar(
    backgroundColor: Colors.transparent,
    title: Text(title),
    leading: leading ?? (showBack ? null : const SizedBox()),
    actions: [
      MixinCloseButton(
        onTap: () => Navigator.maybeOf(context, rootNavigator: true)?.pop(),
      ),
    ],
  );
}
