import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../controllers/device_transfer_controller.dart';
import '../l10n/l10n.dart';
import '../theme.dart';
import 'buttons.dart';
import 'mixin_dialog.dart';
import 'settings_widgets.dart';

Future<void> showDeviceTransferDialog(
  BuildContext context,
  DeviceTransferController controller,
) async => showMixinDialog<void>(
  context: context,
  child: Provider<DeviceTransferController>.value(
    value: controller,
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 400),
      child: const Material(
        color: Colors.transparent,
        child: _DeviceTransferNavigatorWidget(),
      ),
    ),
  ),
);

enum _DeviceTransferPageType {
  deviceTransfer,
  restore,
  backup,
  restoreWaitingConnect,
  backupWaitingConnect;

  Widget build() => switch (this) {
    _DeviceTransferPageType.deviceTransfer => const _DeviceTransferPage(),
    _DeviceTransferPageType.restore => const _RestorePage(),
    _DeviceTransferPageType.backup => const _BackupPage(),
    _DeviceTransferPageType.restoreWaitingConnect =>
      const _RestoreWaitingConnectPage(),
    _DeviceTransferPageType.backupWaitingConnect =>
      const _BackupWaitingConnectPage(),
  };
}

class _DeviceTransferNavigator extends ChangeNotifier {
  final pages = <_DeviceTransferPageType>[
    _DeviceTransferPageType.deviceTransfer,
  ];

  void push(_DeviceTransferPageType page) {
    pages.add(page);
    notifyListeners();
  }

  void pop() {
    if (pages.length <= 1) return;
    pages.removeLast();
    notifyListeners();
  }
}

class _DeviceTransferNavigatorWidget extends StatefulWidget {
  const _DeviceTransferNavigatorWidget();

  @override
  State<_DeviceTransferNavigatorWidget> createState() =>
      _DeviceTransferNavigatorWidgetState();
}

class _DeviceTransferNavigatorWidgetState
    extends State<_DeviceTransferNavigatorWidget> {
  final navigator = _DeviceTransferNavigator();

  @override
  void initState() {
    super.initState();
    navigator.addListener(_changed);
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    navigator
      ..removeListener(_changed)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      ChangeNotifierProvider<_DeviceTransferNavigator>.value(
        value: navigator,
        child: AnimatedSize(
          duration: const Duration(milliseconds: 150),
          alignment: Alignment.topCenter,
          child: navigator.pages.last.build(),
        ),
      );
}

class _DeviceTransferPage extends StatelessWidget {
  const _DeviceTransferPage();

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      MixinAppBar(
        backgroundColor: Colors.transparent,
        title: const Text('Chat Backup and Restore'),
        leading: const SizedBox.shrink(),
        actions: [
          MixinCloseButton(
            onTap: () => Navigator.maybeOf(context, rootNavigator: true)?.pop(),
          ),
        ],
      ),
      const SizedBox(height: 20),
      CellGroup(
        padding: const EdgeInsets.symmetric(horizontal: 20),
        child: CellItem(
          title: const Text('sync from other device'),
          onTap: () => context.read<_DeviceTransferNavigator>().push(
            _DeviceTransferPageType.restore,
          ),
        ),
      ),
      const SizedBox(height: 16),
      CellGroup(
        padding: const EdgeInsets.symmetric(horizontal: 20),
        child: CellItem(
          title: const Text('sync to other device'),
          onTap: () => context.read<_DeviceTransferNavigator>().push(
            _DeviceTransferPageType.backup,
          ),
        ),
      ),
      const SizedBox(height: 32),
    ],
  );
}

class _DialogBackButton extends StatelessWidget {
  const _DialogBackButton({this.onTapped});

  final VoidCallback? onTapped;

  @override
  Widget build(BuildContext context) {
    final navigator = context.watch<_DeviceTransferNavigator>();
    return navigator.pages.length <= 1
        ? const SizedBox.shrink()
        : Center(
            child: MixinBackButton(
              onTap: () {
                onTapped?.call();
                navigator.pop();
              },
            ),
          );
  }
}

class _RestorePage extends StatelessWidget {
  const _RestorePage();

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      MixinAppBar(
        backgroundColor: Colors.transparent,
        title: const Text('sync from other device'),
        leading: const _DialogBackButton(),
        actions: [
          MixinCloseButton(
            onTap: () => Navigator.maybeOf(context, rootNavigator: true)?.pop(),
          ),
        ],
      ),
      const SizedBox(height: 32),
      SvgPicture.asset(MixinAssets.deviceTransfer),
      const SizedBox(height: 20),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 36),
        child: Text(
          'restore chat tip',
          style: TextStyle(
            color: context.mixinTheme.secondaryText,
            fontSize: 14,
          ),
          textAlign: TextAlign.center,
        ),
      ),
      const SizedBox(height: 40),
      CellGroup(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: CellItem(
          title: const Text('restore chat'),
          color: context.mixinTheme.accent,
          trailing: null,
          onTap: () {
            unawaited(
              context.read<DeviceTransferController>().command(
                DeviceTransferCommand.pullToRemote,
              ),
            );
            context.read<_DeviceTransferNavigator>().push(
              _DeviceTransferPageType.restoreWaitingConnect,
            );
          },
        ),
      ),
      const SizedBox(height: 40),
    ],
  );
}

class _RestoreWaitingConnectPage extends StatefulWidget {
  const _RestoreWaitingConnectPage();

  @override
  State<_RestoreWaitingConnectPage> createState() =>
      _RestoreWaitingConnectPageState();
}

class _RestoreWaitingConnectPageState
    extends State<_RestoreWaitingConnectPage> {
  StreamSubscription<DeviceTransferCallbackEvent>? subscription;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    subscription ??= context
        .read<DeviceTransferController>()
        .on(DeviceTransferCallbackType.onRestoreStart)
        .listen((_) {
          if (mounted) {
            Navigator.maybeOf(context, rootNavigator: true)?.pop();
          }
        });
  }

  @override
  void dispose() {
    unawaited(subscription?.cancel());
    super.dispose();
  }

  void _cancel() {
    unawaited(
      context.read<DeviceTransferController>().command(
        DeviceTransferCommand.cancelRestore,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      MixinAppBar(
        backgroundColor: Colors.transparent,
        title: const Text('restore from other device'),
        leading: _DialogBackButton(onTapped: _cancel),
        actions: [
          MixinCloseButton(
            onTap: () {
              Navigator.maybeOf(context, rootNavigator: true)?.pop();
              _cancel();
            },
          ),
        ],
      ),
      const SizedBox(height: 32),
      SvgPicture.asset(MixinAssets.transferFromPhone),
      const SizedBox(height: 20),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 36),
        child: Text(
          'waiting other device connection',
          style: TextStyle(
            color: context.mixinTheme.secondaryText,
            fontSize: 14,
          ),
          textAlign: TextAlign.center,
        ),
      ),
      const SizedBox(height: 40),
      TextButton(
        onPressed: () {
          Navigator.maybeOf(context, rootNavigator: true)?.pop();
          _cancel();
        },
        child: Text(context.l10n.cancel),
      ),
      const SizedBox(height: 40),
    ],
  );
}

class _BackupPage extends StatelessWidget {
  const _BackupPage();

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      MixinAppBar(
        backgroundColor: Colors.transparent,
        title: const Text('backup to other device'),
        leading: const _DialogBackButton(),
        actions: [
          MixinCloseButton(
            onTap: () => Navigator.maybeOf(context, rootNavigator: true)?.pop(),
          ),
        ],
      ),
      const SizedBox(height: 32),
      SvgPicture.asset(
        MixinAssets.deviceTransfer,
        colorFilter: ColorFilter.mode(
          context.mixinTheme.secondaryText,
          BlendMode.srcIn,
        ),
      ),
      const SizedBox(height: 20),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 36),
        child: Text(
          'tips for backup to other device',
          style: TextStyle(
            color: context.mixinTheme.secondaryText,
            fontSize: 14,
          ),
          textAlign: TextAlign.center,
        ),
      ),
      const SizedBox(height: 40),
      CellGroup(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: CellItem(
          title: const Text('backup chat'),
          color: context.mixinTheme.accent,
          trailing: null,
          onTap: () {
            unawaited(
              context.read<DeviceTransferController>().command(
                DeviceTransferCommand.pushToRemote,
              ),
            );
            context.read<_DeviceTransferNavigator>().push(
              _DeviceTransferPageType.backupWaitingConnect,
            );
          },
        ),
      ),
      const SizedBox(height: 40),
    ],
  );
}

class _BackupWaitingConnectPage extends StatefulWidget {
  const _BackupWaitingConnectPage();

  @override
  State<_BackupWaitingConnectPage> createState() =>
      _BackupWaitingConnectPageState();
}

class _BackupWaitingConnectPageState extends State<_BackupWaitingConnectPage> {
  StreamSubscription<DeviceTransferCallbackEvent>? subscription;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    subscription ??= context
        .read<DeviceTransferController>()
        .on(DeviceTransferCallbackType.onBackupStart)
        .listen((_) {
          if (mounted) {
            Navigator.maybeOf(context, rootNavigator: true)?.pop();
          }
        });
  }

  @override
  void dispose() {
    unawaited(subscription?.cancel());
    super.dispose();
  }

  void _cancel() {
    unawaited(
      context.read<DeviceTransferController>().command(
        DeviceTransferCommand.cancelBackup,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      MixinAppBar(
        backgroundColor: Colors.transparent,
        title: const Text('backup to other device'),
        leading: _DialogBackButton(onTapped: _cancel),
        actions: [
          MixinCloseButton(
            onTap: () {
              Navigator.maybeOf(context, rootNavigator: true)?.pop();
              _cancel();
            },
          ),
        ],
      ),
      const SizedBox(height: 32),
      SvgPicture.asset(MixinAssets.transferToPhone),
      const SizedBox(height: 20),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 36),
        child: Text(
          'restore waiting other device',
          style: TextStyle(
            color: context.mixinTheme.secondaryText,
            fontSize: 14,
          ),
          textAlign: TextAlign.center,
        ),
      ),
      const SizedBox(height: 40),
      TextButton(
        onPressed: () {
          Navigator.maybeOf(context, rootNavigator: true)?.pop();
          _cancel();
        },
        child: Text(context.l10n.cancel),
      ),
      const SizedBox(height: 40),
    ],
  );
}
