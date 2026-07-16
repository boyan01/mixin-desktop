import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:lottie/lottie.dart';
import 'package:mixin_desktop_ui/network/core_http_scope.dart';
import 'package:mixin_desktop_ui/utils/cache_client.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';

const _cacheLottieFolderName = 'cache_lottie';

class StickerItem extends HookWidget {
  const StickerItem({
    required this.assetUrl,
    required this.assetType,
    super.key,
    this.stickerId,
    this.errorWidget,
    this.width,
    this.height,
  });

  final String assetUrl;
  final String? assetType;
  final String? stickerId;
  final Widget? errorWidget;
  final double? width;
  final double? height;

  @override
  Widget build(BuildContext context) {
    final network = CoreHttpScope.maybeOf(context);
    final isJson = useMemoized(() => assetType?.toLowerCase() == 'json', [
      assetType,
    ]);
    final controller = useAnimationController();
    final cacheClient = useMemoized(
      () => CacheClient(_cacheLottieFolderName, client: network?.client),
      [network?.client],
    );
    useEffect(() => cacheClient.close, [cacheClient]);
    final lifecycleState = useAppLifecycleState();
    final shouldPlay =
        lifecycleState == null || lifecycleState == AppLifecycleState.resumed;

    useEffect(() {
      if (!isJson || controller.duration == null) return null;
      if (shouldPlay) {
        unawaited(controller.repeat());
      } else {
        controller.stop();
      }
      return null;
    }, [controller, isJson, shouldPlay]);

    final child = assetUrl.isEmpty
        ? errorWidget ?? const SizedBox()
        : isJson
        ? LottieBuilder(
            lottie: NetworkLottie(assetUrl, client: cacheClient),
            controller: controller,
            height: height,
            width: width,
            fit: BoxFit.contain,
            onLoaded: (composition) {
              controller.duration = composition.duration;
              if (shouldPlay) unawaited(controller.repeat());
            },
            errorBuilder: (context, error, stackTrace) =>
                errorWidget ?? const SizedBox(),
          )
        : MixinImage.network(
            assetUrl,
            height: height,
            width: width,
            fit: BoxFit.contain,
            normalizeGif: assetType?.toLowerCase() == 'gif',
            errorBuilder: (context, error, stackTrace) =>
                errorWidget ?? const SizedBox(),
          );

    if (width == null || height == null) {
      return AspectRatio(aspectRatio: 1, child: child);
    }

    return child;
  }
}

class StickerGroupIcon extends StatelessWidget {
  const StickerGroupIcon({
    required this.iconUrl,
    required this.size,
    super.key,
  });

  final String iconUrl;
  final double size;

  @override
  Widget build(BuildContext context) {
    final isLottie = iconUrl.toLowerCase().endsWith('.json');
    return StickerItem(
      assetUrl: iconUrl,
      assetType: isLottie ? 'json' : null,
      width: size,
      height: size,
    );
  }
}
