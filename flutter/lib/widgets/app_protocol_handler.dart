import 'dart:async';

import 'package:dbus/dbus.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:protocol_handler/protocol_handler.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:window_manager/window_manager.dart';

import '../controllers/conversation_list_controller.dart';
import '../controllers/home_navigation_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';
import '../utils/mixin_uri.dart';
import '../utils/web_view.dart';
import 'show_conversation_code_dialog.dart';
import 'show_message_user_dialog.dart';
import 'show_multisigs_payment_dialog.dart';
import 'show_send_message_dialog.dart';
import 'show_snapshot_detail_dialog.dart';
import 'toast.dart';
import 'unknown_mixin_url_dialog.dart';

class AppProtocolHandler extends StatefulWidget {
  const AppProtocolHandler({
    required this.child,
    this.onUri,
    this.onSelectConversation,
    this.currentConversation,
    this.initialUrl,
    super.key,
  });

  final Widget child;
  final ValueChanged<Uri>? onUri;
  final ValueChanged<ConversationListEntry>? onSelectConversation;
  final ConversationListEntry? currentConversation;
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
    } on Object catch (error, stackTrace) {
      e('Open initial protocol URL failed', error, stackTrace);
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
    _handleUri(uri);
  }

  void _handleUri(Uri uri) {
    if (widget.onUri != null) {
      widget.onUri!(uri);
    } else {
      unawaited(
        openProtocolUri(
          context,
          uri,
          onSelectConversation: widget.onSelectConversation,
          currentConversation: widget.currentConversation,
        ),
      );
    }
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
      _AppProtocolUriScope(onUri: _handleUri, child: widget.child);
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

Future<void> openProtocolUri(
  BuildContext context,
  Uri uri, {
  ValueChanged<ConversationListEntry>? onSelectConversation,
  ConversationListEntry? currentConversation,
}) async {
  final selectedController = onSelectConversation == null
      ? context.read<HomeNavigationController>()
      : null;
  onSelectConversation ??= selectedController?.select;
  currentConversation ??= selectedController?.selectedConversation;

  if (!uri.isMixin) {
    await launchUrl(uri);
    return;
  }

  final userId = uri.userId;
  if (userId?.trim().isNotEmpty == true) {
    await _showUser(
      context,
      userId: userId!,
      onSelectConversation: onSelectConversation,
    );
    return;
  }

  final conversationId = uri.conversationId;
  if (conversationId?.trim().isNotEmpty == true) {
    final controller = context.read<ConversationListController>();
    final conversation = await controller.findConversation(conversationId!);
    if (conversation == null) {
      showToastFailed(null);
      return;
    }
    onSelectConversation?.call(conversation);
    final startText = uri.startTextOfConversation;
    if (startText?.trim().isNotEmpty == true) {
      try {
        await context.read<rust.AccountHandle>().message().sendText(
          conversationId: conversationId,
          content: startText!,
          silent: false,
        );
      } catch (error, stackTrace) {
        e('Send protocol text message failed', error, stackTrace);
        showToastFailed(null);
      }
    }
    return;
  }

  final code = uri.code;
  if (code?.trim().isNotEmpty == true) {
    try {
      final account = context.read<rust.AccountHandle>();
      final result = await account.conversation().resolveCode(code: code!);
      if (result.kind == 'user' && result.userId != null) {
        await _showUser(
          context,
          userId: result.userId!,
          onSelectConversation: onSelectConversation,
        );
        return;
      }
      if (result.kind == 'conversation' && result.conversationId != null) {
        String? conversationId;
        if (result.alreadyMember) {
          showToast(context.l10n.groupAlreadyIn);
          conversationId = result.conversationId;
        } else {
          conversationId = await showConversationCodeDialog(
            context,
            account: account,
            result: result,
            code: code,
          );
        }
        if (conversationId == null) return;
        final controller = context.read<ConversationListController>();
        await controller.refresh();
        final conversation = await controller.findConversation(
          conversationId,
        );
        if (conversation != null) {
          onSelectConversation?.call(conversation);
        }
        return;
      }
      if (result.kind == 'payment' || result.kind == 'multisig_request') {
        await showMultisigsPaymentDialog(context, item: result, uri: uri);
        return;
      }
    } on Object catch (error, stackTrace) {
      e('Resolve protocol code failed', error, stackTrace);
      showToastFailed(null);
      return;
    }
  }

  final snapshotTraceId = uri.snapshotTraceId?.trim();
  if (snapshotTraceId?.isNotEmpty == true) {
    try {
      final snapshot = await context.read<rust.AccountHandle>().snapshotByTrace(
        traceId: snapshotTraceId!,
      );
      await showSnapshotDetailItemDialog(context, snapshot: snapshot);
    } on Object catch (error, stackTrace) {
      e('Load protocol snapshot failed: $snapshotTraceId', error, stackTrace);
      showToastFailed(null);
    }
    return;
  }

  if (uri.isSend) {
    final handled = await showSendMessageDialog(
      context,
      account: context.read<rust.AccountHandle>(),
      category: uri.categoryOfSend,
      conversationId: uri.conversationIdOfSend,
      data: uri.dataOfSend,
      userId: uri.userOfSend,
      currentConversation: currentConversation,
      onSelectConversation: onSelectConversation ?? (_) {},
    );
    if (!handled) showToastFailed(null);
    return;
  }

  if (uri.isPay ||
      uri.isMultisigs ||
      uri.isSwap ||
      uri.isMarkets ||
      uri.isMembership) {
    await showUnknownMixinUrlDialog(context, uri);
    return;
  }

  final appId = uri.appId;
  if (appId?.trim().isNotEmpty == true) {
    if (!uri.actionIsOpen) {
      await _showUser(
        context,
        userId: appId!,
        onSelectConversation: onSelectConversation,
      );
      return;
    }
    final account = context.read<rust.AccountHandle>();
    late final List<Object?> results;
    try {
      results = await Future.wait<Object?>([
        account.user().botHomeUri(appId: appId!),
        account.user().userProfile(userId: appId),
      ]);
    } on Object catch (error, stackTrace) {
      e('Load protocol app failed: $appId', error, stackTrace);
      showToastFailed(ToastError(context.l10n.botNotFound));
      return;
    }
    final homeUri = Uri.tryParse(results.first as String? ?? '');
    if (homeUri == null) {
      showToastFailed(ToastError(context.l10n.botNotFound));
      return;
    }
    final queryParameters = {...homeUri.queryParameters}
      ..addAll({...uri.queryParameters}..remove('action'));
    final profile = results.last as rust.UserProfileItem?;
    await openBotWebViewWindow(
      context: context,
      url: homeUri.replace(queryParameters: queryParameters).toString(),
      title: profile?.fullName ?? '',
      conversationId: conversationId ?? '',
      currency: account.profile().fiatCurrency,
    );
    return;
  }

  if (uri.isMixinScheme) await showUnknownMixinUrlDialog(context, uri);
}

Future<void> _showUser(
  BuildContext context, {
  required String userId,
  ValueChanged<ConversationListEntry>? onSelectConversation,
}) async {
  final account = context.read<rust.AccountHandle>();
  final result = await showMessageUserDialog(
    context,
    account: account,
    userId: userId,
  );
  if (result == null) return;
  await handleMessageUserDialogResult(
    context,
    account: account,
    result: result,
    onSelectConversation: onSelectConversation ?? (_) {},
  );
}

extension BuildContextUrlX on BuildContext {
  void openUrl(String value) {
    final uri = Uri.tryParse(value);
    if (uri == null || uri.scheme.isEmpty) return;
    openUri(uri);
  }

  void openUri(Uri uri) {
    if (!AppProtocolHandler.maybeOpen(this, uri)) {
      unawaited(launchUrl(uri));
    }
  }
}
