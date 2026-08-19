import { useRef, useState } from 'react';
import { Fingerprint, LockKeyhole } from 'lucide-react';
import { desktopApi } from '@/shared/api/desktopApi';
import type { SecurityState } from '@/shared/model/types';
import { classes } from '@/shared/styles/classes';

export function AuthGuard({ security }: { security: SecurityState }) {
  const [value, setValue] = useState('');
  const [incorrect, setIncorrect] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  if (!security.ready || !security.locked || !security.hasPasscode) return null;

  function submit(next: string) {
    if (next.length !== 6) return;
    if (security.verify(next)) {
      setIncorrect(false);
      setValue('');
      security.unlock();
    } else {
      setIncorrect(true);
      setValue('');
    }
  }

  async function unlockWithBiometrics() {
    if (await desktopApi.authenticateBiometric()) {
      setIncorrect(false);
      setValue('');
      security.unlock();
    }
  }

  return (
    <div className={classes("auth-guard")} onClick={() => inputRef.current?.focus()}>
      <LockKeyhole size={68} strokeWidth={1.4} />
      <h1>Unlock with Passcode</h1>
      <div className={classes("passcode-dots")}>{Array.from({ length: 6 }, (_, index) => <i className={classes(index < value.length ? 'is-filled' : '')} key={index} />)}</div>
      <input ref={inputRef} autoFocus aria-label="Passcode" inputMode="numeric" value={value} maxLength={6} onChange={(event) => { const next = event.target.value.replace(/\D/g, ''); setValue(next); setIncorrect(false); submit(next); }} />
      <p className={classes(incorrect ? 'is-visible' : '')}>Passcode incorrect</p>
      {security.biometricEnabled && <button type="button" className={classes("biometric-unlock")} onClick={() => void unlockWithBiometrics()}><Fingerprint size={20} /> Use Biometric</button>}
    </div>
  );
}
