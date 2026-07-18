import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';

const _nipWidth = 7.0;

class PinMessageBubble extends StatelessWidget {
  const PinMessageBubble({
    required this.child,
    super.key,
    this.padding = const EdgeInsets.symmetric(horizontal: 16),
  });

  final Widget child;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    const clipper = _PinBubbleClipper();
    return CustomPaint(
      painter: _PinBubblePainter(
        color: context.messageBubbleColor(false),
        clipper: clipper,
      ),
      child: Padding(
        padding: padding.add(const EdgeInsets.only(right: _nipWidth)),
        child: SizedBox.expand(
          child: DefaultTextStyle.merge(
            style: TextStyle(color: context.theme.text),
            child: child,
          ),
        ),
      ),
    );
  }
}

class _PinBubblePainter extends CustomPainter {
  const _PinBubblePainter({required this.color, required this.clipper});

  final Color color;
  final CustomClipper<Path> clipper;

  @override
  void paint(Canvas canvas, Size size) {
    final path = clipper.getClip(size);
    canvas.drawShadow(path, Colors.black, 0.6, false);
    canvas.drawPath(path, Paint()..color = color);
  }

  @override
  bool shouldRepaint(covariant _PinBubblePainter oldDelegate) =>
      color != oldDelegate.color || clipper != oldDelegate.clipper;
}

class _PinBubbleClipper extends CustomClipper<Path> {
  const _PinBubbleClipper();

  @override
  Path getClip(Size size) {
    final bubblePath = Path()
      ..addRRect(
        const BorderRadius.all(
          Radius.circular(8),
        ).toRRect(Offset.zero & Size(size.width - _nipWidth, size.height)),
      );
    const nipSize = Size(_nipWidth, 10);
    final nipPath = Path()
      ..lineTo(0, 0)
      ..cubicTo(
        0,
        0,
        nipSize.width * 0.85,
        nipSize.height / 3,
        nipSize.width * 0.85,
        nipSize.height / 3,
      )
      ..cubicTo(
        nipSize.width * 1.05,
        nipSize.height * 0.41,
        nipSize.width * 1.05,
        nipSize.height * 0.59,
        nipSize.width * 0.85,
        nipSize.height * 0.67,
      )
      ..cubicTo(
        nipSize.width * 0.85,
        nipSize.height * 0.67,
        0,
        nipSize.height,
        0,
        nipSize.height,
      )
      ..cubicTo(0, nipSize.height, 0, 0, 0, 0);
    return Path.combine(
      PathOperation.union,
      bubblePath,
      nipPath.shift(
        Offset(
          size.width - nipSize.width,
          size.height / 2 - nipSize.height / 2,
        ),
      ),
    );
  }

  @override
  bool shouldReclip(covariant _PinBubbleClipper oldClipper) => false;
}
