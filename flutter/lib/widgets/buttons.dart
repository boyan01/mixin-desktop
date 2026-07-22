import 'package:flutter/widgets.dart';
import '../constants/assets.dart';
import '../theme.dart';
import 'action_button.dart';

class MixinBackButton extends StatelessWidget {
  const MixinBackButton({super.key, this.color, this.onTap});

  final Color? color;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(right: 8),
    child: ActionButton(
      name: MixinAssets.back,
      color: color ?? context.mixinTheme.icon,
      onTap: onTap ?? () => Navigator.pop(context),
    ),
  );
}

class MixinCloseButton extends StatelessWidget {
  const MixinCloseButton({super.key, this.color, this.onTap});

  final Color? color;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => ActionButton(
    name: MixinAssets.close,
    color: color ?? context.mixinTheme.icon,
    onTap: onTap ?? () => Navigator.pop(context),
  );
}

class NTapGestureDetector extends StatefulWidget {
  const NTapGestureDetector({
    required this.child,
    required this.n,
    super.key,
    this.onTap,
  }) : assert(n > 2, 'n must be greater than 2');

  final GestureTapCallback? onTap;
  final Widget child;
  final int n;

  @override
  State<NTapGestureDetector> createState() => _NTapGestureDetectorState();
}

class _NTapGestureDetectorState extends State<NTapGestureDetector> {
  final clicks = <int>[];

  @override
  Widget build(BuildContext context) => GestureDetector(
    onTap: () {
      final now = DateTime.now().millisecondsSinceEpoch;
      if (clicks.isEmpty) {
        clicks.add(now);
        return;
      }
      if (now - clicks.last < 500) {
        clicks.add(now);
      } else {
        clicks.clear();
      }
      if (clicks.length == widget.n) {
        clicks.clear();
        widget.onTap?.call();
      }
    },
    child: widget.child,
  );
}
