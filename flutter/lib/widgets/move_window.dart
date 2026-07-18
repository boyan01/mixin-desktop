import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

class MoveWindowBarrier extends StatelessWidget {
  const MoveWindowBarrier({required this.child, super.key, this.enable = true});

  final Widget child;
  final bool enable;

  @override
  Widget build(BuildContext context) =>
      GestureDetector(onPanStart: enable ? (_) {} : null, child: child);
}

class MoveWindow extends StatelessWidget {
  const MoveWindow({
    super.key,
    this.child,
    this.behavior,
    this.clickToFullScreen = false,
  });

  final Widget? child;
  final HitTestBehavior? behavior;
  final bool clickToFullScreen;

  @override
  Widget build(BuildContext context) => GestureDetector(
    behavior: behavior,
    onDoubleTap: clickToFullScreen
        ? () async {
            if (defaultTargetPlatform != TargetPlatform.macOS) return;
            if (await windowManager.isMaximized()) {
              await windowManager.unmaximize();
              if (await windowManager.isMaximized()) {
                const windowSize = Size(960, 720);
                final position = await calcWindowPosition(
                  windowSize,
                  Alignment.center,
                );
                await windowManager.setBounds(
                  null,
                  position: position,
                  size: windowSize,
                  animate: true,
                );
              }
            } else {
              await windowManager.maximize();
            }
          }
        : null,
    onPanStart: (_) {
      if (defaultTargetPlatform == TargetPlatform.windows) return;
      if (kIsWeb ||
          !const {
            TargetPlatform.linux,
            TargetPlatform.macOS,
            TargetPlatform.windows,
          }.contains(defaultTargetPlatform)) {
        return;
      }
      windowManager.startDragging();
    },
    child: child,
  );
}

class GlobalMoveWindow extends StatelessWidget {
  const GlobalMoveWindow({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.expand,
    textDirection: TextDirection.ltr,
    children: [
      child,
      const Align(
        alignment: Alignment.topCenter,
        child: SizedBox(height: 28, child: MoveWindow(clickToFullScreen: true)),
      ),
    ],
  );
}
