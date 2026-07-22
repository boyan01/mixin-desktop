import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../constants/assets.dart';
import '../l10n/l10n.dart';
import '../models/command_palette_item.dart';
import '../theme.dart';
import 'adaptive_selection_toolbar.dart';
import 'avatar_view.dart';
import 'buttons.dart';
import 'interactive_decorated_box.dart';

const _itemHeight = 72.0;
const _bottomPadding = 22.0;

Future<void> showCommandPalette({
  required BuildContext context,
  required Future<List<CommandPaletteItem>> Function(String keyword) search,
  required ValueChanged<CommandPaletteItem> onSelected,
}) => showGeneralDialog<void>(
  context: context,
  barrierDismissible: true,
  barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
  transitionDuration: const Duration(milliseconds: 80),
  pageBuilder: (dialogContext, _, _) => Center(
    child: Padding(
      padding:
          MediaQuery.viewInsetsOf(dialogContext) + const EdgeInsets.all(32),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 600),
        child: DecoratedBox(
          decoration: const BoxDecoration(
            borderRadius: BorderRadius.all(Radius.circular(11)),
            boxShadow: [
              BoxShadow(
                color: Color.fromRGBO(0, 0, 0, 0.15),
                offset: Offset(0, 8),
                blurRadius: 40,
              ),
              BoxShadow(
                color: Color.fromRGBO(0, 0, 0, 0.07),
                offset: Offset(0, 4),
                blurRadius: 16,
              ),
            ],
          ),
          child: Material(
            color: dialogContext.mixinTheme.popUp,
            borderRadius: const BorderRadius.all(Radius.circular(11)),
            clipBehavior: Clip.antiAlias,
            child: _CommandPalettePage(search: search, onSelected: onSelected),
          ),
        ),
      ),
    ),
  ),
);

class _CommandPalettePage extends StatefulWidget {
  const _CommandPalettePage({required this.search, required this.onSelected});

  final Future<List<CommandPaletteItem>> Function(String keyword) search;
  final ValueChanged<CommandPaletteItem> onSelected;

  @override
  State<_CommandPalettePage> createState() => _CommandPalettePageState();
}

class _CommandPalettePageState extends State<_CommandPalettePage> {
  final textController = TextEditingController();
  final focusNode = FocusNode();
  final scrollController = ScrollController();
  Timer? debounce;
  List<CommandPaletteItem> items = const [];
  int selectedIndex = 0;
  int revision = 0;

  @override
  void initState() {
    super.initState();
    textController.addListener(_queryChanged);
    _search();
  }

  @override
  void dispose() {
    debounce?.cancel();
    textController
      ..removeListener(_queryChanged)
      ..dispose();
    focusNode.dispose();
    scrollController.dispose();
    super.dispose();
  }

  void _queryChanged() {
    debounce?.cancel();
    debounce = Timer(const Duration(milliseconds: 100), _search);
    setState(() {});
  }

  Future<void> _search() async {
    final request = ++revision;
    final result = await widget.search(textController.text);
    if (!mounted || request != revision) return;
    setState(() {
      items = result;
      selectedIndex = 0;
    });
  }

  void _move(int delta) {
    if (items.isEmpty) return;
    final next = (selectedIndex + delta).clamp(0, items.length - 1);
    if (next == selectedIndex) return;
    setState(() => selectedIndex = next);
    _jumpToPosition();
  }

  void _jumpToPosition() {
    if (!scrollController.hasClients) return;
    final viewport = scrollController.position.viewportDimension;
    final contentHeight = items.length * _itemHeight + _bottomPadding;
    final maxExtent = contentHeight - viewport;
    final itemOffset = selectedIndex * _itemHeight;
    final itemEnd = itemOffset + _itemHeight;
    final current = scrollController.offset;
    if (itemOffset >= current && itemEnd <= current + viewport) return;
    final target = (itemOffset - (viewport - _itemHeight) / 2).clamp(
      0.0,
      maxExtent,
    );
    unawaited(
      scrollController.animateTo(
        target,
        duration: const Duration(milliseconds: 150),
        curve: Curves.easeInOut,
      ),
    );
  }

  void _select([int? index]) {
    if (items.isEmpty) return;
    final item = items[index ?? selectedIndex];
    Navigator.pop(context);
    widget.onSelected(item);
  }

  @override
  Widget build(BuildContext context) => Shortcuts(
    shortcuts: {
      const SingleActivator(LogicalKeyboardKey.arrowDown): const _MoveIntent(1),
      const SingleActivator(LogicalKeyboardKey.arrowUp): const _MoveIntent(-1),
      const SingleActivator(LogicalKeyboardKey.tab): const _MoveIntent(1),
      const SingleActivator(LogicalKeyboardKey.enter): const _SelectIntent(),
      if (defaultTargetPlatform == TargetPlatform.macOS) ...{
        const SingleActivator(LogicalKeyboardKey.keyN, control: true):
            const _MoveIntent(1),
        const SingleActivator(LogicalKeyboardKey.keyP, control: true):
            const _MoveIntent(-1),
      },
    },
    child: Actions(
      actions: {
        _MoveIntent: CallbackAction<_MoveIntent>(
          onInvoke: (intent) {
            _move(intent.delta);
            return null;
          },
        ),
        _SelectIntent: CallbackAction<_SelectIntent>(
          onInvoke: (_) {
            _select();
            return null;
          },
        ),
      },
      child: Focus(
        autofocus: true,
        child: ConstrainedBox(
          constraints: const BoxConstraints(
            minWidth: 400,
            maxWidth: 480,
            maxHeight: 600,
            minHeight: 400,
          ),
          child: Column(
            children: [
              Padding(
                padding: const EdgeInsets.only(
                  right: 20,
                  left: 20,
                  top: 20,
                  bottom: 10,
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: _PaletteSearchField(
                        controller: textController,
                        focusNode: focusNode,
                      ),
                    ),
                    const MixinCloseButton(),
                  ],
                ),
              ),
              if (items.isEmpty)
                Expanded(
                  child: Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SvgPicture.asset(
                          MixinAssets.empty,
                          height: 80,
                          width: 80,
                          colorFilter: ColorFilter.mode(
                            context.mixinTheme.secondaryText,
                            BlendMode.srcIn,
                          ),
                        ),
                        const SizedBox(height: 20),
                        Text(
                          context.l10n.noResults,
                          style: TextStyle(
                            color: context.mixinTheme.secondaryText,
                          ),
                        ),
                      ],
                    ),
                  ),
                )
              else
                Expanded(
                  child: ListView.builder(
                    controller: scrollController,
                    padding: const EdgeInsets.only(bottom: _bottomPadding),
                    itemCount: items.length,
                    itemExtent: _itemHeight,
                    itemBuilder: (context, index) => Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 20),
                      child: _PaletteItem(
                        item: items[index],
                        keyword: textController.text.trim(),
                        selected: selectedIndex == index,
                        onTap: () => _select(index),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    ),
  );
}

class _PaletteSearchField extends StatelessWidget {
  const _PaletteSearchField({
    required this.controller,
    required this.focusNode,
  });

  final TextEditingController controller;
  final FocusNode focusNode;

  @override
  Widget build(BuildContext context) {
    final background = Theme.of(context).brightness == Brightness.light
        ? const Color.fromRGBO(245, 247, 250, 1)
        : Colors.white.withValues(alpha: 0.08);
    return Container(
      height: 36,
      decoration: ShapeDecoration(
        color: background,
        shape: const StadiumBorder(),
      ),
      child: Row(
        children: [
          Padding(
            padding: const EdgeInsets.only(left: 16, right: 8),
            child: SvgPicture.asset(
              MixinAssets.search,
              colorFilter: ColorFilter.mode(
                context.mixinTheme.secondaryText,
                BlendMode.srcIn,
              ),
            ),
          ),
          Expanded(
            child: TextField(
              controller: controller,
              focusNode: focusNode,
              autofocus: true,
              contextMenuBuilder: (context, state) =>
                  MixinAdaptiveSelectionToolbar(editableTextState: state),
              style: TextStyle(color: context.mixinTheme.text, fontSize: 14),
              decoration: InputDecoration(
                isDense: true,
                border: InputBorder.none,
                hintText: context.l10n.search,
                hintStyle: TextStyle(color: context.mixinTheme.secondaryText),
                contentPadding: EdgeInsets.zero,
              ),
            ),
          ),
          if (controller.text.isNotEmpty)
            InkWell(
              onTap: controller.clear,
              child: Padding(
                padding: const EdgeInsets.only(right: 16, left: 8),
                child: Icon(
                  Icons.close,
                  color: context.mixinTheme.secondaryText,
                  size: 16,
                ),
              ),
            )
          else
            const SizedBox(width: 40),
        ],
      ),
    );
  }
}

class _PaletteItem extends StatelessWidget {
  const _PaletteItem({
    required this.item,
    required this.keyword,
    required this.selected,
    required this.onTap,
  });

  final CommandPaletteItem item;
  final String keyword;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final decoration = BoxDecoration(
      borderRadius: const BorderRadius.all(Radius.circular(8)),
      color: context.mixinTheme.listSelected,
    );
    return InteractiveDecoratedBox(
      decoration: selected ? decoration : const BoxDecoration(),
      hoveringDecoration: decoration,
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14),
        child: Row(
          children: [
            if (item.conversation != null)
              ConversationAvatarView(
                conversation: item.conversation!,
                size: 40,
              )
            else
              AvatarView(
                userId: item.id,
                name: item.name,
                avatarUrl: item.avatarUrl,
                size: 40,
              ),
            const SizedBox(width: 12),
            Expanded(
              child: _PaletteHighlightedText(text: item.name, keyword: keyword),
            ),
            if (item.isVerified || item.isBot)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: SvgPicture.asset(
                  item.isVerified ? MixinAssets.verified : MixinAssets.botBadge,
                  width: 12,
                  height: 12,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _PaletteHighlightedText extends StatelessWidget {
  const _PaletteHighlightedText({required this.text, required this.keyword});

  final String text;
  final String keyword;

  @override
  Widget build(BuildContext context) {
    final index = text.toLowerCase().indexOf(keyword.toLowerCase());
    final style = TextStyle(color: context.mixinTheme.text, fontSize: 16);
    if (keyword.isEmpty || index < 0) {
      return Text(
        text,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: style,
      );
    }
    return Text.rich(
      TextSpan(
        style: style,
        children: [
          TextSpan(text: text.substring(0, index)),
          TextSpan(
            text: text.substring(index, index + keyword.length),
            style: TextStyle(color: context.mixinTheme.accent),
          ),
          TextSpan(text: text.substring(index + keyword.length)),
        ],
      ),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }
}

class _MoveIntent extends Intent {
  const _MoveIntent(this.delta);

  final int delta;
}

class _SelectIntent extends Intent {
  const _SelectIntent();
}
