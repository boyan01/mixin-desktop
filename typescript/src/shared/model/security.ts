export interface SecurityState {
  ready: boolean;
  hasPasscode: boolean;
  biometricEnabled: boolean;
  biometricAvailable: boolean;
  autoLockMinutes: number;
  locked: boolean;
  setPasscode: (passcode: string | null) => Promise<void>;
  setAutoLockMinutes: (minutes: number) => Promise<void>;
  setBiometric: (enabled: boolean) => Promise<boolean>;
  verify: (passcode: string) => boolean;
  lockNow: () => void;
  unlock: () => void;
}
