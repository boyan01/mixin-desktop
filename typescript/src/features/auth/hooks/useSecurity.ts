import { useCallback, useEffect, useRef, useState } from 'react';
import { desktopApi } from '@/shared/api/desktopApi';
import type { SecurityState } from '@/shared/model/types';

export function useSecurity(accountId: string): SecurityState {
  const [ready, setReady] = useState(false);
  const [passcode, setPasscodeValue] = useState<string | null>(null);
  const [autoLockMinutes, setAutoLockValue] = useState(1);
  const [biometricEnabled, setBiometricEnabled] = useState(false);
  const [biometricAvailable, setBiometricAvailable] = useState(false);
  const [locked, setLocked] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  const prefix = `security.${accountId}.`;

  useEffect(() => {
    let active = true;
    void Promise.all([
      desktopApi.setting(`${prefix}passcode`),
      desktopApi.setting(`${prefix}lockDuration`),
      desktopApi.setting(`${prefix}biometric`),
      desktopApi.biometricAvailable(),
    ]).then(([storedPasscode, storedDuration, storedBiometric, available]) => {
      if (!active) return;
      const nextPasscode = storedPasscode === null ? null : JSON.parse(storedPasscode) as string;
      setPasscodeValue(nextPasscode);
      setAutoLockValue(storedDuration === null ? 1 : Number(JSON.parse(storedDuration)));
      setBiometricEnabled(available && storedBiometric !== null && Boolean(JSON.parse(storedBiometric)));
      setBiometricAvailable(available);
      setLocked(nextPasscode !== null);
      setReady(true);
    });
    return () => { active = false; };
  }, [prefix]);

  useEffect(() => {
    function cancelTimer() {
      if (timer.current !== undefined) window.clearTimeout(timer.current);
      timer.current = undefined;
    }
    function onBlur() {
      cancelTimer();
      if (!passcode || locked || autoLockMinutes === 0) return;
      timer.current = window.setTimeout(() => setLocked(true), autoLockMinutes * 60_000);
    }
    function onFocus() { cancelTimer(); }
    window.addEventListener('blur', onBlur);
    window.addEventListener('focus', onFocus);
    return () => {
      cancelTimer();
      window.removeEventListener('blur', onBlur);
      window.removeEventListener('focus', onFocus);
    };
  }, [autoLockMinutes, locked, passcode]);

  const setPasscode = useCallback(async (next: string | null) => {
    if (next !== null && !/^\d{6}$/.test(next)) throw new Error('Passcode must be 6 digits');
    await desktopApi.setSetting(`${prefix}passcode`, next === null ? null : JSON.stringify(next));
    if (next === null) {
      await desktopApi.setSetting(`${prefix}biometric`, JSON.stringify(false));
      setBiometricEnabled(false);
      await desktopApi.setSetting(`${prefix}lockDuration`, null);
      setAutoLockValue(1);
      setLocked(false);
    }
    setPasscodeValue(next);
  }, [prefix]);

  const setAutoLockMinutes = useCallback(async (minutes: number) => {
    await desktopApi.setSetting(`${prefix}lockDuration`, JSON.stringify(minutes));
    setAutoLockValue(minutes);
  }, [prefix]);

  const setBiometric = useCallback(async (enabled: boolean) => {
    if (enabled && !biometricAvailable) return false;
    await desktopApi.setSetting(`${prefix}biometric`, JSON.stringify(enabled));
    setBiometricEnabled(enabled);
    return true;
  }, [biometricAvailable, prefix]);

  return {
    ready,
    hasPasscode: passcode !== null,
    biometricEnabled,
    biometricAvailable,
    autoLockMinutes,
    locked,
    setPasscode,
    setAutoLockMinutes,
    setBiometric,
    verify: (value) => value === passcode,
    lockNow: () => passcode && setLocked(true),
    unlock: () => setLocked(false),
  };
}
