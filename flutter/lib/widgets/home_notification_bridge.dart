import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';

import '../controllers/conversation_list_controller.dart';
import '../controllers/home_navigation_controller.dart';
import '../controllers/settings_controller.dart';
import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart';
import '../utils/app_logger.dart';
import '../utils/local_notification_center.dart';
import '../utils/mixin_uri.dart';
import '../utils/notification_preview.dart';
import 'toast.dart';

class HomeNotificationBridge extends StatefulWidget {
  const HomeNotificationBridge({
    required this.account,
    required this.child,
    super.key,
  });

  final AccountHandle account;
  final Widget child;

  @override
  State<HomeNotificationBridge> createState() => _HomeNotificationBridgeState();
}

class _HomeNotificationBridgeState extends State<HomeNotificationBridge> {
  StreamSubscription<NotificationEvent>? _events;
  StreamSubscription<Uri>? _selections;

  @override
  void initState() {
    super.initState();
    _events = widget.account.desktopNotificationEvents().listen(
      (event) => unawaited(_showMessageNotification(event)),
      onError: (Object error, StackTrace stackTrace) =>
          e('watch desktop notifications failed', error, stackTrace),
    );
    _selections = notificationSelections.listen(
      (uri) => unawaited(_selectNotification(uri)),
      onError: (Object error, StackTrace stackTrace) =>
          e('watch notification selections failed', error, stackTrace),
    );
  }

  @override
  void dispose() {
    unawaited(_events?.cancel());
    unawaited(_selections?.cancel());
    super.dispose();
  }

  Future<void> _showMessageNotification(NotificationEvent message) async {
    final dismissMessageId = message.dismissMessageId;
    if (dismissMessageId != null) {
      await dismissMessageNotification(dismissMessageId);
      return;
    }
    final createdAt = DateTime.fromMicrosecondsSinceEpoch(
      message.createdAtMicros,
    );
    if (createdAt.isBefore(
      DateTime.now().subtract(const Duration(minutes: 2)),
    )) {
      return;
    }
    final navigation = context.read<HomeNavigationController>();
    final appActive =
        WidgetsBinding.instance.lifecycleState == null ||
        WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed;
    if (appActive &&
        navigation.selectedConversation?.id == message.conversationId) {
      return;
    }

    final showPreview = context.read<SettingsController>().messagePreview;
    final preview = showPreview
        ? notificationPreview(context, message)
        : context.l10n.aMessage;
    await showMessageNotification(
      title: message.conversationName,
      body: preview,
      uri: Uri(
        scheme: 'mixin',
        host: 'conversations',
        path: message.conversationId,
        queryParameters: {'message': message.messageId},
      ),
      conversationId: message.conversationId,
      messageId: message.messageId,
    );
  }

  Future<void> _selectNotification(Uri uri) async {
    await windowManager.show();
    await windowManager.focus();
    if (!mounted) return;
    final conversationId = uri.conversationId;
    if (conversationId?.trim().isEmpty ?? true) return;
    final conversation = await context
        .read<ConversationListController>()
        .findConversation(conversationId!);
    if (!mounted) return;
    if (conversation == null) {
      showToastFailed(null);
      return;
    }
    final messageId = uri.queryParameters['message'];
    final navigation = context.read<HomeNavigationController>()
      ..select(conversation);
    if (messageId?.isNotEmpty == true) navigation.locateMessage(messageId!);
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
