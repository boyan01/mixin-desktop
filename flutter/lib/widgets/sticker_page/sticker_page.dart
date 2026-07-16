import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/svg.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../../constants/assets.dart';
import '../../l10n/l10n.dart';
import '../../theme.dart';
import '../automatic_keep_alive_client_widget.dart';
import '../hover_overlay.dart';
import '../interactive_decorated_box.dart';
import 'emoji_page.dart';
import 'sticker_data.dart';
import 'sticker_item.dart';

enum PresetStickerGroup { store, emoji, recent, favorite, gif }

class StickerPage extends StatelessWidget {
  const StickerPage({
    required this.tabLength,
    required this.tabController,
    required this.presetStickerGroups,
    required this.stickerAlbums,
    required this.albumStickers,
    required this.recentStickers,
    required this.personalStickers,
    required this.textEditingController,
    required this.recentUsedEmoji,
    required this.onEmojiUsed,
    required this.onStickerSelected,
    required this.onRemoveSticker,
    this.onAddSticker,
    this.onOpenStore,
    this.onStickerSent,
    super.key,
  });

  final TabController tabController;
  final int tabLength;
  final List<PresetStickerGroup> presetStickerGroups;
  final List<StickerAlbumData> stickerAlbums;
  final Map<String, List<StickerData>> albumStickers;
  final List<StickerData> recentStickers;
  final List<StickerData> personalStickers;
  final TextEditingController textEditingController;
  final List<String> recentUsedEmoji;
  final ValueChanged<String> onEmojiUsed;
  final Future<bool> Function(StickerData) onStickerSelected;
  final Future<void> Function(StickerData) onRemoveSticker;
  final Future<void> Function()? onAddSticker;
  final Future<void> Function()? onOpenStore;
  final VoidCallback? onStickerSent;

  @override
  Widget build(BuildContext context) => Material(
    color: Colors.transparent,
    elevation: 5,
    borderRadius: const BorderRadius.all(Radius.circular(11)),
    child: Container(
      width: 464,
      height: 407,
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(11)),
        color: context.dynamicColor(
          const Color.fromRGBO(255, 255, 255, 1),
          darkColor: const Color.fromRGBO(62, 65, 72, 1),
        ),
      ),
      child: ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(11)),
        child: Column(
          children: [
            Expanded(
              child: TabBarView(
                controller: tabController,
                physics: const NeverScrollableScrollPhysics(),
                children: List.generate(tabLength, (index) {
                  if (index < presetStickerGroups.length) {
                    final preset = presetStickerGroups[index];
                    switch (preset) {
                      case PresetStickerGroup.store:
                        return _StickerStoreEmptyPage();
                      case PresetStickerGroup.emoji:
                        return AutomaticKeepAliveClientWidget(
                          child: EmojiPage(
                            textEditingController: textEditingController,
                            recentUsedEmoji: recentUsedEmoji,
                            onEmojiUsed: onEmojiUsed,
                          ),
                        );
                      case PresetStickerGroup.recent:
                        return _StickerAlbumPage(
                          stickers: recentStickers,
                          onStickerSelected: onStickerSelected,
                          onStickerSent: onStickerSent,
                        );
                      case PresetStickerGroup.favorite:
                        return _StickerAlbumPage(
                          stickers: personalStickers,
                          delete: onRemoveSticker,
                          onAddSticker: onAddSticker,
                          onStickerSelected: onStickerSelected,
                          onStickerSent: onStickerSent,
                        );
                      case PresetStickerGroup.gif:
                        return const SizedBox();
                    }
                  }
                  return _StickerAlbumPage(
                    onStickerSent: onStickerSent,
                    stickers:
                        albumStickers[stickerAlbums[index -
                                presetStickerGroups.length]
                            .albumId] ??
                        const [],
                    onStickerSelected: onStickerSelected,
                  );
                }),
              ),
            ),
            _StickerAlbumBar(
              tabLength: tabLength,
              tabController: tabController,
              presetStickerGroups: presetStickerGroups,
              stickerAlbums: stickerAlbums,
              onOpenStore: onOpenStore,
            ),
          ],
        ),
      ),
    ),
  );
}

class _StickerAlbumPage extends HookWidget {
  const _StickerAlbumPage({
    required this.stickers,
    required this.onStickerSelected,
    this.delete,
    this.onAddSticker,
    this.onStickerSent,
  });

  final List<StickerData> stickers;
  final Future<bool> Function(StickerData) onStickerSelected;
  final Future<void> Function(StickerData)? delete;
  final Future<void> Function()? onAddSticker;
  final VoidCallback? onStickerSent;

  @override
  Widget build(BuildContext context) {
    final controller = useMemoized(ScrollController.new);
    return GridView.builder(
      controller: controller,
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 20),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 4,
        mainAxisSpacing: 8,
        crossAxisSpacing: 8,
      ),
      itemCount: stickers.length + (onAddSticker == null ? 0 : 1),
      itemBuilder: (context, index) {
        if (onAddSticker != null && index == 0) {
          return _AddStickerWidget(onTap: onAddSticker!);
        }
        final stickerIndex = onAddSticker == null ? index : index - 1;
        return _StickerAlbumPageItem(
          key: Key('sticker-${stickers[stickerIndex].stickerId}'),
          sticker: stickers[stickerIndex],
          onStickerSelected: onStickerSelected,
          delete: delete,
          onStickerSent: onStickerSent,
        );
      },
    );
  }
}

class _AddStickerWidget extends StatelessWidget {
  const _AddStickerWidget({required this.onTap});

  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox(
    key: const Key('add-sticker'),
    hoveringDecoration: BoxDecoration(
      color: context.dynamicColor(
        const Color.fromRGBO(229, 231, 235, 1),
        darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
      ),
      borderRadius: const BorderRadius.all(Radius.circular(8)),
    ),
    onTap: onTap,
    child: Center(
      child: SvgPicture.asset(
        MixinAssets.addSticker,
        width: 78,
        height: 78,
        colorFilter: ColorFilter.mode(
          context.theme.secondaryText,
          BlendMode.srcIn,
        ),
      ),
    ),
  );
}

class _StickerStoreEmptyPage extends StatelessWidget {
  @override
  Widget build(BuildContext context) => Center(
    child: Text(
      context.l10n.stickerStore,
      style: TextStyle(color: context.theme.secondaryText, fontSize: 18),
    ),
  );
}

class _StickerAlbumPageItem extends StatelessWidget {
  const _StickerAlbumPageItem({
    required this.sticker,
    required this.onStickerSelected,
    this.delete,
    this.onStickerSent,
    super.key,
  });

  final StickerData sticker;
  final Future<bool> Function(StickerData) onStickerSelected;
  final Future<void> Function(StickerData)? delete;
  final VoidCallback? onStickerSent;

  @override
  Widget build(BuildContext context) {
    Widget widget = InteractiveDecoratedBox(
      onTap: () async {
        if (await onStickerSelected(sticker)) onStickerSent?.call();
      },
      hoveringDecoration: BoxDecoration(
        color: context.dynamicColor(
          const Color.fromRGBO(229, 231, 235, 1),
          darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
        ),
        borderRadius: const BorderRadius.all(Radius.circular(8)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: RepaintBoundary(
          child: Builder(
            builder: (context) => StickerItem(
              stickerId: sticker.stickerId,
              assetUrl: sticker.assetUrl,
              assetType: sticker.assetType,
            ),
          ),
        ),
      ),
    );
    if (delete != null) {
      widget = ContextMenuWidget(
        menuProvider: (request) => Menu(
          children: [
            MenuAction(
              title: context.l10n.delete,
              image: MenuImage.icon(Icons.delete_outline),
              callback: () => delete?.call(sticker),
            ),
          ],
        ),
        child: widget,
      );
    }

    return widget;
  }
}

class _StickerAlbumBar extends HookWidget {
  const _StickerAlbumBar({
    required this.tabLength,
    required this.tabController,
    required this.presetStickerGroups,
    required this.stickerAlbums,
    this.onOpenStore,
  });

  final int tabLength;
  final TabController tabController;
  final List<PresetStickerGroup> presetStickerGroups;
  final List<StickerAlbumData> stickerAlbums;
  final Future<void> Function()? onOpenStore;

  @override
  Widget build(BuildContext context) {
    final validIndexRef = useRef<int?>(tabController.index);

    final setPreviousIndex = useCallback(() {
      final previousIndex = tabController.previousIndex;
      if (previousIndex != 0) {
        validIndexRef.value = previousIndex;
      }

      if (validIndexRef.value != 0) {
        // Sometimes tabController.index is validIndex, but TabBar.currentIndex is 0, they are not synchronized, so we need reset tabController.index to 0, then set to validIndex.
        tabController
          ..index = 0
          ..index = validIndexRef.value!;
      }
    }, []);

    useEffect(() {
      Future<void> listener() async {
        if (tabController.index != 0) return;

        HoverOverlay.forceHidden(context);
        setPreviousIndex();
        await onOpenStore?.call();
        // When the dialog is closed, the scroll status is idle most of the time, reset tabController.index to validIndex.
        setPreviousIndex();
      }

      tabController.addListener(listener);
      return () {
        tabController.removeListener(listener);
      };
    }, [tabController]);
    return Container(
      width: double.infinity,
      height: 50,
      color: context.dynamicColor(
        const Color.fromRGBO(0, 0, 0, 0.05),
        darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
      ),
      child: TabBar(
        controller: tabController,
        isScrollable: true,
        tabAlignment: TabAlignment.start,
        indicator: BoxDecoration(
          color: context.dynamicColor(
            const Color.fromRGBO(229, 231, 235, 1),
            darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
          ),
          borderRadius: const BorderRadius.all(Radius.circular(8)),
        ),
        labelPadding: EdgeInsets.zero,
        indicatorPadding: const EdgeInsets.all(5),
        dividerColor: Colors.transparent,
        tabs: List.generate(
          tabLength,
          (index) => _StickerAlbumBarItem(
            key: Key('sticker-tab-$index'),
            index: index,
            presetStickerGroups: presetStickerGroups,
            stickerAlbums: stickerAlbums,
          ),
        ),
      ),
    );
  }
}

class _StickerAlbumBarItem extends StatelessWidget {
  const _StickerAlbumBarItem({
    required this.index,
    required this.presetStickerGroups,
    required this.stickerAlbums,
    super.key,
  });

  final int index;
  final List<PresetStickerGroup> presetStickerGroups;
  final List<StickerAlbumData> stickerAlbums;

  @override
  Widget build(BuildContext context) => SizedBox.fromSize(
    size: const Size.square(48),
    child: Padding(
      padding: const EdgeInsets.all(4),
      child: _StickerGroupIconHoverContainer(
        child: Center(
          child: Center(
            child: Builder(
              builder: (context) {
                final presetStickerAlbum = {
                  PresetStickerGroup.store: MixinAssets.stickerStore,
                  PresetStickerGroup.emoji: MixinAssets.emojiSticker,
                  PresetStickerGroup.recent: MixinAssets.recentSticker,
                  PresetStickerGroup.favorite: MixinAssets.personalSticker,
                  PresetStickerGroup.gif: MixinAssets.sticker,
                };

                if (index < presetStickerGroups.length) {
                  return SvgPicture.asset(
                    presetStickerAlbum[presetStickerGroups[index]]!,
                    colorFilter: index != 0
                        ? ColorFilter.mode(
                            context.theme.secondaryText,
                            BlendMode.srcIn,
                          )
                        : null,
                    width: 24,
                    height: 24,
                  );
                }

                return StickerGroupIcon(
                  iconUrl:
                      stickerAlbums[index - presetStickerGroups.length].iconUrl,
                  size: 28,
                );
              },
            ),
          ),
        ),
      ),
    ),
  );
}

class _StickerGroupIconHoverContainer extends HookWidget {
  const _StickerGroupIconHoverContainer({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final isHovering = useState(false);
    return MouseRegion(
      onEnter: (event) {
        isHovering.value = true;
      },
      onExit: (event) {
        isHovering.value = false;
      },
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: isHovering.value
              ? context.dynamicColor(
                  const Color.fromRGBO(229, 231, 235, 1),
                  darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
                )
              : null,
          borderRadius: const BorderRadius.all(Radius.circular(8)),
        ),
        child: child,
      ),
    );
  }
}
