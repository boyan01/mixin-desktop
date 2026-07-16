import 'dart:math' as math;

import 'package:emojis/emoji.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/svg.dart';

import '../../l10n/l10n.dart';
import '../../theme.dart';
import '../../utils/emoji.dart';
import '../interactive_decorated_box.dart';

const emojiGroups = [
  [EmojiGroup.smileysEmotion, EmojiGroup.peopleBody],
  [EmojiGroup.animalsNature],
  [EmojiGroup.foodDrink],
  [EmojiGroup.travelPlaces],
  [EmojiGroup.activities],
  [EmojiGroup.objects],
  [EmojiGroup.symbols],
  [EmojiGroup.flags],
];

class EmojiPage extends StatelessWidget {
  const EmojiPage({
    required this.textEditingController,
    required this.recentUsedEmoji,
    required this.onEmojiUsed,
    super.key,
  });

  final TextEditingController textEditingController;
  final List<String> recentUsedEmoji;
  final ValueChanged<String> onEmojiUsed;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => _EmojiPageBody(
      layoutWidth: constraints.maxWidth,
      textEditingController: textEditingController,
      recentUsedEmoji: recentUsedEmoji,
      onEmojiUsed: onEmojiUsed,
    ),
  );
}

class _EmojiPageBody extends HookWidget {
  const _EmojiPageBody({
    required this.layoutWidth,
    required this.textEditingController,
    required this.recentUsedEmoji,
    required this.onEmojiUsed,
  });

  final double layoutWidth;
  final TextEditingController textEditingController;
  final List<String> recentUsedEmoji;
  final ValueChanged<String> onEmojiUsed;

  @override
  Widget build(BuildContext context) {
    const emojiGroupIcon = [
      'assets/images/emoji_recent.svg',
      'assets/images/emoji_face.svg',
      'assets/images/emoji_animal.svg',
      'assets/images/emoji_food.svg',
      'assets/images/emoji_travel.svg',
      'assets/images/emoji_sports.svg',
      'assets/images/emoji_objects.svg',
      'assets/images/emoji_symbol.svg',
      'assets/images/emoji_flags.svg',
    ];

    final offset = useState(0.0);

    final emojiLineStride = useRef(8);

    useEffect(() {
      for (var stride = 10; stride >= 8; stride--) {
        final emojiItemSize = (layoutWidth - 14 * 2) / stride;
        if (emojiItemSize >= 40) {
          emojiLineStride.value = stride;
          break;
        }
      }
      return null;
    }, [layoutWidth]);

    final groupedEmojis = useMemoized(
      () => [
        recentUsedEmoji,
        ...emojiGroups.map(
          (group) => group
              .expand(Emoji.byGroup)
              .where((e) => !e.modifiable)
              .map((emoji) => emoji.char)
              .toList(),
        ),
      ],
      [recentUsedEmoji],
    );

    final groupOffset = useMemoized(() {
      final array = List<double>.filled(groupedEmojis.length, 0);
      for (var i = 1; i < groupedEmojis.length; i++) {
        final emojiLineCount =
            (groupedEmojis[i - 1].length / emojiLineStride.value).ceil();
        final emojiItemSize = (layoutWidth - 14 * 2) / emojiLineStride.value;
        final headerHeight = i == 1 ? 0 : 40;
        array[i] = array[i - 1] + emojiLineCount * emojiItemSize + headerHeight;
      }
      return array;
    }, [emojiLineStride.value, layoutWidth, groupedEmojis]);

    final selectedIndex = useMemoized(() {
      for (var i = groupOffset.length - 1; i >= 0; i--) {
        if (groupOffset[i] <= offset.value) {
          return i;
        }
      }
      return 0;
    }, [offset.value]);

    final emojiOffsetController = useStreamController<double>();
    final emojiOffsetStream = useMemoized(() => emojiOffsetController.stream);

    return Column(
      children: [
        _EmojiGroupHeader(
          selectedIndex: selectedIndex,
          icons: emojiGroupIcon,
          onTap: (index) {
            offset.value = groupOffset[index];
            emojiOffsetController.add(groupOffset[index]);
          },
        ),
        Divider(color: context.theme.divider, height: 1),
        const SizedBox(height: 8),
        Expanded(
          child: _AllEmojisPage(
            initialOffset: offset.value,
            offsetStream: emojiOffsetStream,
            emojiLineStride: emojiLineStride.value,
            groupedEmojis: groupedEmojis,
            onScroll: (value) => offset.value = value,
            textEditingController: textEditingController,
            onEmojiUsed: onEmojiUsed,
          ),
        ),
      ],
    );
  }
}

class _EmojiGroupHeader extends HookWidget {
  const _EmojiGroupHeader({
    required this.icons,
    required this.onTap,
    required this.selectedIndex,
  });

  final List<String> icons;
  final void Function(int index) onTap;
  final int selectedIndex;

  @override
  Widget build(BuildContext context) {
    final tabController = useTabController(initialLength: icons.length);
    useEffect(() {
      tabController.index = selectedIndex;
      return null;
    }, [selectedIndex]);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: SizedBox(
        width: double.infinity,
        height: 48,
        child: TabBar(
          controller: tabController,
          isScrollable: true,
          labelPadding: EdgeInsets.zero,
          indicator: const BoxDecoration(color: Colors.transparent),
          dividerColor: Colors.transparent,
          tabAlignment: TabAlignment.start,
          tabs: List.generate(
            icons.length,
            (index) => Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              child: _EmojiGroupIcon(
                icon: icons[index],
                onTap: () => onTap(index),
                index: index,
                selectedIndex: selectedIndex,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _EmojiGroupIcon extends StatelessWidget {
  const _EmojiGroupIcon({
    required this.index,
    required this.onTap,
    required this.icon,
    required this.selectedIndex,
  });

  final int index;
  final VoidCallback onTap;
  final String icon;
  final int selectedIndex;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox(
    onTap: onTap,
    hoveringDecoration: BoxDecoration(
      color: context.theme.sidebarSelected,
      borderRadius: const BorderRadius.all(Radius.circular(8)),
    ),
    child: Padding(
      padding: const EdgeInsets.all(4),
      child: SvgPicture.asset(
        icon,
        width: 24,
        height: 24,
        colorFilter: ColorFilter.mode(
          selectedIndex == index
              ? context.theme.accent
              : context.theme.secondaryText,
          BlendMode.srcIn,
        ),
      ),
    ),
  );
}

class _AllEmojisPage extends HookWidget {
  const _AllEmojisPage({
    required this.initialOffset,
    required this.offsetStream,
    required this.emojiLineStride,
    required this.groupedEmojis,
    required this.onScroll,
    required this.textEditingController,
    required this.onEmojiUsed,
  });

  final double initialOffset;
  final Stream<double> offsetStream;
  final int emojiLineStride;
  final List<List<String>> groupedEmojis;
  final ValueChanged<double> onScroll;
  final TextEditingController textEditingController;
  final ValueChanged<String> onEmojiUsed;

  @override
  Widget build(BuildContext context) {
    final controller = useMemoized(
      () => ScrollController(initialScrollOffset: initialOffset),
    );

    final groupTitles = [
      context.l10n.smileysAndPeople,
      context.l10n.animalsAndNature,
      context.l10n.foodAndDrink,
      context.l10n.travelAndPlaces,
      context.l10n.activity,
      context.l10n.objects,
      context.l10n.symbols,
      context.l10n.flags,
    ];

    useEffect(() {
      void scrollListener() {
        onScroll(controller.offset);
      }

      controller.addListener(scrollListener);
      return () {
        controller.removeListener(scrollListener);
      };
    }, [controller]);

    useEffect(() => offsetStream.listen(controller.jumpTo).cancel, [
      offsetStream,
    ]);

    return CustomScrollView(
      controller: controller,
      slivers: [
        for (var i = 0; i < groupedEmojis.length; i++) ...[
          if (i > 0)
            SliverToBoxAdapter(
              child: _EmojiGroupTitle(title: groupTitles[i - 1]),
            ),
          if (groupedEmojis[i].isNotEmpty)
            SliverPadding(
              padding: const EdgeInsets.symmetric(horizontal: 14),
              sliver: SliverGrid(
                delegate: SliverChildBuilderDelegate(
                  (context, index) => _EmojiItem(
                    emoji: groupedEmojis[i][index],
                    textEditingController: textEditingController,
                    onEmojiUsed: onEmojiUsed,
                  ),
                  childCount: groupedEmojis[i].length,
                ),
                gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                  crossAxisCount: emojiLineStride,
                ),
              ),
            ),
        ],
      ],
    );
  }
}

class _EmojiGroupTitle extends StatelessWidget {
  const _EmojiGroupTitle({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 40,
    child: Padding(
      padding: const EdgeInsets.only(left: 20, top: 12),
      child: Text(
        title,
        style: TextStyle(fontSize: 14, color: context.theme.secondaryText),
      ),
    ),
  );
}

class _EmojiItem extends StatelessWidget {
  const _EmojiItem({
    required this.emoji,
    required this.textEditingController,
    required this.onEmojiUsed,
  });

  final String emoji;
  final TextEditingController textEditingController;
  final ValueChanged<String> onEmojiUsed;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(2),
    child: InteractiveDecoratedBox(
      onTap: () {
        final textController = textEditingController;
        final textEditingValue = textController.value;
        final selection = textEditingValue.selection;
        if (!selection.isValid) {
          textController.text = '${textEditingValue.text}$emoji';
        } else {
          final int lastSelectionIndex = math.max(
            selection.baseOffset,
            selection.extentOffset,
          );
          final collapsedTextEditingValue = textEditingValue.copyWith(
            selection: TextSelection.collapsed(offset: lastSelectionIndex),
          );
          textController.value = collapsedTextEditingValue.replaced(
            selection,
            emoji,
          );
        }
        onEmojiUsed(emoji);
      },
      hoveringDecoration: BoxDecoration(
        color: context.dynamicColor(
          const Color.fromRGBO(229, 231, 235, 1),
          darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
        ),
        borderRadius: const BorderRadius.all(Radius.circular(8)),
      ),
      child: Center(
        child: Text(
          emoji,
          style: TextStyle(
            fontSize: 26,
            height: 1,
            fontFamily: kEmojiFontFamily,
            inherit: false,
          ),
          strutStyle: const StrutStyle(height: 1),
          textAlign: TextAlign.center,
        ),
      ),
    ),
  );
}
