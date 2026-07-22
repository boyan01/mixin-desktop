import 'dart:async';
import 'dart:convert';

import '../src/rust/desktop_api.dart';

enum DeviceTransferCommand {
  pullToRemote('pull_to_remote'),
  pushToRemote('push_to_remote'),
  cancelRestore('cancel_restore'),
  cancelBackup('cancel_backup'),
  cancelBackupRequest('cancel_backup_request'),
  cancelRestoreRequest('cancel_restore_request'),
  confirmRestore('confirm_restore'),
  confirmBackup('confirm_backup');

  const DeviceTransferCommand(this.wireValue);

  final String wireValue;
}

enum DeviceTransferCallbackType {
  onRestoreConnected('restore_connected'),
  onRestoreStart('restore_start'),
  onRestoreSucceed('restore_succeed'),
  onRestoreFailed('restore_failed'),
  onBackupServerCreated('backup_server_created'),
  onBackupStart('backup_start'),
  onBackupSucceed('backup_succeed'),
  onBackupFailed('backup_failed'),
  onRestoreProgress('restore_progress'),
  onBackupProgress('backup_progress'),
  onRestoreNetworkSpeed('restore_network_speed'),
  onBackupNetworkSpeed('backup_network_speed'),
  onBackupRequestReceived('backup_request_received'),
  onRestoreRequestReceived('restore_request_received'),
  onConnectionFailed('connection_failed');

  const DeviceTransferCallbackType(this.wireValue);

  final String wireValue;
}

enum ConnectionFailedReason { versionNotMatched, unknown }

class DeviceTransferCallbackEvent {
  const DeviceTransferCallbackEvent(this.action, [this.payload]);

  factory DeviceTransferCallbackEvent.fromJson(String source) {
    final json = jsonDecode(source) as Map<String, dynamic>;
    final kind = json['kind'] as String;
    final action = DeviceTransferCallbackType.values.firstWhere(
      (value) => value.wireValue == kind,
    );
    Object? payload = json['value'];
    if (action == DeviceTransferCallbackType.onConnectionFailed) {
      payload = switch (json['reason']) {
        'version_not_matched' => ConnectionFailedReason.versionNotMatched,
        _ => ConnectionFailedReason.unknown,
      };
    }
    return DeviceTransferCallbackEvent(action, payload);
  }

  final DeviceTransferCallbackType action;
  final Object? payload;
}

class DeviceTransferController {
  DeviceTransferController(this.account) {
    _subscription = account.deviceTransferEvents().listen(
      (event) => _events.add(DeviceTransferCallbackEvent.fromJson(event)),
      onError: _events.addError,
    );
  }

  final AccountHandle account;
  final _events = StreamController<DeviceTransferCallbackEvent>.broadcast();
  late final StreamSubscription<String> _subscription;

  Stream<DeviceTransferCallbackEvent> get events => _events.stream;

  Stream<DeviceTransferCallbackEvent> on(DeviceTransferCallbackType type) =>
      events.where((event) => event.action == type);

  Future<void> command(DeviceTransferCommand command) =>
      account.deviceTransferCommand(command: command.wireValue);

  Future<void> dispose() async {
    await _subscription.cancel();
    await _events.close();
  }
}
