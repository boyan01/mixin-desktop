export type DeviceTransferCommand =
  | 'pullToRemote'
  | 'pushToRemote'
  | 'cancelRestore'
  | 'cancelBackup'
  | 'cancelBackupRequest'
  | 'cancelRestoreRequest'
  | 'confirmRestore'
  | 'confirmBackup';
