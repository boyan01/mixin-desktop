import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:intl/intl.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../constants/assets.dart';
import '../models/message_list_entry.dart';
import '../theme.dart';
import 'message_style.dart';

class MessageDatetimeAndStatus extends StatelessWidget {
  const MessageDatetimeAndStatus({
    required this.message,
    required this.isCurrentUser,
    super.key,
    this.color,
    this.hideStatus = false,
    this.isRepresentative = false,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final Color? color;
  final bool hideStatus;
  final bool isRepresentative;

  @override
  Widget build(BuildContext context) {
    final effectiveColor =
        color ??
        context.dynamicColor(
          const Color.fromRGBO(131, 145, 158, 1),
          darkColor: const Color.fromRGBO(128, 131, 134, 1),
        );
    return SelectionContainer.disabled(
      child: SizedBox(
        height: 12,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (message.pinned)
              _MessageMetadataIcon(
                assetName: MixinAssets.messagePin,
                color: color,
              ),
            if (_isSecretCategory(message.category))
              _MessageMetadataIcon(
                assetName: MixinAssets.messageSecret,
                color: color,
              ),
            if (isRepresentative)
              _MessageMetadataIcon(
                assetName: MixinAssets.messageRepresentative,
                color: color,
              ),
            Text(
              DateFormat.Hm().format(message.createdAt.toLocal()),
              key: Key('message-time-${message.id}'),
              style: TextStyle(
                color: effectiveColor,
                fontSize: context.messageStyle.statusFontSize,
              ),
            ),
            if (isCurrentUser && !hideStatus) ...[
              const SizedBox(width: 8),
              MessageStatusIcon(
                messageId: message.id,
                status: message.status,
                color: color,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _MessageMetadataIcon extends StatelessWidget {
  const _MessageMetadataIcon({required this.assetName, this.color});

  final String assetName;
  final Color? color;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(right: 4),
    child: SvgPicture.asset(
      assetName,
      width: 8,
      height: 8,
      colorFilter: ColorFilter.mode(
        color ??
            context.dynamicColor(
              const Color.fromRGBO(131, 145, 158, 1),
              darkColor: const Color.fromRGBO(128, 131, 134, 1),
            ),
        BlendMode.srcIn,
      ),
    ),
  );
}

bool _isSecretCategory(String category) =>
    category.startsWith('SIGNAL_') || category.startsWith('ENCRYPTED_');

class MessageStatusIcon extends StatelessWidget {
  const MessageStatusIcon({
    required this.messageId,
    required this.status,
    super.key,
    this.color,
  });

  final String messageId;
  final String status;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final normalized = status.toUpperCase();
    final effectiveColor = normalized == 'READ'
        ? context.theme.accent
        : color ?? context.theme.secondaryText;
    final asset = switch (normalized) {
      'SENT' => MixinAssets.sent,
      'DELIVERED' => MixinAssets.delivered,
      'READ' => MixinAssets.read,
      _ => null,
    };
    if (asset == null) {
      return _VisibilityAwareSendingIcon(
        key: Key('message-status-$messageId'),
        color: effectiveColor,
      );
    }
    return SvgPicture.asset(
      asset,
      key: Key('message-status-$messageId'),
      colorFilter: ColorFilter.mode(effectiveColor, BlendMode.srcIn),
    );
  }
}

class _VisibilityAwareSendingIcon extends StatefulWidget {
  const _VisibilityAwareSendingIcon({required this.color, super.key});

  final Color color;

  @override
  State<_VisibilityAwareSendingIcon> createState() =>
      _VisibilityAwareSendingIconState();
}

class _VisibilityAwareSendingIconState
    extends State<_VisibilityAwareSendingIcon>
    with SingleTickerProviderStateMixin, WidgetsBindingObserver {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 2400),
  );
  bool _visible = false;
  late bool _active;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    final lifecycle = WidgetsBinding.instance.lifecycleState;
    _active = lifecycle == null || lifecycle == AppLifecycleState.resumed;
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _updateAnimation();
  }

  void _updateAnimation() {
    if (_visible && _active) {
      if (!_controller.isAnimating) unawaited(_controller.repeat());
    } else {
      _controller.stop();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => VisibilityDetector(
    key: ValueKey(widget.key),
    onVisibilityChanged: (info) {
      if (!mounted) return;
      _visible = info.visibleFraction > 0;
      _updateAnimation();
    },
    child: AnimatedBuilder(
      animation: _controller,
      builder: (_, _) => CustomPaint(
        painter: _SendingIconPainter(
          color: widget.color,
          progress: _controller.value,
        ),
        child: const SizedBox.square(dimension: 14),
      ),
    ),
  );
}

class _SendingIconPainter extends CustomPainter {
  const _SendingIconPainter({required this.color, required this.progress});

  final Color color;
  final double progress;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 1;
    final center = Offset(size.width / 2, size.height / 2);
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        Rect.fromCenter(center: center, width: 11, height: 9),
        const Radius.circular(2.15),
      ),
      paint,
    );
    final minuteAngle = math.pi * 2 * (1 - progress * 4);
    final hourAngle = math.pi * 2 * (1 - progress);
    canvas
      ..drawLine(
        center,
        center + Offset(math.sin(hourAngle) * 3, math.cos(hourAngle) * 3),
        paint,
      )
      ..drawLine(
        center,
        center + Offset(math.sin(minuteAngle) * 4, math.cos(minuteAngle) * 4),
        paint,
      );
  }

  @override
  bool shouldRepaint(covariant _SendingIconPainter oldDelegate) =>
      oldDelegate.color != color || oldDelegate.progress != progress;
}
