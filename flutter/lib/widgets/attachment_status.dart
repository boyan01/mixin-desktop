import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:provider/provider.dart';

class AttachmentStatusPending extends StatefulWidget {
  const AttachmentStatusPending({required this.messageId, super.key});

  final String messageId;

  @override
  State<AttachmentStatusPending> createState() =>
      _AttachmentStatusPendingState();
}

class _AttachmentStatusPendingState extends State<AttachmentStatusPending> {
  Timer? _timer;
  double _value = 0;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _timer ??= Timer.periodic(const Duration(milliseconds: 100), (_) {
      if (!mounted) return;
      final value = context.read<AccountHandle>().downloadProgress(
        messageId: widget.messageId,
      );
      if (value != _value) setState(() => _value = value);
    });
  }

  @override
  void didUpdateWidget(covariant AttachmentStatusPending oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.messageId != widget.messageId) _value = 0;
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _StatusLayout(
    child: Stack(
      fit: StackFit.expand,
      children: [
        Center(
          child: SizedBox.fromSize(
            size: const Size.square(10),
            child: DecoratedBox(
              decoration: BoxDecoration(color: context.theme.accent),
            ),
          ),
        ),
        TweenAnimationBuilder<double>(
          tween: Tween<double>(end: _value),
          duration: const Duration(milliseconds: 100),
          builder: (context, value, _) => CircularProgressIndicator(
            value: value,
            valueColor: AlwaysStoppedAnimation(context.theme.accent),
          ),
        ),
      ],
    ),
  );
}

class AttachmentStatusWarning extends StatelessWidget {
  const AttachmentStatusWarning({super.key});

  @override
  Widget build(BuildContext context) => _StatusLayout(
    child: Center(
      child: SvgPicture.asset(
        MixinAssets.statusWarning,
        colorFilter: ColorFilter.mode(context.theme.text, BlendMode.srcIn),
      ),
    ),
  );
}

class AttachmentStatusDownload extends StatelessWidget {
  const AttachmentStatusDownload({super.key});

  @override
  Widget build(BuildContext context) => _StatusLayout(
    child: Center(
      child: SvgPicture.asset(
        MixinAssets.statusDownload,
        colorFilter: ColorFilter.mode(context.theme.accent, BlendMode.srcIn),
      ),
    ),
  );
}

class AttachmentStatusUpload extends StatelessWidget {
  const AttachmentStatusUpload({super.key});

  @override
  Widget build(BuildContext context) => _StatusLayout(
    child: Center(
      child: SvgPicture.asset(
        MixinAssets.statusUpload,
        colorFilter: ColorFilter.mode(context.theme.accent, BlendMode.srcIn),
      ),
    ),
  );
}

class AttachmentStatusAudioPlay extends StatelessWidget {
  const AttachmentStatusAudioPlay({super.key});

  @override
  Widget build(BuildContext context) => _StatusLayout(
    child: Center(
      child: SvgPicture.asset(
        MixinAssets.statusAudioPlay,
        colorFilter: ColorFilter.mode(context.theme.accent, BlendMode.srcIn),
      ),
    ),
  );
}

class AttachmentStatusAudioStop extends StatelessWidget {
  const AttachmentStatusAudioStop({super.key});

  @override
  Widget build(BuildContext context) => _StatusLayout(
    child: Center(
      child: SvgPicture.asset(
        MixinAssets.statusAudioStop,
        colorFilter: ColorFilter.mode(context.theme.accent, BlendMode.srcIn),
      ),
    ),
  );
}

class _StatusLayout extends StatelessWidget {
  const _StatusLayout({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => Container(
    height: 38,
    width: 38,
    decoration: BoxDecoration(
      color: context.theme.statusBackground,
      shape: BoxShape.circle,
    ),
    child: child,
  );
}
