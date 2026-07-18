import 'package:desktop_webview_window/desktop_webview_window.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/theme.dart';

class WebViewNavigationBar extends StatelessWidget {
  const WebViewNavigationBar({super.key});

  @override
  Widget build(BuildContext context) {
    final state = TitleBarWebViewState.of(context);
    final controller = TitleBarWebViewController.of(context);
    return Row(
      children: [
        const SizedBox(width: 10),
        _NavigationAction(
          asset: MixinAssets.back,
          color: state.canGoBack
              ? context.mixinTheme.icon
              : context.mixinTheme.icon.withValues(alpha: 0.5),
          onTap: controller.back,
        ),
        const SizedBox(width: 16),
        _NavigationAction(
          asset: MixinAssets.forward,
          color: state.canGoForward
              ? context.mixinTheme.icon
              : context.mixinTheme.icon.withValues(alpha: 0.5),
          onTap: controller.forward,
        ),
        const SizedBox(width: 16),
        _NavigationAction(
          asset: MixinAssets.webViewRefresh,
          color: context.mixinTheme.icon,
          onTap: controller.reload,
        ),
      ],
    );
  }
}

class _NavigationAction extends StatelessWidget {
  const _NavigationAction({
    required this.asset,
    required this.color,
    required this.onTap,
  });

  final String asset;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => GestureDetector(
    onTap: onTap,
    child: SvgPicture.asset(
      asset,
      width: 16,
      height: 16,
      colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
    ),
  );
}
