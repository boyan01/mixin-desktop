import 'dart:async';

import 'package:dbus/dbus.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:protocol_handler/protocol_handler.dart';
import 'package:window_manager/window_manager.dart';

class AppProtocolHandler extends StatefulWidget {
  const AppProtocolHandler({
    required this.child,
    required this.onUri,
    this.initialUrl,
    super.key,
  });

  final Widget child;
  final ValueChanged<Uri> onUri;
  final String? initialUrl;

  static bool maybeOpen(BuildContext context, Uri uri) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<_AppProtocolUriScope>();
    if (scope == null) return false;
    scope.onUri(uri);
    return true;
  }

  @override
  State<AppProtocolHandler> createState() => _AppProtocolHandlerState();
}

class _AppProtocolHandlerState extends State<AppProtocolHandler>
    with ProtocolListener {
  DBusClient? _dbusClient;
  _MixinDbusObject? _dbusObject;

  @override
  void initState() {
    super.initState();
    if (defaultTargetPlatform == TargetPlatform.linux) {
      unawaited(_initializeLinuxHandler());
    } else {
      protocolHandler.addListener(this);
    }
    unawaited(_openInitialUri());
  }

  Future<void> _openInitialUri() async {
    try {
      final source =
          widget.initialUrl ??
          (defaultTargetPlatform == TargetPlatform.linux
              ? null
              : await protocolHandler.getInitialUrl());
      if (!mounted || source == null) return;
      _open(source);
    } on Object {
      return;
    }
  }

  Future<void> _initializeLinuxHandler() async {
    final client = DBusClient.session();
    final object = _MixinDbusObject(
      open: (url) {
        unawaited(windowManager.show());
        unawaited(windowManager.focus());
        if (url != null) _open(url);
      },
    );
    _dbusClient = client;
    _dbusObject = object;
    final reply = await client.requestName(
      'one.mixin.messenger',
      flags: {DBusRequestNameFlag.replaceExisting},
    );
    if (reply != DBusRequestNameReply.primaryOwner) return;
    await client.registerObject(object);
  }

  @override
  void onProtocolUrlReceived(String url) => _open(url);

  void _open(String source) {
    final uri = Uri.tryParse(source);
    if (uri == null) return;
    if (!kIsWeb &&
        const {
          TargetPlatform.linux,
          TargetPlatform.macOS,
          TargetPlatform.windows,
        }.contains(defaultTargetPlatform)) {
      unawaited(windowManager.show());
      unawaited(windowManager.focus());
    }
    widget.onUri(uri);
  }

  @override
  void dispose() {
    if (defaultTargetPlatform == TargetPlatform.linux) {
      final client = _dbusClient;
      final object = _dbusObject;
      if (client != null && object != null) client.unregisterObject(object);
      if (client != null) {
        unawaited(client.releaseName('one.mixin.messenger'));
        unawaited(client.close());
      }
    } else {
      protocolHandler.removeListener(this);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      _AppProtocolUriScope(onUri: widget.onUri, child: widget.child);
}

class _AppProtocolUriScope extends InheritedWidget {
  const _AppProtocolUriScope({required this.onUri, required super.child});

  final ValueChanged<Uri> onUri;

  @override
  bool updateShouldNotify(_AppProtocolUriScope oldWidget) =>
      onUri != oldWidget.onUri;
}

class _MixinDbusObject extends DBusObject {
  _MixinDbusObject({required this.open})
    : super(DBusObjectPath('/one/mixin/messenger'));

  final ValueChanged<String?> open;

  @override
  Future<DBusMethodResponse> handleMethodCall(DBusMethodCall call) async {
    if (call.interface != 'one.mixin.messenger') {
      return DBusMethodErrorResponse.unknownInterface();
    }
    if (call.name == 'Open') {
      final value = call.values.firstOrNull;
      open(value is DBusString ? value.value : null);
    }
    return DBusMethodSuccessResponse();
  }
}
