import { useEffect, useState, type FormEvent } from 'react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { openExternal } from '@/shared/lib/openExternal';
import type { Profile } from '@/shared/model/types';
import { classes } from '@/shared/styles/classes';

export function ProfileSetupOverlay({ biography, onSaved }: { biography: string; onSaved: (profile: Profile) => void }) {
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const fullName = name.trim();
    if (!fullName || saving) return;
    setSaving(true);
    setError(undefined);
    try {
      onSaved(await desktopApi.updateProfile(fullName, biography));
    } catch (reason) {
      e('Unable to update profile name', reason);
      setError(reason instanceof Error ? reason.message : 'Unable to update your name');
    } finally {
      setSaving(false);
    }
  }

  return <div className={classes("profile-setup-overlay")} role="dialog" aria-modal="true" aria-labelledby="profile-setup-title"><form className={classes("profile-setup-dialog")} onSubmit={(event) => void save(event)}><h1 id="profile-setup-title">Edit Name</h1><input autoFocus value={name} maxLength={40} onChange={(event) => setName(event.target.value)} placeholder="Name" aria-label="Name" />{error && <p role="alert">{error}</p>}<footer><button className={classes("primary-button")} type="submit" disabled={!name.trim() || saving}>{saving ? 'Saving…' : 'Confirm'}</button></footer></form></div>;
}

export function AccountHealthOverlay({ health }: { health: 'time_inaccurate' | 'update_required' }) {
  const [refreshing, setRefreshing] = useState(false);
  const [version, setVersion] = useState('');
  useEffect(() => {
    if (health === 'update_required') void desktopApi.appVersion().then(setVersion).catch((reason) => e('Unable to read app version', reason));
  }, [health]);

  async function refresh() {
    if (refreshing) return;
    setRefreshing(true);
    try { await desktopApi.refreshAccountHealth(); }
    catch (reason) { e('Unable to refresh account health', reason); }
    finally { setRefreshing(false); }
  }

  if (health === 'time_inaccurate') {
    return <div className={classes("account-health-overlay")}><div><h1>Loading Time</h1>{refreshing ? <span className={classes("account-health-spinner")} aria-label="Loading" /> : <button className={classes("primary-button")} type="button" onClick={() => void refresh()}>Continue</button>}</div></div>;
  }
  return <div className={classes("account-health-overlay")}><div><h1>Update Mixin</h1><p>Please upgrade Mixin Messenger{version ? ` (${version})` : ''} to continue.</p><button className={classes("primary-button")} type="button" onClick={() => void openExternal('https://mixin.one/messenger')}>Upgrade</button><small>{version}</small></div></div>;
}
