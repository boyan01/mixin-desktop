import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:lottie/lottie.dart';
import 'package:provider/provider.dart';

import '../../app.dart';
import '../../network/core_http_scope.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../utils/cache_client.dart';
import '../mixin_image.dart';

const _cacheLottieFolderName = 'cache_lottie';

void _triggerRefreshJob(BuildContext context, String? stickerId) {
  if (stickerId == null || stickerId.isEmpty) return;

  scheduleMicrotask(() {
    if (!context.mounted) return;
    unawaited(
      context.read<rust.AccountHandle>().sticker().refreshSticker(
        stickerId: stickerId,
      ),
    );
  });
}

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
    final isJson = useMemoized(() => assetType == 'json', [assetType]);
    final controller = useAnimationController();
    final cacheClient = useMemoized(
      () => CacheClient(_cacheLottieFolderName, client: network?.client),
      [network?.client],
    );
    useEffect(() => cacheClient.close, [cacheClient]);
    final lifecycleState = useAppLifecycleState();
    final routeVisible = useState(true);
    final routeAware = useMemoized(
      () => _StickerRouteAware(
        () => routeVisible.value = false,
        () => routeVisible.value = true,
      ),
    );
    final secondNavigatorContext = useMemoized(
      () => _secondNavigatorContext(context),
    );
    final route = ModalRoute.of(secondNavigatorContext ?? context);
    useEffect(() {
      if (route == null) return null;
      rootRouteObserver.subscribe(routeAware, route);
      return () => rootRouteObserver.unsubscribe(routeAware);
    }, [routeAware, route]);
    final shouldPlay =
        (lifecycleState == null ||
            lifecycleState == AppLifecycleState.resumed) &&
        routeVisible.value;

    useEffect(() {
      if (!isJson || controller.duration == null) return null;
      if (shouldPlay) {
        unawaited(controller.repeat());
      } else {
        controller.stop();
      }
      return null;
    }, [controller, isJson, shouldPlay]);

    final child = isJson
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
            errorBuilder: (context, error, stackTrace) {
              _triggerRefreshJob(context, stickerId);
              return errorWidget ?? const SizedBox();
            },
          )
        : MixinImage.network(
            assetUrl,
            height: height,
            width: width,
            fit: BoxFit.contain,
            normalizeGif: assetType?.toLowerCase() == 'gif',
            errorBuilder: (context, error, stackTrace) {
              _triggerRefreshJob(context, stickerId);
              return errorWidget ?? const SizedBox();
            },
          );

    if (width == null || height == null) {
      return AspectRatio(aspectRatio: 1, child: child);
    }

    return child;
  }
}

class _StickerRouteAware extends RouteAware {
  _StickerRouteAware(this._didPushNext, this._didPopNext);

  final VoidCallback _didPushNext;
  final VoidCallback _didPopNext;

  @override
  void didPushNext() => _didPushNext();

  @override
  void didPopNext() => _didPopNext();
}

BuildContext? _secondNavigatorContext(BuildContext context) {
  final rootNavigator = Navigator.maybeOf(context, rootNavigator: true);
  if (rootNavigator == null) return null;

  BuildContext? find(BuildContext current) {
    final navigator = current.findAncestorStateOfType<NavigatorState>();
    if (navigator == null) return null;
    if (navigator == rootNavigator) return current;
    return find(navigator.context);
  }

  return find(context);
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
