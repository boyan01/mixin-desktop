import 'dart:async';

import 'package:desktop_keep_screen_on/desktop_keep_screen_on.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import 'mixin_dialog.dart';

class DeviceTransferHandlerWidget extends HookWidget {
  const DeviceTransferHandlerWidget({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final account = context.read<AccountHandle>();
    final progressShowing = useRef(false);

    Future<void> showProgress({required bool restore}) async {
      if (progressShowing.value || !context.mounted) return;
      progressShowing.value = true;
      await showMixinDialog<void>(
        context: context,
        child: restore
            ? const _RestoreProcessingDialog()
            : const _BackupProcessingDialog(),
        barrierDismissible: false,
      );
      progressShowing.value = false;
    }

    Future<void> finishProgress(String message) async {
      if (!context.mounted || !progressShowing.value) return;
      Navigator.of(context, rootNavigator: true).pop();
      progressShowing.value = false;
      await showMixinDialog<void>(
        context: context,
        child: _ConfirmDialog(message: message),
        barrierDismissible: false,
      );
    }

    useEffect(() {
      final subscription = account.deviceTransferEvents().listen((event) async {
        switch (event) {
          case DeviceTransferEvent_RestoreConnected():
            await showProgress(restore: true);
          case DeviceTransferEvent_BackupServerCreated():
            await showProgress(restore: false);
          case DeviceTransferEvent_RestoreSucceed():
          case DeviceTransferEvent_BackupSucceed():
            await finishProgress(context.l10n.transferCompleted);
          case DeviceTransferEvent_RestoreFailed():
          case DeviceTransferEvent_BackupFailed():
            await finishProgress(context.l10n.deviceTransferFailed);
          case DeviceTransferEvent_BackupRequestReceived():
            final approved = await showMixinDialog<bool>(
              context: context,
              child: _ApproveDialog(
                message: context.l10n.confirmSyncChatsFromPhone,
              ),
            );
            await account.deviceTransferCommand(
              command: approved == true
                  ? DeviceTransferCommand.confirmRestore
                  : DeviceTransferCommand.cancelRestoreRequest,
            );
          case DeviceTransferEvent_RestoreRequestReceived():
            final approved = await showMixinDialog<bool>(
              context: context,
              child: _ApproveDialog(
                message: context.l10n.confirmSyncChatsToPhone,
              ),
            );
            await account.deviceTransferCommand(
              command: approved == true
                  ? DeviceTransferCommand.confirmBackup
                  : DeviceTransferCommand.cancelBackupRequest,
            );
          case DeviceTransferEvent_ConnectionFailed(field0: final version):
            await showMixinDialog<void>(
              context: context,
              child: _ConfirmDialog(
                message: version == ConnectionFailedReason.versionNotMatched
                    ? context.l10n.transferProtocolVersionNotMatched
                    : context.l10n.deviceTransferFailed,
              ),
            );
          case DeviceTransferEvent_RestoreStart():
          case DeviceTransferEvent_BackupStart():
          case DeviceTransferEvent_RestoreProgress():
          case DeviceTransferEvent_BackupProgress():
          case DeviceTransferEvent_RestoreNetworkSpeed():
          case DeviceTransferEvent_BackupNetworkSpeed():
            break;
        }
      });
      return subscription.cancel;
    }, [account]);

    return child;
  }
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
  const _RestoreProcessingDialog();

  @override
  Widget build(BuildContext context) => _TransferProcessDialog(
    onCancelTapped: () {
      unawaited(
        context.read<AccountHandle>().deviceTransferCommand(
          command: DeviceTransferCommand.cancelRestore,
        ),
      );
      Navigator.pop(context);
    },
    iconAssetName: MixinAssets.transferFromPhone,
    progressValue: (event) => switch (event) {
      DeviceTransferEvent_RestoreProgress(:final field0) => field0,
      _ => null,
    },
    networkSpeedValue: (event) => switch (event) {
      DeviceTransferEvent_RestoreNetworkSpeed(:final field0) => field0,
      _ => null,
    },
  );
}

class _BackupProcessingDialog extends StatelessWidget {
  const _BackupProcessingDialog();

  @override
  Widget build(BuildContext context) => _TransferProcessDialog(
    onCancelTapped: () {
      unawaited(
        context.read<AccountHandle>().deviceTransferCommand(
          command: DeviceTransferCommand.cancelBackup,
        ),
      );
      Navigator.pop(context);
    },
    iconAssetName: MixinAssets.transferToPhone,
    progressValue: (event) => switch (event) {
      DeviceTransferEvent_BackupProgress(:final field0) => field0,
      _ => null,
    },
    networkSpeedValue: (event) => switch (event) {
      DeviceTransferEvent_BackupNetworkSpeed(:final field0) => field0,
      _ => null,
    },
  );
}

class _TransferProcessDialog extends StatefulWidget {
  const _TransferProcessDialog({
    required this.onCancelTapped,
    required this.progressValue,
    required this.iconAssetName,
    required this.networkSpeedValue,
  });

  final VoidCallback onCancelTapped;
  final double? Function(DeviceTransferEvent event) progressValue;
  final String iconAssetName;
  final double? Function(DeviceTransferEvent event) networkSpeedValue;

  @override
  State<_TransferProcessDialog> createState() => _TransferProcessDialogState();
}

class _TransferProcessDialogState extends State<_TransferProcessDialog> {
  double progress = 0;
  double networkSpeed = 0;
  late final StreamSubscription<DeviceTransferEvent> subscription;

  @override
  void initState() {
    super.initState();
    DesktopKeepScreenOn.setPreventSleep(true);
    subscription = context.read<AccountHandle>().deviceTransferEvents().listen((
      event,
    ) {
      final progressValue = widget.progressValue(event);
      if (progressValue != null && mounted) {
        setState(() => progress = progressValue);
      }

      final networkSpeedValue = widget.networkSpeedValue(event);
      if (networkSpeedValue != null && mounted) {
        setState(() => networkSpeed = networkSpeedValue);
      }
    });
  }

  @override
  void dispose() {
    DesktopKeepScreenOn.setPreventSleep(false);
    unawaited(subscription.cancel());
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
