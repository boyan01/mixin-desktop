import 'dart:async';

import 'package:desktop_keep_screen_on/desktop_keep_screen_on.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/device_transfer_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:provider/provider.dart';

class DeviceTransferHandlerWidget extends StatefulWidget {
  const DeviceTransferHandlerWidget({
    required this.controller,
    required this.child,
    super.key,
  });

  final DeviceTransferController controller;
  final Widget child;

  @override
  State<DeviceTransferHandlerWidget> createState() =>
      _DeviceTransferHandlerWidgetState();
}

class _DeviceTransferHandlerWidgetState
    extends State<DeviceTransferHandlerWidget> {
  StreamSubscription<DeviceTransferCallbackEvent>? subscription;
  bool progressShowing = false;

  @override
  void initState() {
    super.initState();
    subscription = widget.controller.events.listen(_handleEvent);
  }

  Future<void> _handleEvent(DeviceTransferCallbackEvent event) async {
    if (!mounted) return;
    switch (event.action) {
      case DeviceTransferCallbackType.onRestoreConnected:
        await _showProgress(restore: true);
      case DeviceTransferCallbackType.onBackupServerCreated:
        await _showProgress(restore: false);
      case DeviceTransferCallbackType.onRestoreSucceed:
      case DeviceTransferCallbackType.onBackupSucceed:
        await _finishProgress(context.l10n.transferCompleted);
      case DeviceTransferCallbackType.onRestoreFailed:
      case DeviceTransferCallbackType.onBackupFailed:
        await _finishProgress(context.l10n.deviceTransferFailed);
      case DeviceTransferCallbackType.onBackupRequestReceived:
        final approved = await showMixinDialog<bool>(
          context: context,
          child: _ApproveDialog(
            message: context.l10n.confirmSyncChatsFromPhone,
          ),
        );
        await widget.controller.command(
          approved == true
              ? DeviceTransferCommand.confirmRestore
              : DeviceTransferCommand.cancelRestoreRequest,
        );
      case DeviceTransferCallbackType.onRestoreRequestReceived:
        final approved = await showMixinDialog<bool>(
          context: context,
          child: _ApproveDialog(message: context.l10n.confirmSyncChatsToPhone),
        );
        await widget.controller.command(
          approved == true
              ? DeviceTransferCommand.confirmBackup
              : DeviceTransferCommand.cancelBackupRequest,
        );
      case DeviceTransferCallbackType.onConnectionFailed:
        final reason = event.payload as ConnectionFailedReason?;
        await showMixinDialog<void>(
          context: context,
          child: _ConfirmDialog(
            message: reason == ConnectionFailedReason.versionNotMatched
                ? context.l10n.transferProtocolVersionNotMatched
                : context.l10n.deviceTransferFailed,
          ),
        );
      case DeviceTransferCallbackType.onRestoreStart:
      case DeviceTransferCallbackType.onBackupStart:
      case DeviceTransferCallbackType.onRestoreProgress:
      case DeviceTransferCallbackType.onBackupProgress:
      case DeviceTransferCallbackType.onRestoreNetworkSpeed:
      case DeviceTransferCallbackType.onBackupNetworkSpeed:
        break;
    }
  }

  Future<void> _showProgress({required bool restore}) async {
    if (progressShowing || !mounted) return;
    progressShowing = true;
    await showMixinDialog<void>(
      context: context,
      child: restore
          ? _RestoreProcessingDialog(controller: widget.controller)
          : _BackupProcessingDialog(controller: widget.controller),
      barrierDismissible: false,
    );
    progressShowing = false;
  }

  Future<void> _finishProgress(String message) async {
    if (!mounted || !progressShowing) return;
    Navigator.of(context, rootNavigator: true).pop();
    progressShowing = false;
    await showMixinDialog<void>(
      context: context,
      child: _ConfirmDialog(message: message),
      barrierDismissible: false,
    );
  }

  @override
  void dispose() {
    unawaited(subscription?.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      Provider.value(value: widget.controller, child: widget.child);
}

class _ConfirmDialog extends StatelessWidget {
  const _ConfirmDialog({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => AlertDialogLayout(
    content: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 420),
      child: Text(message),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(false),
        child: Text(context.l10n.confirm),
      ),
    ],
  );
}

class _ApproveDialog extends StatelessWidget {
  const _ApproveDialog({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => AlertDialogLayout(
    content: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 420),
      child: Text(message),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(false),
        child: Text(
          context.l10n.cancel,
          style: TextStyle(color: context.mixinTheme.secondaryText),
        ),
      ),
      MixinButton<bool>(
        onTap: () => Navigator.of(context).pop(true),
        child: Text(context.l10n.confirm),
      ),
    ],
  );
}

class _RestoreProcessingDialog extends StatelessWidget {
  const _RestoreProcessingDialog({required this.controller});

  final DeviceTransferController controller;

  @override
  Widget build(BuildContext context) => _TransferProcessDialog(
    controller: controller,
    onCancelTapped: () {
      unawaited(controller.command(DeviceTransferCommand.cancelRestore));
      Navigator.pop(context);
    },
    iconAssetName: MixinAssets.transferFromPhone,
    progressType: DeviceTransferCallbackType.onRestoreProgress,
    networkSpeedType: DeviceTransferCallbackType.onRestoreNetworkSpeed,
  );
}

class _BackupProcessingDialog extends StatelessWidget {
  const _BackupProcessingDialog({required this.controller});

  final DeviceTransferController controller;

  @override
  Widget build(BuildContext context) => _TransferProcessDialog(
    controller: controller,
    onCancelTapped: () {
      unawaited(controller.command(DeviceTransferCommand.cancelBackup));
      Navigator.pop(context);
    },
    iconAssetName: MixinAssets.transferToPhone,
    progressType: DeviceTransferCallbackType.onBackupProgress,
    networkSpeedType: DeviceTransferCallbackType.onBackupNetworkSpeed,
  );
}

class _TransferProcessDialog extends StatefulWidget {
  const _TransferProcessDialog({
    required this.controller,
    required this.onCancelTapped,
    required this.progressType,
    required this.iconAssetName,
    required this.networkSpeedType,
  });

  final DeviceTransferController controller;
  final VoidCallback onCancelTapped;
  final DeviceTransferCallbackType progressType;
  final String iconAssetName;
  final DeviceTransferCallbackType networkSpeedType;

  @override
  State<_TransferProcessDialog> createState() => _TransferProcessDialogState();
}

class _TransferProcessDialogState extends State<_TransferProcessDialog> {
  double progress = 0;
  double networkSpeed = 0;
  final subscriptions = <StreamSubscription<DeviceTransferCallbackEvent>>[];

  @override
  void initState() {
    super.initState();
    DesktopKeepScreenOn.setPreventSleep(true);
    subscriptions
      ..add(
        widget.controller.on(widget.progressType).listen((event) {
          if (mounted) {
            setState(() => progress = (event.payload as num).toDouble());
          }
        }),
      )
      ..add(
        widget.controller.on(widget.networkSpeedType).listen((event) {
          if (mounted) {
            setState(() => networkSpeed = (event.payload as num).toDouble());
          }
        }),
      );
  }

  @override
  void dispose() {
    DesktopKeepScreenOn.setPreventSleep(false);
    for (final subscription in subscriptions) {
      unawaited(subscription.cancel());
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 420,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 40, vertical: 20),
      child: Material(
        color: Colors.transparent,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(height: 40),
            SvgPicture.asset(
              widget.iconAssetName,
              width: 72,
              height: 72,
              colorFilter: ColorFilter.mode(
                context.mixinTheme.secondaryText,
                BlendMode.srcIn,
              ),
            ),
            const SizedBox(height: 38),
            DefaultTextStyle.merge(
              style: TextStyle(color: context.mixinTheme.text, fontSize: 18),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(context.l10n.transferringChats),
                  const SizedBox(width: 2),
                  if (progress > 0) Text('(${progress.toStringAsFixed(2)}%)'),
                ],
              ),
            ),
            const SizedBox(height: 16),
            DefaultTextStyle.merge(
              style: TextStyle(
                color: context.mixinTheme.secondaryText,
                fontSize: 14,
              ),
              textAlign: TextAlign.center,
              child: Text(context.l10n.transferringChatsTips),
            ),
            const SizedBox(height: 18),
            Text(
              formatNetworkSpeed(networkSpeed),
              style: TextStyle(
                color: context.mixinTheme.secondaryText,
                fontSize: 14,
              ),
            ),
            const SizedBox(height: 32),
            TextButton(
              onPressed: widget.onCancelTapped,
              child: Text(
                context.l10n.cancel,
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.normal,
                  color: context.mixinTheme.accent,
                ),
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

String formatNetworkSpeed(double speed) {
  final speedInKb = speed / 1024;
  if (speedInKb < 1024) {
    return '${speedInKb.toStringAsFixed(2)} KB/s';
  }
  return '${(speedInKb / 1024).toStringAsFixed(2)} MB/s';
}
