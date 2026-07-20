import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';

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
  bool get isInvalidImageMessage =>
      isImage && (mediaWidth == null || mediaHeight == null);

  bool get isInvalidSpecialMessage {
    try {
      final value = jsonDecode(content);
      if (category.endsWith('_LOCATION')) {
        return value is! Map<String, dynamic> ||
            value['latitude'] is! num ||
            value['longitude'] is! num;
      }
      if (category.endsWith('_TRANSCRIPT') || category == 'APP_BUTTON_GROUP') {
        return value is! List<dynamic> ||
            value.any((item) => item is! Map<String, dynamic>);
      }
      if (category == 'APP_CARD') {
        if (value is! Map<String, dynamic>) return true;
        final actions = value['actions'];
        return actions != null &&
            (actions is! List<dynamic> ||
                actions.any((item) => item is! Map<String, dynamic>));
      }
      return false;
    } on Object {
      return category.endsWith('_LOCATION') ||
          category.endsWith('_TRANSCRIPT') ||
          category == 'APP_BUTTON_GROUP' ||
          category == 'APP_CARD';
    }
  }

  bool get isUnresolvedMessage {
    final normalized = status.toUpperCase();
    return normalized == 'UNKNOWN' || normalized == 'FAILED';
  }

  bool get showMessageBubble =>
      isUnresolvedMessage ||
      isInvalidImageMessage ||
      isInvalidSpecialMessage ||
      (!isSticker && !(isImage && (caption?.trim().isEmpty ?? true)));

  bool get includeMessageBubbleNip =>
      !isUnresolvedMessage &&
      !isInvalidImageMessage &&
      !isInvalidSpecialMessage &&
      ((isImage && (caption?.trim().isEmpty ?? true)) ||
          isVideo ||
          category.endsWith('_LOCATION') ||
          category == 'SYSTEM_SAFE_INSCRIPTION');

  bool get clipMessageBubble =>
      !isUnresolvedMessage &&
      !isInvalidImageMessage &&
      !isInvalidSpecialMessage &&
      (isImage ||
          isVideo ||
          isSticker ||
          category.endsWith('_LOCATION') ||
          category == 'SYSTEM_SAFE_INSCRIPTION');

  bool get useOuterMessageDateAndStatus =>
      !isUnresolvedMessage &&
      !isInvalidImageMessage &&
      !isInvalidSpecialMessage &&
      (isAudio ||
          isSticker ||
          category.endsWith('_DATA') ||
          category.endsWith('_CONTACT') ||
          category.endsWith('_LOCATION') ||
          category == 'SYSTEM_ACCOUNT_SNAPSHOT' ||
          category == 'SYSTEM_SAFE_SNAPSHOT' ||
          category == 'SYSTEM_SAFE_INSCRIPTION');

  bool get hideOuterMessageStatus => category.startsWith('SYSTEM_');

  bool? get forceCurrentMessageBubbleColor =>
      isAudio ||
          category == 'SYSTEM_SAFE_SNAPSHOT' ||
          category == 'SYSTEM_SAFE_INSCRIPTION'
      ? false
      : null;

  EdgeInsetsGeometry get messageBubblePadding =>
      isUnresolvedMessage || isInvalidImageMessage || isInvalidSpecialMessage
      ? const EdgeInsets.all(8)
      : category.endsWith('_TRANSCRIPT')
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
    this.highlightOpacity = 0,
    this.outerTimeAndStatusWidget,
    this.forceIsCurrentUserColor,
    this.isDisappearingMessage = false,
    this.isPinnedPage = false,
    this.onPinnedMessageTap,
    this.quote,
    this.constrainQuoteWidth = false,
    this.highlightMedia = false,
    this.shrinkWrap = false,
  });

  final Widget child;
  final bool isCurrentUser;
  final bool showNip;
  final bool showBubble;
  final bool includeNip;
  final bool clip;
  final EdgeInsetsGeometry padding;
  final bool highlighted;
  final double highlightOpacity;
  final Widget? outerTimeAndStatusWidget;
  final bool? forceIsCurrentUserColor;
  final bool isDisappearingMessage;
  final bool isPinnedPage;
  final VoidCallback? onPinnedMessageTap;
  final Widget? quote;
  final bool constrainQuoteWidth;
  final bool highlightMedia;
  final bool shrinkWrap;

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

    if (quote != null) {
      final quoteContent = MessageBubbleNipPadding(
        currentUser: isCurrentUser,
        child: quote!,
      );
      content = constrainQuoteWidth
          ? _ConstrainedQuoteLayout(quote: quoteContent, content: content)
          : IntrinsicWidth(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [quoteContent, content],
              ),
            );
    }

    if (clip) {
      content = RepaintBoundary(
        child: ClipPath(clipper: clipper, child: content),
      );
    }

    final bubbleColor = context.messageBubbleColor(
      forceIsCurrentUserColor ?? isCurrentUser,
    );
    if (quote != null || showBubble) {
      content = CustomPaint(
        painter: BubblePainter(color: bubbleColor, clipper: clipper),
        child: content,
      );
    }
    final effectiveHighlightOpacity = highlighted ? 1.0 : highlightOpacity;
    if ((quote != null || showBubble || highlightMedia) &&
        effectiveHighlightOpacity > 0) {
      content = MessageBubbleHighlight(
        clipper: clipper,
        currentUser: isCurrentUser,
        media: highlightMedia,
        opacity: effectiveHighlightOpacity,
        child: content,
      );
    }

    if (isPinnedPage) {
      final pinArrow = ActionButton(
        size: 16,
        name: MixinAssets.pinArrow,
        onTap: onPinnedMessageTap,
      );
      content = Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isCurrentUser) pinArrow,
          Flexible(child: content),
          if (!isCurrentUser) pinArrow,
        ],
      );
    }

    if (isDisappearingMessage) {
      final icon = Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: SvgPicture.asset(
          Theme.of(context).brightness == Brightness.dark
              ? MixinAssets.expiringDark
              : MixinAssets.expiring,
          width: 16,
          height: 16,
        ),
      );
      content = Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isCurrentUser) icon,
          Flexible(child: content),
          if (!isCurrentUser) icon,
        ],
      );
    }

    return Align(
      alignment: isCurrentUser ? Alignment.centerRight : Alignment.centerLeft,
      widthFactor: shrinkWrap ? 1 : null,
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

class _ConstrainedQuoteLayout extends MultiChildRenderObjectWidget {
  _ConstrainedQuoteLayout({required Widget quote, required Widget content})
    : super(children: [quote, content]);

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderConstrainedQuoteLayout();
}

class _RenderConstrainedQuoteLayout extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, MultiChildLayoutParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, MultiChildLayoutParentData> {
  RenderBox get quoteChild => firstChild!;

  RenderBox get contentChild => lastChild!;

  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! MultiChildLayoutParentData) {
      child.parentData = MultiChildLayoutParentData();
    }
  }

  @override
  void performLayout() {
    final childConstraints = constraints.loosen();
    contentChild.layout(childConstraints, parentUsesSize: true);
    final width = constraints.constrainWidth(contentChild.size.width);
    quoteChild.layout(
      BoxConstraints.tightFor(width: width),
      parentUsesSize: true,
    );
    size = constraints.constrain(
      Size(width, quoteChild.size.height + contentChild.size.height),
    );
    (quoteChild.parentData! as MultiChildLayoutParentData).offset = Offset.zero;
    (contentChild.parentData! as MultiChildLayoutParentData).offset = Offset(
      0,
      quoteChild.size.height,
    );
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      defaultHitTestChildren(result, position: position);

  @override
  void paint(PaintingContext context, Offset offset) =>
      defaultPaint(context, offset);
}

class MessageBubbleHighlight extends StatelessWidget {
  const MessageBubbleHighlight({
    required this.clipper,
    required this.currentUser,
    required this.media,
    required this.child,
    super.key,
    this.enabled = true,
    this.opacity = 1,
  });

  final CustomClipper<Path> clipper;
  final bool currentUser;
  final bool media;
  final Widget child;
  final bool enabled;
  final double opacity;

  @override
  Widget build(BuildContext context) {
    if (!enabled || opacity <= 0) return child;
    final dark = Theme.of(context).brightness == Brightness.dark;
    final color = media || currentUser
        ? Colors.black
        : dark
        ? Colors.white
        : Colors.black;
    final baseOpacity = media
        ? 0.2
        : currentUser
        ? (dark ? 0.18 : 0.16)
        : (dark ? 0.12 : 0.13);
    return CustomPaint(
      foregroundPainter: _MessageBubbleHighlightPainter(
        clipper: clipper,
        color: color.withValues(alpha: baseOpacity * opacity),
      ),
      child: child,
    );
  }
}

class _MessageBubbleHighlightPainter extends CustomPainter {
  const _MessageBubbleHighlightPainter({
    required this.clipper,
    required this.color,
  });

  final CustomClipper<Path> clipper;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawPath(clipper.getClip(size), Paint()..color = color);
  }

  @override
  bool shouldRepaint(covariant _MessageBubbleHighlightPainter oldDelegate) =>
      clipper != oldDelegate.clipper || color != oldDelegate.color;
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
