import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;
import 'package:system_tray/system_tray.dart';
import 'package:window_manager/window_manager.dart';

import '../l10n/l10n.dart';

class SystemTrayWidget extends StatefulWidget {
  const SystemTrayWidget({required this.child, super.key});

  final Widget child;

  @override
  State<SystemTrayWidget> createState() => _SystemTrayWidgetState();
}

class _SystemTrayWidgetState extends State<SystemTrayWidget> {
  final _systemTray = SystemTray();
  var _initialized = false;

  @override
  void initState() {
    super.initState();
    if (Platform.isWindows) unawaited(_initialize());
  }

  Future<void> _initialize() async {
    await _systemTray.initSystemTray(
      title: 'Mixin',
      iconPath: p.joinAll([
        p.dirname(Platform.resolvedExecutable),
        'data/flutter_assets',
        'assets/images/notify_icon.ico',
      ]),
      toolTip: 'Mixin',
    );
    _systemTray.registerSystemTrayEventHandler((eventName) {
      switch (eventName) {
        case 'leftMouseUp':
          unawaited(windowManager.show());
        case 'rightMouseUp':
          unawaited(_systemTray.popUpContextMenu());
      }
    });
    if (!mounted) return;
    _initialized = true;
    await _updateMenu();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (Platform.isWindows && _initialized) unawaited(_updateMenu());
  }

  Future<void> _updateMenu() => _systemTray.setContextMenu([
    MenuItem(label: context.l10n.show, onClicked: windowManager.show),
    MenuSeparator(),
    MenuItem(label: context.l10n.exit, onClicked: () => exit(0)),
  ]);

  @override
  Widget build(BuildContext context) => widget.child;
}
