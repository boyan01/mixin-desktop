import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'interactive_decorated_box.dart';

class ActionButton extends StatelessWidget {
  const ActionButton({
    super.key,
    this.name,
    this.child,
    this.onTap,
    this.onTapUp,
    this.padding = const EdgeInsets.all(8),
    this.size = 24,
    this.color,
    this.onEnter,
    this.onExit,
    this.onHover,
    this.interactive = true,
  }) : assert(name != null || child != null);

  final String? name;
  final Widget? child;
  final VoidCallback? onTap;
  final GestureTapUpCallback? onTapUp;
  final EdgeInsets padding;
  final double size;
  final Color? color;
  final PointerEnterEventListener? onEnter;
  final PointerExitEventListener? onExit;
  final PointerHoverEventListener? onHover;
  final bool interactive;

  @override
  Widget build(BuildContext context) {
    var buttonChild = child;
    if (name?.isNotEmpty ?? false) {
      buttonChild = SvgPicture.asset(
        name!,
        width: size,
        height: size,
        colorFilter: color == null
            ? null
            : ColorFilter.mode(color!, BlendMode.srcIn),
      );
    }
    buttonChild = Padding(
      padding: padding,
      child: SizedBox.square(dimension: size, child: buttonChild),
    );
    if (!interactive) return buttonChild;
    return InteractiveDecoratedBox.color(
      onTap: onTap,
      onTapUp: onTapUp,
      onEnter: onEnter,
      onExit: onExit,
      onHover: onHover,
      decoration: const BoxDecoration(shape: BoxShape.circle),
      hoveringColor: Theme.of(context).brightness == Brightness.light
          ? const Color.fromRGBO(0, 0, 0, 0.03)
          : const Color.fromRGBO(255, 255, 255, 0.2),
      child: buttonChild,
    );
  }
}
