import 'dart:ui' show lerpDouble;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import '../theme.dart';
import 'interactive_decorated_box.dart';

class MixinAdaptiveSelectionToolbar extends StatelessWidget {
  const MixinAdaptiveSelectionToolbar({
    required this.editableTextState,
    super.key,
  });

  final EditableTextState editableTextState;

  @override
  Widget build(BuildContext context) {
    if (!_isDesktop) {
      return AdaptiveTextSelectionToolbar.editableText(
        editableTextState: editableTextState,
      );
    }
    return CustomSingleChildLayout(
      delegate: _PositionedLayoutDelegate(
        position: editableTextState.contextMenuAnchors.primaryAnchor,
      ),
      child: _ContextMenuPage(
        menus: [
          if (editableTextState.copyEnabled)
            _ContextMenu(
              title: MaterialLocalizations.of(context).copyButtonLabel,
              onTap: () {
                editableTextState.copySelection(SelectionChangedCause.toolbar);
                ContextMenuController.removeAny();
              },
            ),
          if (editableTextState.cutEnabled)
            _ContextMenu(
              title: MaterialLocalizations.of(context).cutButtonLabel,
              onTap: () {
                editableTextState.cutSelection(SelectionChangedCause.toolbar);
                ContextMenuController.removeAny();
              },
            ),
          if (editableTextState.selectAllEnabled)
            _ContextMenu(
              title: MaterialLocalizations.of(context).selectAllButtonLabel,
              onTap: () {
                editableTextState.selectAll(SelectionChangedCause.toolbar);
                ContextMenuController.removeAny();
              },
            ),
          if (editableTextState.pasteEnabled)
            _ContextMenu(
              title: MaterialLocalizations.of(context).pasteButtonLabel,
              onTap: () {
                editableTextState.pasteText(SelectionChangedCause.toolbar);
                ContextMenuController.removeAny();
              },
            ),
        ],
      ),
    );
  }
}

class MixinAdaptiveSelectionAreaToolbar extends StatelessWidget {
  const MixinAdaptiveSelectionAreaToolbar({required this.state, super.key});

  final SelectableRegionState state;

  @override
  Widget build(BuildContext context) {
    if (!_isDesktop) {
      return AdaptiveTextSelectionToolbar.selectableRegion(
        selectableRegionState: state,
      );
    }
    return CustomSingleChildLayout(
      delegate: _PositionedLayoutDelegate(
        position: state.contextMenuAnchors.primaryAnchor,
      ),
      child: _ContextMenuPage(
        menus: [
          if (state.copyEnabled)
            _ContextMenu(
              title: MaterialLocalizations.of(context).copyButtonLabel,
              onTap: () {
                // ignore: deprecated_member_use
                state.copySelection(SelectionChangedCause.toolbar);
                ContextMenuController.removeAny();
              },
            ),
          if (state.selectAllEnabled)
            _ContextMenu(
              title: MaterialLocalizations.of(context).selectAllButtonLabel,
              onTap: () {
                state.selectAll();
                ContextMenuController.removeAny();
              },
            ),
        ],
      ),
    );
  }
}

bool get _isDesktop => switch (defaultTargetPlatform) {
  TargetPlatform.linux ||
  TargetPlatform.macOS ||
  TargetPlatform.windows => true,
  _ => false,
};

class _PositionedLayoutDelegate extends SingleChildLayoutDelegate {
  _PositionedLayoutDelegate({required this.position});

  final Offset position;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) =>
      BoxConstraints.loose(constraints.biggest);

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    var dx = position.dx + 1;
    var dy = position.dy + 1;
    if (size.width - position.dx < childSize.width) {
      dx = position.dx - childSize.width;
    }
    if (size.height - position.dy < childSize.height) {
      dy = position.dy - childSize.height;
    }
    return Offset(dx, dy);
  }

  @override
  bool shouldRelayout(covariant _PositionedLayoutDelegate oldDelegate) =>
      position != oldDelegate.position;
}

class _ContextMenuPage extends StatelessWidget {
  const _ContextMenuPage({required this.menus});

  final List<Widget> menus;

  @override
  Widget build(BuildContext context) {
    final brightness = Theme.of(context).brightness == Brightness.dark
        ? 1.0
        : 0.0;
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(11)),
        border: Border.all(
          color: Color.lerp(
            Colors.transparent,
            const Color.fromRGBO(255, 255, 255, 0.08),
            brightness,
          )!,
        ),
        boxShadow: [
          BoxShadow(
            color: const Color.fromRGBO(0, 0, 0, 0.15),
            offset: Offset(0, lerpDouble(0, 2, brightness)!),
            blurRadius: lerpDouble(16, 40, brightness)!,
          ),
          BoxShadow(
            color: const Color.fromRGBO(0, 0, 0, 0.07),
            offset: Offset(0, lerpDouble(4, 0, brightness)!),
            blurRadius: lerpDouble(6, 12, brightness)!,
          ),
        ],
        color: context.dynamicColor(
          Colors.white,
          darkColor: const Color.fromRGBO(62, 65, 72, 1),
        ),
      ),
      child: ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(11)),
        child: Material(
          color: Colors.transparent,
          child: IntrinsicWidth(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: menus,
            ),
          ),
        ),
      ),
    );
  }
}

class _ContextMenu extends StatelessWidget {
  const _ContextMenu({required this.title, required this.onTap});

  final String title;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final backgroundColor = context.dynamicColor(
      Colors.white,
      darkColor: const Color.fromRGBO(62, 65, 72, 1),
    );
    return ConstrainedBox(
      constraints: const BoxConstraints(minWidth: 160),
      child: InteractiveDecoratedBox.color(
        decoration: BoxDecoration(color: backgroundColor),
        tapDowningColor: Color.alphaBlend(
          context.theme.listSelected,
          backgroundColor,
        ),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  title,
                  style: TextStyle(
                    fontSize: 14,
                    color: context.dynamicColor(
                      Colors.black,
                      darkColor: const Color.fromRGBO(255, 255, 255, 0.9),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
