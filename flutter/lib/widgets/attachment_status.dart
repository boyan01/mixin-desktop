import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../src/rust/api/account.dart';
import '../theme.dart';

class AttachmentStatusPending extends HookWidget {
  const AttachmentStatusPending({required this.messageId, super.key});

  final String messageId;

  @override
  Widget build(BuildContext context) {
    final progress = useState<double>(0);
    useEffect(() {
      progress.value = 0;
      final timer = Timer.periodic(const Duration(milliseconds: 100), (_) {
        final value = context.read<AccountHandle>().attachmentProgress(
          messageId: messageId,
        );
        if (value != progress.value) progress.value = value;
      });
      return timer.cancel;
    }, [messageId]);

    return _StatusLayout(
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
            tween: Tween<double>(end: progress.value),
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
