import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/theme.dart';

const _nipWidth = 9.0;
const _lightCurrentBubble = Color.fromRGBO(197, 237, 253, 1);
const _darkCurrentBubble = Color.fromRGBO(59, 79, 103, 1);
const lightOtherBubble = Colors.white;
const darkOtherBubble = Color.fromRGBO(52, 59, 67, 1);

extension BubbleColor on BuildContext {
  Color messageBubbleColor(bool isCurrentUser) => isCurrentUser
      ? dynamicColor(_lightCurrentBubble, darkColor: _darkCurrentBubble)
      : dynamicColor(lightOtherBubble, darkColor: darkOtherBubble);
}

class MessageBubble extends StatelessWidget {
  const MessageBubble({
    required this.child,
    required this.isCurrentUser,
    required this.showNip,
    super.key,
    this.padding = const EdgeInsets.all(8),
    this.highlighted = false,
  });

  final Widget child;
  final bool isCurrentUser;
  final bool showNip;
  final EdgeInsetsGeometry padding;
  final bool highlighted;

  @override
  Widget build(BuildContext context) {
    final clipper = BubbleClipper(currentUser: isCurrentUser, showNip: showNip);
    final content = Padding(
      padding: padding,
      child: MessageBubbleNipPadding(currentUser: isCurrentUser, child: child),
    );

    final bubbleColor = context.messageBubbleColor(isCurrentUser);
    return Align(
      alignment: isCurrentUser ? Alignment.centerRight : Alignment.centerLeft,
      child: CustomPaint(
        painter: BubblePainter(
          color: highlighted
              ? Color.alphaBlend(
                  context.theme.accent.withValues(alpha: 0.16),
                  bubbleColor,
                )
              : bubbleColor,
          clipper: clipper,
        ),
        child: content,
      ),
    );
  }
}

class MessageBubbleNipPadding extends StatelessWidget {
  const MessageBubbleNipPadding({
    required this.currentUser,
    required this.child,
    super.key,
  });

  final bool currentUser;
  final Widget child;

  @override
  Widget build(BuildContext context) => Padding(
    padding: EdgeInsets.only(
      left: currentUser ? 0 : _nipWidth,
      right: currentUser ? _nipWidth : 0,
    ),
    child: child,
  );
}

class BubbleClipper extends CustomClipper<Path> {
  BubbleClipper({
    required this.currentUser,
    required this.showNip,
    this.nipPadding = true,
  });

  final bool currentUser;
  final bool showNip;
  final bool nipPadding;

  @override
  Path getClip(Size size) {
    final nipWidth = nipPadding ? _nipWidth : 0.0;
    final bubblePath = _bubblePath(
      Size(size.width - nipWidth, size.height),
    ).shift(Offset(currentUser ? 0 : nipWidth, 0));

    if (!showNip) return bubblePath;
    final nipPath = currentUser ? _rightNipPath(size) : _leftNipPath(size);
    return Path.combine(PathOperation.union, bubblePath, nipPath);
  }

  Path _bubblePath(Size size) => Path()
    ..addRRect(
      const BorderRadius.all(Radius.circular(8)).toRRect(Offset.zero & size),
    );

  Path _leftNipPath(Size bubbleSize) {
    const size = Size(_nipWidth, 12);
    final path = Path()
      ..lineTo(size.width * 1.04, size.height)
      ..cubicTo(
        size.width * 1.04,
        size.height,
        size.width * 1.04,
        0,
        size.width * 1.04,
        0,
      )
      ..cubicTo(
        size.width * 1.04,
        0,
        size.width * 1.04,
        size.height * 0.12,
        size.width,
        size.height * 0.19,
      )
      ..cubicTo(
        size.width * 0.81,
        size.height * 0.41,
        size.width / 2,
        size.height * 0.59,
        size.width * 0.14,
        size.height * 0.67,
      )
      ..cubicTo(
        size.width * 0.03,
        size.height * 0.69,
        size.width * 0.01,
        size.height * 0.79,
        size.width * 0.11,
        size.height * 0.84,
      )
      ..cubicTo(
        size.width * 0.12,
        size.height * 0.84,
        size.width * 0.13,
        size.height * 0.85,
        size.width * 0.13,
        size.height * 0.85,
      )
      ..cubicTo(
        size.width * 0.36,
        size.height * 0.94,
        size.width * 0.62,
        size.height,
        size.width * 0.91,
        size.height,
      )
      ..cubicTo(
        size.width * 0.95,
        size.height,
        size.width * 1.04,
        size.height,
        size.width * 1.04,
        size.height,
      )
      ..cubicTo(
        size.width * 1.04,
        size.height,
        size.width * 1.04,
        size.height,
        size.width * 1.04,
        size.height,
      );

    return path.shift(Offset(-0.38, bubbleSize.height - 9 - 12));
  }

  Path _rightNipPath(Size bubbleSize) {
    const size = Size(_nipWidth, 12);
    final path = Path()
      ..lineTo(0, size.height)
      ..cubicTo(0, size.height, 0, 0, 0, 0)
      ..cubicTo(
        0,
        0,
        0,
        size.height * 0.12,
        size.width * 0.05,
        size.height * 0.19,
      )
      ..cubicTo(
        size.width * 0.24,
        size.height * 0.41,
        size.width * 0.54,
        size.height * 0.59,
        size.width * 0.9,
        size.height * 0.67,
      )
      ..cubicTo(
        size.width * 1.02,
        size.height * 0.69,
        size.width * 1.04,
        size.height * 0.79,
        size.width * 0.94,
        size.height * 0.84,
      )
      ..cubicTo(
        size.width * 0.92,
        size.height * 0.84,
        size.width * 0.91,
        size.height * 0.85,
        size.width * 0.91,
        size.height * 0.85,
      )
      ..cubicTo(
        size.width * 0.68,
        size.height * 0.94,
        size.width * 0.42,
        size.height,
        size.width * 0.13,
        size.height,
      )
      ..cubicTo(size.width * 0.09, size.height, 0, size.height, 0, size.height)
      ..cubicTo(0, size.height, 0, size.height, 0, size.height);

    return path.shift(
      Offset(bubbleSize.width - 9 - 0.05, bubbleSize.height - 9 - 12),
    );
  }

  @override
  bool shouldReclip(covariant BubbleClipper oldClipper) =>
      currentUser != oldClipper.currentUser ||
      showNip != oldClipper.showNip ||
      nipPadding != oldClipper.nipPadding;
}

class BubblePainter extends CustomPainter {
  BubblePainter({
    required this.clipper,
    required this.color,
    this.elevation = 0.6,
    this.shadowColor = Colors.black,
  });

  final BubbleClipper clipper;
  final Color color;
  final double elevation;
  final Color shadowColor;

  @override
  void paint(Canvas canvas, Size size) {
    final clip = clipper.getClip(size);
    if (elevation != 0) {
      canvas.drawShadow(clip, shadowColor, elevation, false);
    }
    canvas.drawPath(
      clip,
      Paint()
        ..color = color
        ..style = PaintingStyle.fill,
    );
  }

  @override
  bool shouldRepaint(covariant BubblePainter oldDelegate) =>
      clipper.shouldReclip(oldDelegate.clipper) ||
      color != oldDelegate.color ||
      elevation != oldDelegate.elevation ||
      shadowColor != oldDelegate.shadowColor;
}
