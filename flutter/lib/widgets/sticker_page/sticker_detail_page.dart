import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_item.dart';

Future<void> showStickerDetailPage(
  BuildContext context, {
  required rust.AccountHandle account,
  required String stickerId,
  Future<void> Function()? onAlbumChanged,
}) async {
  final detail = account.sticker().stickerDetail(stickerId: stickerId);
  await showDialog<void>(
    context: context,
    builder: (context) => Dialog(
      backgroundColor: context.theme.popUp,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: FutureBuilder(
          future: detail,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              return _StickerDetailError(error: snapshot.error!);
            }
            final value = snapshot.data;
            if (value == null) return const _StickerDetailLoading();
            return StickerDetailPage(
              detail: value,
              onAlbumAddedChanged: (added) async {
                final album = value.album;
                if (album == null) return;
                await account.sticker().setStickerAlbumAdded(
                  albumId: album.albumId,
                  added: added,
                );
                await onAlbumChanged?.call();
              },
            );
          },
        ),
      ),
    ),
  );
}

class _StickerDetailLoading extends StatelessWidget {
  const _StickerDetailLoading();

  @override
  Widget build(BuildContext context) => const SizedBox(
    width: 480,
    height: 480,
    child: Center(child: CircularProgressIndicator()),
  );
}

class _StickerDetailError extends StatelessWidget {
  const _StickerDetailError({required this.error});

  final Object error;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 480,
    height: 240,
    child: Column(
      children: [
        Align(
          alignment: Alignment.centerRight,
          child: IconButton(
            onPressed: () => Navigator.maybeOf(context)?.pop(),
            icon: const Icon(Icons.close),
          ),
        ),
        Expanded(child: Center(child: Text(error.toString()))),
      ],
    ),
  );
}

class StickerDetailPage extends HookWidget {
  const StickerDetailPage({
    required this.detail,
    required this.onAlbumAddedChanged,
    super.key,
  });

  final rust.StickerDetailItem detail;
  final Future<void> Function(bool added) onAlbumAddedChanged;

  @override
  Widget build(BuildContext context) {
    final sticker = useState<rust.StickerItem>(detail.sticker);
    final album = detail.album;
    final stickers = detail.albumStickers;
    final albumAdded = useState(album?.added ?? false);

    return DefaultTabController(
      length: stickers.length,
      child: HookBuilder(
        builder: (context) {
          final tabController = DefaultTabController.of(context);
          useEffect(() {
            void listener() {
              sticker.value = stickers[tabController.index];
            }

            tabController.addListener(listener);

            return () {
              tabController.removeListener(listener);
            };
          }, [tabController]);

          return AnimatedSize(
            duration: const Duration(milliseconds: 200),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  height: 56,
                  child: Align(
                    alignment: Alignment.centerRight,
                    child: IconButton(
                      onPressed: () => Navigator.maybeOf(
                        context,
                        rootNavigator: true,
                      )?.pop(),
                      icon: Icon(Icons.close, color: context.theme.icon),
                    ),
                  ),
                ),
                AspectRatio(
                  aspectRatio: 1,
                  child: Container(
                    margin: const EdgeInsets.all(56).copyWith(top: 0),
                    color: context.theme.background,
                    alignment: Alignment.center,
                    child: SizedBox(
                      height: 256,
                      width: 256,
                      child: sticker.value.assetUrl.isNotEmpty
                          ? StickerItem(
                              stickerId: sticker.value.stickerId,
                              assetUrl: sticker.value.assetUrl,
                              assetType: sticker.value.assetType,
                            )
                          : const SizedBox(),
                    ),
                  ),
                ),
                if (album != null)
                  Padding(
                    padding: const EdgeInsets.only(
                      right: 24,
                      left: 24,
                      bottom: 8,
                    ),
                    child: Row(
                      children: [
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
                        FilledButton(
                          style: FilledButton.styleFrom(
                            backgroundColor: albumAdded.value
                                ? context.theme.red
                                : context.theme.accent,
                          ),
                          onPressed: () async {
                            final added = !albumAdded.value;
                            await onAlbumAddedChanged(added);
                            albumAdded.value = added;
                          },
                          child: Text(
                            albumAdded.value
                                ? context.l10n.removeStickers
                                : context.l10n.addStickers,
                          ),
                        ),
                      ],
                    ),
                  ),
                if (album != null && stickers.isNotEmpty)
                  TabBar(
                    isScrollable: true,
                    overlayColor: WidgetStateProperty.all(Colors.transparent),
                    indicator: BoxDecoration(
                      color: context.dynamicColor(
                        const Color.fromRGBO(229, 231, 235, 1),
                        darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
                      ),
                      borderRadius: const BorderRadius.all(Radius.circular(8)),
                    ),
                    labelPadding: EdgeInsets.zero,
                    indicatorPadding: const EdgeInsets.all(6),
                    dividerHeight: 0,
                    tabs: stickers
                        .map(
                          (e) => Padding(
                            padding: const EdgeInsets.all(16),
                            child: StickerItem(
                              stickerId: e.stickerId,
                              assetUrl: e.assetUrl,
                              assetType: e.assetType,
                              width: 64,
                              height: 64,
                            ),
                          ),
                        )
                        .toList(),
                  ),
              ],
            ),
          );
        },
      ),
    );
  }
}
