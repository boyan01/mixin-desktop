import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
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

extension MessageBubbleSemantics on MessageListEntry {
  bool get showMessageBubble =>
      !isSticker && !(isImage && (caption?.trim().isEmpty ?? true));

  bool get includeMessageBubbleNip =>
      (isImage && (caption?.trim().isEmpty ?? true)) ||
      isVideo ||
      category.endsWith('_LOCATION') ||
      category == 'SYSTEM_SAFE_INSCRIPTION';

  bool get clipMessageBubble =>
      isImage ||
      isVideo ||
      isSticker ||
      category.endsWith('_LOCATION') ||
      category == 'SYSTEM_SAFE_INSCRIPTION';

  bool get useOuterMessageDateAndStatus =>
      isAudio ||
      isSticker ||
      category.endsWith('_DATA') ||
      category.endsWith('_CONTACT') ||
      category.endsWith('_LOCATION') ||
      category == 'SYSTEM_ACCOUNT_SNAPSHOT' ||
      category == 'SYSTEM_SAFE_SNAPSHOT' ||
      category == 'SYSTEM_SAFE_INSCRIPTION';

  bool get hideOuterMessageStatus => category.startsWith('SYSTEM_');

  bool? get forceCurrentMessageBubbleColor =>
      isAudio ||
          category == 'SYSTEM_SAFE_SNAPSHOT' ||
          category == 'SYSTEM_SAFE_INSCRIPTION'
      ? false
      : null;

  EdgeInsetsGeometry get messageBubblePadding =>
      category.endsWith('_TRANSCRIPT')
      ? const EdgeInsets.only(top: 4, bottom: 2, right: 2, left: 2)
      : isImage ||
            isVideo ||
            isSticker ||
            category.endsWith('_LOCATION') ||
            category == 'SYSTEM_SAFE_INSCRIPTION'
      ? EdgeInsets.zero
      : const EdgeInsets.all(8);
}

class MessageBubble extends StatelessWidget {
  const MessageBubble({
    required this.child,
    required this.isCurrentUser,
    required this.showNip,
    super.key,
    this.showBubble = true,
    this.includeNip = false,
    this.clip = false,
    this.padding = const EdgeInsets.all(8),
    this.highlighted = false,
    this.outerTimeAndStatusWidget,
    this.forceIsCurrentUserColor,
  });

  final Widget child;
  final bool isCurrentUser;
  final bool showNip;
  final bool showBubble;
  final bool includeNip;
  final bool clip;
  final EdgeInsetsGeometry padding;
  final bool highlighted;
  final Widget? outerTimeAndStatusWidget;
  final bool? forceIsCurrentUserColor;

  @override
  Widget build(BuildContext context) {
    final clipper = BubbleClipper(currentUser: isCurrentUser, showNip: showNip);
    Widget content = child;
    if (!includeNip) {
      content = MessageBubbleNipPadding(
        currentUser: isCurrentUser,
        child: content,
      );
    }
    content = Padding(padding: padding, child: content);

    if (clip) {
      content = RepaintBoundary(
        child: ClipPath(clipper: clipper, child: content),
      );
    }

    final bubbleColor = context.messageBubbleColor(
      forceIsCurrentUserColor ?? isCurrentUser,
    );
    if (showBubble) {
      content = CustomPaint(
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
      );
    } else if (highlighted) {
      content = DecoratedBox(
        decoration: BoxDecoration(
          color: context.theme.accent.withValues(alpha: 0.16),
          borderRadius: const BorderRadius.all(Radius.circular(8)),
        ),
        child: content,
      );
    }

    return Align(
      alignment: isCurrentUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          content,
          if (outerTimeAndStatusWidget != null)
            Padding(
              padding: const EdgeInsets.only(left: 10, right: 10, top: 4),
              child: outerTimeAndStatusWidget,
            ),
        ],
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
