import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';

typedef CustomPopupMenuItemBuilder<T> =
    List<CustomPopupMenuItem<T>> Function(BuildContext context);

class CustomPopupMenuButton<T> extends StatefulWidget {
  const CustomPopupMenuButton({
    required this.itemBuilder,
    this.onSelected,
    this.child,
    this.icon,
    this.color,
    this.alignment,
    this.size = 24,
    this.padding = const EdgeInsets.all(8),
    this.onVisibilityChanged,
    super.key,
  });

  final CustomPopupMenuItemBuilder<T> itemBuilder;
  final PopupMenuItemSelected<T>? onSelected;
  final String? icon;
  final Widget? child;
  final Color? color;
  final Alignment? alignment;
  final double size;
  final EdgeInsets padding;
  final ValueChanged<bool>? onVisibilityChanged;

  @override
  State<CustomPopupMenuButton<T>> createState() =>
      _CustomPopupMenuButtonState<T>();
}

class _CustomPopupMenuButtonState<T> extends State<CustomPopupMenuButton<T>> {
  Offset? position;

  void _show(TapUpDetails details) {
    final alignment = widget.alignment;
    if (alignment == null) {
      setState(() => position = details.globalPosition);
      widget.onVisibilityChanged?.call(true);
      return;
    }
    final renderBox = context.findRenderObject() as RenderBox?;
    if (renderBox == null) {
      setState(() => position = details.globalPosition);
      widget.onVisibilityChanged?.call(true);
      return;
    }
    final local = alignment.withinRect(renderBox.paintBounds);
    setState(() => position = renderBox.localToGlobal(local));
    widget.onVisibilityChanged?.call(true);
  }

  void _close() {
    setState(() => position = null);
    widget.onVisibilityChanged?.call(false);
  }

  @override
  Widget build(BuildContext context) {
    final visible = position != null;
    return PortalTarget(
      visible: visible,
      portalFollower: GestureDetector(
        behavior: visible
            ? HitTestBehavior.opaque
            : HitTestBehavior.deferToChild,
        onTap: _close,
      ),
      child: PortalTarget(
        visible: visible,
        portalFollower: position == null
            ? const SizedBox()
            : CustomSingleChildLayout(
                delegate: PositionedLayoutDelegate(position: position!),
                child: _ContextMenuPage(
                  items: widget.itemBuilder(context),
                  onSelected: (value) {
                    _close();
                    widget.onSelected?.call(value);
                  },
                ),
              ),
        child: Focus(
          onKeyEvent: (node, event) {
            if (visible && event.logicalKey == LogicalKeyboardKey.escape) {
              _close();
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: ActionButton(
            name: widget.icon,
            color: widget.color ?? context.theme.icon,
            onTapUp: _show,
            size: widget.size,
            padding: widget.padding,
            child: widget.child,
          ),
        ),
      ),
    );
  }
}

class CustomPopupMenuItem<T> {
  const CustomPopupMenuItem({
    required this.title,
    required this.value,
    this.isDestructiveAction = false,
    this.icon,
  });

  final String title;
  final T value;
  final bool isDestructiveAction;
  final String? icon;
}

class PositionedLayoutDelegate extends SingleChildLayoutDelegate {
  PositionedLayoutDelegate({required this.position});

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
  bool shouldRelayout(PositionedLayoutDelegate oldDelegate) =>
      position != oldDelegate.position;
}

class _ContextMenuPage<T> extends StatelessWidget {
  const _ContextMenuPage({required this.items, required this.onSelected});

  final List<CustomPopupMenuItem<T>> items;
  final ValueChanged<T> onSelected;

  @override
  Widget build(BuildContext context) => _ContextMenuContainerLayout(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final item in items)
          _ContextMenuItem<T>(item: item, onSelected: onSelected),
      ],
    ),
  );
}

class _ContextMenuContainerLayout extends StatelessWidget {
  const _ContextMenuContainerLayout({required this.child});

  final Widget child;

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
          const Color.fromRGBO(255, 255, 255, 1),
          darkColor: const Color.fromRGBO(62, 65, 72, 1),
        ),
      ),
      child: ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(11)),
        child: Material(
          color: Colors.transparent,
          child: IntrinsicWidth(child: child),
        ),
      ),
    );
  }
}

class _ContextMenuItem<T> extends StatelessWidget {
  const _ContextMenuItem({required this.item, required this.onSelected});

  final CustomPopupMenuItem<T> item;
  final ValueChanged<T> onSelected;

  @override
  Widget build(BuildContext context) {
    final backgroundColor = context.dynamicColor(
      const Color.fromRGBO(255, 255, 255, 1),
      darkColor: const Color.fromRGBO(62, 65, 72, 1),
    );
    final color = item.isDestructiveAction
        ? context.theme.red
        : context.dynamicColor(
            const Color.fromRGBO(0, 0, 0, 1),
            darkColor: const Color.fromRGBO(255, 255, 255, 0.9),
          );
    return ConstrainedBox(
      constraints: const BoxConstraints(minWidth: 160),
      child: InteractiveDecoratedBox.color(
        decoration: BoxDecoration(color: backgroundColor),
        tapDowningColor: Color.alphaBlend(
          context.theme.listSelected,
          backgroundColor,
        ),
        onTap: () => onSelected(item.value),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Row(
            children: [
              if (item.icon != null)
                Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: SvgPicture.asset(
                    item.icon!,
                    colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
                    width: 20,
                    height: 20,
                  ),
                ),
              Expanded(
                child: Text(
                  item.title,
                  style: TextStyle(fontSize: 14, color: color),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
