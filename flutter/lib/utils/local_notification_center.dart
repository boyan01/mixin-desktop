import 'dart:async';
import 'dart:io';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';

final _plugin = FlutterLocalNotificationsPlugin();
final _selectionController = StreamController<Uri>.broadcast();
final _notifications = <String, ({int id, String conversationId})>{};
var _nextId = 0;

Stream<Uri> get notificationSelections => _selectionController.stream;

Future<void> initNotificationListener() async {
  if (!Platform.isMacOS && !Platform.isLinux && !Platform.isWindows) return;
  await _plugin.initialize(
    settings: InitializationSettings(
      macOS: const DarwinInitializationSettings(),
      linux: const LinuxInitializationSettings(defaultActionName: 'default'),
      windows: const WindowsInitializationSettings(
        appName: 'Mixin Messenger',
        appUserModelId: '14801MixinLtd.MixinDesktop',
        guid: '94B64592-528D-48B4-B37B-C82D634F1BE7',
      ),
    ),
    onDidReceiveNotificationResponse: (response) {
      final payload = response.payload;
      if (payload == null) return;
      final uri = Uri.tryParse(payload);
      if (uri != null) _selectionController.add(uri);
    },
  );
  await _plugin.cancelAll();
  _notifications.clear();
}

Future<void> showMessageNotification({
  required String title,
  required String body,
  required Uri uri,
  required String conversationId,
  required String messageId,
}) async {
  if (!Platform.isMacOS && !Platform.isLinux && !Platform.isWindows) return;
  _nextId = (_nextId + 1) & 0x7fffffff;
  const details = NotificationDetails(
    macOS: DarwinNotificationDetails(sound: 'mixin.caf', presentSound: true),
    windows: WindowsNotificationDetails(),
  );
  await _plugin.show(
    id: _nextId,
    title: title,
    body: body,
    notificationDetails: details,
    payload: uri.toString(),
  );
  _notifications[messageId] = (id: _nextId, conversationId: conversationId);
}

Future<void> dismissMessageNotification(String messageId) async {
  final notification = _notifications.remove(messageId);
  if (notification != null) await _plugin.cancel(id: notification.id);
}

Future<void> dismissConversationNotifications(String conversationId) async {
  final entries = _notifications.entries
      .where((entry) => entry.value.conversationId == conversationId)
      .toList(growable: false);
  for (final entry in entries) {
    await _plugin.cancel(id: entry.value.id);
    _notifications.remove(entry.key);
  }
}

Future<bool?> requestNotificationPermission() async => await _plugin
    .resolvePlatformSpecificImplementation<
      MacOSFlutterLocalNotificationsPlugin
    >()
    ?.requestPermissions(alert: true, badge: true, sound: true);
