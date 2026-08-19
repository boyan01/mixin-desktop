import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import {
  Check,
  ChevronLeft,
  Copy,
  Database,
  LockKeyhole,
  Network,
  Plus,
  Trash2,
} from 'lucide-react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { openExternal } from '@/shared/lib/openExternal';
import { AlertDialogLayout, MixinButton, MixinDialog } from '@/shared/ui/MixinDialog';
import type {
  DownloadSettings,
  McpServerStatus,
  McpSettings,
  Profile,
  ProxyItem,
  ProxySettings,
  ConversationStorageUsage,
  StorageCategoryUsage,
  UiSettings,
} from '@/shared/model/types';
import { useDesktopUi } from '@/state/desktopUi';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { IconButton } from '@/shared/ui/IconButton';
import { SettingsCellGroup, SettingsCellItem } from '@/features/settings/components/SettingsCells';
import { LogsDialog } from '@/features/diagnostics/components/LogsDialog';
import { UnboundedSlider } from '@/shared/ui/UnboundedSlider';
import type { SecurityState } from '@/shared/model/types';
import profileAsset from '@flutter-assets/images/ic_profile.svg';
import notificationAsset from '@flutter-assets/images/ic_notification.svg';
import storageAsset from '@flutter-assets/images/ic_storage_usage.svg';
import shieldAsset from '@flutter-assets/images/shield.svg';
import proxyAsset from '@flutter-assets/images/proxy.svg';
import appearanceAsset from '@flutter-assets/images/ic_appearance.svg';
import aboutAsset from '@flutter-assets/images/ic_about.svg';
import signOutAsset from '@flutter-assets/images/ic_sign_out.svg';
import warningAsset from '@flutter-assets/images/triangle_warning.svg';
import { classes } from '@/shared/styles/classes';

type SettingsTab = 'profile' | 'notifications' | 'downloads' | 'security' | 'proxy' | 'appearance' | 'mcp' | 'about';

interface SettingsProps {
  profile: Profile;
  onProfileUpdated: (profile: Profile) => void;
  onSignedOut: () => void;
  security: SecurityState;
}

const navigation: Array<{ tab: SettingsTab; label: string; asset: string }> = [
  { tab: 'profile', label: 'Edit Profile', asset: profileAsset },
  { tab: 'notifications', label: 'Notifications', asset: notificationAsset },
  { tab: 'downloads', label: 'Data and Storage Usage', asset: storageAsset },
  { tab: 'security', label: 'Security', asset: shieldAsset },
  { tab: 'proxy', label: 'Proxy', asset: proxyAsset },
  { tab: 'appearance', label: 'Appearance', asset: appearanceAsset },
  { tab: 'mcp', label: 'Local MCP Server', asset: '' },
  { tab: 'about', label: 'About', asset: aboutAsset },
];

const detailTitles: Record<SettingsTab, string> = {
  profile: 'Edit Profile', notifications: 'Notifications', downloads: 'Data and Storage Usage', security: 'Security', proxy: 'Proxy', appearance: 'Appearance', mcp: 'Local MCP Server', about: 'About',
};

export function Settings({ profile, onProfileUpdated, onSignedOut, security }: SettingsProps) {
  const pageRef = useRef<HTMLElement>(null);
  const [wideMode, setWideMode] = useState(false);
  const [mobileDetailOpen, setMobileDetailOpen] = useState(false);
  const [notificationGranted, setNotificationGranted] = useState<boolean>();
  const [tab, setTab] = useState<SettingsTab>('profile');
  const [fullName, setFullName] = useState(profile.fullName);
  const [biography, setBiography] = useState(profile.biography);
  const [accountProfile, setAccountProfile] = useState(profile);
  const [downloads, setDownloads] = useState<DownloadSettings>();
  const [proxy, setProxy] = useState<ProxySettings>();
  const [mcp, setMcp] = useState<McpSettings>();
  const [mcpStatus, setMcpStatus] = useState<McpServerStatus>();
  const [version, setVersion] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const [confirmSignOut, setConfirmSignOut] = useState(false);
  const uiSettings = useDesktopUi((state) => state.uiSettings);
  const setUiSettings = useDesktopUi((state) => state.setUiSettings);
  const openDeviceTransfer = useDesktopUi((state) => state.openDeviceTransfer);

  useEffect(() => {
    const updateWidth = () => {
      const width = pageRef.current?.getBoundingClientRect().width ?? window.innerWidth;
      setWideMode(width >= 620);
    };
    updateWidth();
    window.addEventListener('resize', updateWidth);
    const page = pageRef.current;
    const observer = typeof ResizeObserver === 'undefined' || !page ? undefined : new ResizeObserver(updateWidth);
    if (observer && page) observer.observe(page);
    return () => {
      window.removeEventListener('resize', updateWidth);
      observer?.disconnect();
    };
  }, []);

  useEffect(() => {
    let active = true;
    const refresh = () => {
      void isPermissionGranted().then(async (granted) => granted || (await requestPermission()) === 'granted').then((granted) => {
        if (active) setNotificationGranted(granted);
      }).catch((reason) => {
        e('Unable to read notification permission', reason);
        if (active) setNotificationGranted(false);
      });
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === 'visible') refresh();
    };
    refresh();
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      active = false;
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, []);

  useEffect(() => {
    if (wideMode) setMobileDetailOpen(false);
  }, [wideMode]);

  function selectTab(next: SettingsTab) {
    setTab(next);
    if (!wideMode) setMobileDetailOpen(true);
  }

  useEffect(() => {
    void Promise.all([
      desktopApi.downloadSettings().then(setDownloads),
      desktopApi.proxySettings().then(setProxy),
      Promise.all([desktopApi.mcpSettings(), desktopApi.mcpServerStatus()]).then(([settings, status]) => {
        setMcp(settings);
        setMcpStatus(status);
      }),
      desktopApi.appVersion().then(setVersion),
    ]).catch((reason: unknown) => { e('Unable to load settings', reason); setError(commandMessage(reason, 'Unable to load settings')); });
  }, []);

  useEffect(() => {
    setAccountProfile(profile); setFullName(profile.fullName); setBiography(profile.biography);
  }, [profile]);

  useEffect(() => {
    void desktopApi.refreshProfile().then((next) => { setAccountProfile(next); setFullName(next.fullName); setBiography(next.biography); onProfileUpdated(next); }).catch((reason) => e('Unable to refresh profile', reason));
  // The remote profile refresh mirrors Flutter's one-time page initialization.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function saveProfile(event: FormEvent) {
    event.preventDefault();
    if (!fullName.trim() || saving) return;
    await runUpdate(async () => { const next = await desktopApi.updateProfile(fullName, biography); setAccountProfile(next); onProfileUpdated(next); });
  }

  async function runUpdate(action: () => Promise<void>) {
    if (saving) return;
    setSaving(true);
    setError(undefined);
    try {
      await action();
    } catch (reason) {
      e('Unable to update settings', reason);
      setError(commandMessage(reason, 'Unable to update settings'));
    } finally {
      setSaving(false);
    }
  }

  async function updateDownloads(kind: keyof DownloadSettings) {
    if (!downloads) return;
    const previous = downloads;
    const next = { ...downloads, [kind]: !downloads[kind] };
    setDownloads(next);
    try {
      await desktopApi.setDownloadSetting(kind, next[kind]);
    } catch (reason) {
      setDownloads(previous);
      e('Unable to update download settings', reason);
      setError(commandMessage(reason, 'Unable to update download settings'));
    }
  }

  async function updateProxy(next: ProxySettings) {
    const previous = proxy;
    setProxy(next);
    try {
      await desktopApi.setProxySettings(next);
    } catch (reason) {
      setProxy(previous);
      e('Unable to update proxy settings', reason);
      setError(commandMessage(reason, 'Unable to update proxy settings'));
    }
  }

  async function updateMcp(next: McpSettings) {
    await runUpdate(async () => {
      setMcpStatus(await desktopApi.updateMcpSettings(next));
      setMcp(next);
    });
  }

  async function signOut() {
    await runUpdate(async () => {
      await desktopApi.signOut();
      onSignedOut();
    });
  }

  async function updateUiSetting<K extends keyof UiSettings>(key: K, value: UiSettings[K]) {
    const previous = uiSettings[key];
    setUiSettings({ [key]: value });
    const storageKey = {
      theme: 'brightness', showAvatar: 'messageShowAvatar', showIdentityNumber: 'messageShowIdentityNumber',
      messagePreview: 'messagePreview', chatFontSizeDelta: 'chatFontSizeDelta',
    }[key];
    const storedValue = key === 'theme' ? (value === 'dark' ? 1 : value === 'light' ? 2 : 0) : value;
    try {
      await desktopApi.setSetting(storageKey, JSON.stringify(storedValue));
    } catch (reason) {
      setUiSettings({ [key]: previous });
      e('Unable to update appearance', reason);
      setError(commandMessage(reason, 'Unable to update appearance'));
    }
  }

  return (
    <section ref={pageRef} className={classes("settings-page")}>
      <div className={classes(`settings-layout ${wideMode ? '' : mobileDetailOpen ? 'settings-mobile-detail' : 'settings-mobile-home'}`)}>
        {(wideMode || !mobileDetailOpen) && (
        <nav className={classes("settings-nav")}>
          <Avatar name={profile.fullName} url={profile.avatarUrl} size={90} />
          <h1>{profile.fullName}<Badges verified={profile.isVerified} isBot={false} membership={profile.membership} /></h1><small>Mixin ID: {profile.identityNumber}</small>
          <SettingsCellGroup className={classes("settings-nav-group")}>
            <SettingsCellItem leading={<SettingsAssetIcon src={navigation[0].asset} />} title={navigation[0].label} selected={wideMode && tab === 'profile'} onClick={() => selectTab('profile')} />
          </SettingsCellGroup>
          <SettingsCellGroup className={classes("settings-nav-group")}>
            {navigation.slice(1).map((item) => {
              const notificationProblem = item.tab === 'notifications' && notificationGranted === false;
              return <SettingsCellItem key={item.tab} leading={item.asset ? <SettingsAssetIcon src={item.asset} color={notificationProblem ? 'var(--red)' : undefined} /> : <Network size={24} />} title={item.label} color={notificationProblem ? 'var(--red)' : undefined} trailing={notificationProblem ? <span className={classes("settings-warning-icon")}><SettingsAssetIcon src={warningAsset} color="var(--red)" /></span> : undefined} selected={wideMode && tab === item.tab} onClick={() => selectTab(item.tab)} />;
            })}
          </SettingsCellGroup>
          <SettingsCellGroup className={classes("settings-nav-group")}>
            <SettingsCellItem leading={<SettingsAssetIcon src={signOutAsset} />} title="Sign Out" destructive trailing={null} disabled={saving} onClick={() => setConfirmSignOut(true)} />
          </SettingsCellGroup>
        </nav>
        )}
        {(wideMode || mobileDetailOpen) && (
        <main className={classes("settings-detail")}>
          <header className={classes("settings-detail-header")}>{wideMode ? <span /> : <IconButton label="Back to settings" onClick={() => setMobileDetailOpen(false)}><ChevronLeft size={20} /></IconButton>}<strong>{detailTitles[tab]}</strong>{tab === 'profile' ? <button className={classes("settings-save-button")} form="settings-profile-form" type="submit" disabled={!fullName.trim() || saving}>Save</button> : <span />}</header>
          {tab === 'profile' && (
            <form id="settings-profile-form" className={classes("profile-form")} onSubmit={saveProfile}>
              <Avatar name={accountProfile.fullName} url={accountProfile.avatarUrl} size={100} />
              <small className={classes("profile-form-id")}>Mixin ID: {accountProfile.identityNumber}</small>
              <ProfileField label="Name" value={fullName} maxLength={40} onChange={setFullName} />
              <ProfileField label="Biography" value={biography} maxLength={140} multiline onChange={setBiography} />
              <ProfileField label="Phone Number" value={accountProfile.phone ?? ''} readOnly onChange={() => undefined} />
              {accountProfile.createdAt && <small className={classes("profile-form-joined")}>Joined {new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(new Date(accountProfile.createdAt))}</small>}
            </form>
          )}
          {tab === 'notifications' && <NotificationPanel settings={uiSettings} onChange={updateUiSetting} />}
          {tab === 'downloads' && <StoragePanel settings={downloads} onToggle={updateDownloads} />}
          {tab === 'security' && <SecurityPanel security={security} />}
          {tab === 'proxy' && proxy && <ProxyPanel settings={proxy} onChange={updateProxy} />}
          {tab === 'appearance' && <AppearancePanel settings={uiSettings} onChange={updateUiSetting} />}
          {tab === 'mcp' && mcp && <McpPanel settings={mcp} status={mcpStatus} disabled={saving} onChange={updateMcp} />}
          {tab === 'about' && <AboutPanel version={version} onDeviceTransfer={openDeviceTransfer} />}
          {error && <p className={classes("dialog-error")}>{error}</p>}
        </main>
        )}
      </div>
      {confirmSignOut && <SettingsConfirmation title="Sign Out" description="Sign out of Mixin Messenger on this device?" confirmLabel="Sign Out" destructive onClose={() => setConfirmSignOut(false)} onConfirm={() => void signOut()} />}
    </section>
  );
}

function StoragePanel({ settings, onToggle }: { settings?: DownloadSettings; onToggle: (kind: keyof DownloadSettings) => Promise<void> }) {
  const [usage, setUsage] = useState<ConversationStorageUsage[]>();
  const [selected, setSelected] = useState<ConversationStorageUsage>();
  const [categories, setCategories] = useState<StorageCategoryUsage[]>();
  const [checked, setChecked] = useState<string[]>([]);
  const [confirmClear, setConfirmClear] = useState(false);

  async function openUsage() {
    setUsage(await desktopApi.storageUsage());
  }

  async function openConversation(item: ConversationStorageUsage) {
    setSelected(item);
    setCategories(await desktopApi.conversationStorageUsage(item.conversation.conversationId));
    setChecked([]);
  }

  async function clearSelected() {
    if (!selected || checked.length === 0) return;
    await desktopApi.clearConversationStorage(selected.conversation.conversationId, checked);
    setCategories(await desktopApi.conversationStorageUsage(selected.conversation.conversationId));
    setUsage(await desktopApi.storageUsage());
    setChecked([]);
  }

  if (selected && categories) return (
    <section className={classes("settings-preference-page storage-usage")}>
      <div className={classes("settings-subheader")}><button type="button" onClick={() => setSelected(undefined)}>‹ Back</button><h2>{selected.conversation.name}</h2><button type="button" disabled={checked.length === 0} onClick={() => setConfirmClear(true)}>Clear</button></div>
      {categories.map((item) => <label key={item.category}><input type="checkbox" checked={checked.includes(item.category)} onChange={() => setChecked((current) => current.includes(item.category) ? current.filter((value) => value !== item.category) : [...current, item.category])} /><span>{storageLabel(item.category)}</span><small>{formatBytes(item.sizeBytes)}</small></label>)}
      {confirmClear && <SettingsConfirmation title="Clear Cached Media" description="Clear the selected cached media from this device?" confirmLabel="Clear" destructive onClose={() => setConfirmClear(false)} onConfirm={() => { setConfirmClear(false); void clearSelected(); }} />}
    </section>
  );

  if (usage) return (
    <section className={classes("settings-preference-page storage-usage")}>
      <div className={classes("settings-subheader")}><button type="button" onClick={() => setUsage(undefined)}>‹ Back</button><h2>Storage Usage</h2><span /></div>
      {usage.map((item) => <button className={classes("storage-row")} key={item.conversation.conversationId} type="button" onClick={() => void openConversation(item)}><Avatar name={item.conversation.name} url={item.conversation.avatarUrl} size={44} /><span><strong>{item.conversation.name}</strong><small>{formatBytes(item.sizeBytes)}</small></span><b>›</b></button>)}
      {usage.length === 0 && <p className={classes("empty-list")}>No cached media</p>}
    </section>
  );

  return (
    <div className={classes("settings-stack settings-preference-page")}>
      <SettingsSection title="Automatic Media Download">
        {(['photo', 'video', 'file'] as const).map((kind) => <SettingsToggleCell key={kind} label={`${kind[0].toUpperCase() + kind.slice(1)}s`} checked={settings?.[kind] ?? false} disabled={!settings} onChange={() => void onToggle(kind)} />)}
      </SettingsSection>
      <p className={classes("settings-note")}>Automatically download media from incoming messages.</p>
      <SettingsCellGroup><SettingsCellItem leading={<Database size={18} />} title="Storage Usage" onClick={() => void openUsage()} /></SettingsCellGroup>
    </div>
  );
}

function SettingsConfirmation({ title, description, confirmLabel, destructive = false, onClose, onConfirm }: { title: string; description: string; confirmLabel: string; destructive?: boolean; onClose: () => void; onConfirm: () => void }) {
  return <MixinDialog open onClose={onClose} maxWidth={400}><AlertDialogLayout title={title} content={description} actions={<><MixinButton transparent onClick={onClose}>Cancel</MixinButton><MixinButton className={classes(destructive ? 'is-danger' : '')} onClick={onConfirm}>{confirmLabel}</MixinButton></>} /></MixinDialog>;
}

function NotificationPanel({ settings, onChange }: { settings: UiSettings; onChange: <K extends keyof UiSettings>(key: K, value: UiSettings[K]) => Promise<void> }) {
  const [granted, setGranted] = useState<boolean>();
  useEffect(() => {
    let active = true;
    void isPermissionGranted().then(async (value) => value || (await requestPermission()) === 'granted').then((value) => { if (active) setGranted(value); }).catch((reason) => { e('Unable to read notification permission', reason); if (active) setGranted(false); });
    return () => { active = false; };
  }, []);
  const macOS = navigator.platform.toLowerCase().includes('mac');
  return <div className={classes("settings-stack settings-preference-page")}><SettingsSection title="Notifications"><SettingsToggleCell label="Message Preview" checked={settings.messagePreview} onChange={(value) => void onChange('messagePreview', value)} /></SettingsSection><p className={classes("settings-note")}>Show sender and message content in system notifications.</p>{granted === false && <>{macOS && <SettingsCellGroup><SettingsCellItem title="Enable Push Notifications" onClick={() => openExternal('x-apple.systempreferences:com.apple.preference.notifications')} /></SettingsCellGroup>}<p className={classes("settings-note")}>Enable notifications to receive message content and alerts.</p></>}</div>;
}

function SecurityPanel({ security }: { security: SecurityState }) {
  const [firstPasscode, setFirstPasscode] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [setting, setSetting] = useState(false);
  const [error, setError] = useState<string>();

  async function enable(event: FormEvent) {
    event.preventDefault();
    if (!/^\d{6}$/.test(confirmation)) {
      setError('Enter a 6-digit passcode.');
      return;
    }
    if (!firstPasscode) {
      setFirstPasscode(confirmation); setConfirmation(''); setError(undefined);
      return;
    }
    if (firstPasscode !== confirmation) {
      setFirstPasscode(''); setConfirmation(''); setError('Passcodes do not match.');
      return;
    }
    await security.setPasscode(confirmation);
    setFirstPasscode(''); setConfirmation(''); setSetting(false); setError(undefined);
  }

  return (
    <div className={classes("settings-stack")}>
      <section className={classes("settings-card setting-list")}><h2>Security</h2><ToggleRow label="Screen Passcode" checked={security.hasPasscode} disabled={!security.ready} onChange={(enabled) => { if (enabled) setSetting(true); else void security.setPasscode(null); }} />
        {security.hasPasscode && <label><span>Auto Lock</span><select value={security.autoLockMinutes} onChange={(event) => void security.setAutoLockMinutes(Number(event.target.value))}><option value={0}>Never</option><option value={1}>After 1 minute</option><option value={5}>After 5 minutes</option><option value={60}>After 1 hour</option><option value={300}>After 5 hours</option></select></label>}
        {security.hasPasscode && <ToggleRow label="Biometric" checked={security.biometricEnabled} disabled={!security.ready} onChange={(enabled) => void security.setBiometric(enabled).then((accepted) => { if (!accepted) setError('This device does not support biometric authentication.'); })} />}
      </section>
      {error && !setting && <p className={classes("settings-note dialog-error")}>{error}</p>}
      {setting && <form className={classes("settings-card passcode-form")} onSubmit={enable}><h2>{firstPasscode ? 'Confirm Passcode' : 'Set Passcode'}</h2><p>{firstPasscode ? 'Enter the same 6-digit passcode again.' : 'Enter a 6-digit passcode.'}</p><input autoFocus type="password" inputMode="numeric" autoComplete="one-time-code" maxLength={6} value={confirmation} placeholder="••••••" onChange={(event) => { setConfirmation(event.target.value.replace(/\D/g, '')); setError(undefined); }} />{error && <p className={classes("dialog-error")}>{error}</p>}<div><button type="button" onClick={() => { setFirstPasscode(''); setConfirmation(''); setSetting(false); }}>Cancel</button><button className={classes("primary-button")} disabled={confirmation.length !== 6} type="submit">{firstPasscode ? 'Confirm' : 'Next'}</button></div></form>}
      {security.hasPasscode && <SettingsCellGroup><SettingsCellItem leading={<LockKeyhole size={18} />} title="Lock Now" trailing={null} onClick={security.lockNow} /></SettingsCellGroup>}
    </div>
  );
}

function AppearancePanel({ settings, onChange }: { settings: UiSettings; onChange: <K extends keyof UiSettings>(key: K, value: UiSettings[K]) => Promise<void> }) {
  return (
    <div className={classes("settings-stack settings-preference-page")}>
      <SettingsSection title="Theme">{(['system', 'light', 'dark'] as const).map((theme) => <SettingsRadioCell key={theme} name="theme" label={theme === 'system' ? 'Follow System' : theme[0].toUpperCase() + theme.slice(1)} checked={settings.theme === theme} onChange={() => void onChange('theme', theme)} />)}</SettingsSection>
      <SettingsSection title="Chats"><SettingsToggleCell label="Show Avatar" checked={settings.showAvatar} onChange={(value) => void onChange('showAvatar', value)} /><SettingsToggleCell label="Show Identity Number" checked={settings.showIdentityNumber} onChange={(value) => void onChange('showIdentityNumber', value)} /></SettingsSection>
      <section className={classes("chat-font-setting")}><h2 className={classes("settings-section-title")}>Chat Text Size</h2><div className={classes("chat-font-preview")} style={{ fontSize: `${16 + settings.chatFontSizeDelta}px` }}>The quick brown fox</div><label><small>A</small><UnboundedSlider value={settings.chatFontSizeDelta} min={-2} max={4} step={1} ariaLabel="Chat text size" onChange={(value) => void onChange('chatFontSizeDelta', value)} /><strong>A</strong></label></section>
    </div>
  );
}

function ProxyPanel({ settings, onChange }: { settings: ProxySettings; onChange: (settings: ProxySettings) => Promise<void> }) {
  const [adding, setAdding] = useState(false);

  return (
    <div className={classes("settings-stack settings-preference-page")}>
      <SettingsSection title="Proxy"><SettingsToggleCell label="Proxy" checked={settings.enabled && settings.proxies.length > 0} disabled={settings.proxies.length === 0} onChange={(enabled) => void onChange({ ...settings, enabled })} /></SettingsSection>
      <SettingsCellGroup className={classes("proxy-list")}>
        <SettingsCellItem leading={<Plus size={18} />} title="Add Proxy" trailing={null} onClick={() => setAdding(true)} />
        {settings.proxies.map((item) => {
          const selected = (settings.selectedProxyId ?? settings.proxies[0]?.id) === item.id;
          return (
            <div className={classes("proxy-row")} key={item.id}>
              <button type="button" onClick={() => void onChange({ ...settings, selectedProxyId: item.id })}>
                <span className={classes("proxy-check")}>{selected && <Check size={18} />}</span><span><strong>{item.host}:{item.port}</strong><small>{item.kind}</small></span>
              </button>
              <IconButton label="Delete proxy" onClick={() => {
                const proxies = settings.proxies.filter((proxy) => proxy.id !== item.id);
                void onChange({ ...settings, enabled: proxies.length > 0 && settings.enabled, selectedProxyId: selected ? proxies[0]?.id : settings.selectedProxyId, proxies });
              }}><Trash2 size={17} /></IconButton>
            </div>
          );
        })}
      </SettingsCellGroup>
      {adding && <ProxyDialog onClose={() => setAdding(false)} onAdd={(item) => { void onChange({ ...settings, selectedProxyId: settings.selectedProxyId ?? item.id, proxies: [...settings.proxies, item] }); setAdding(false); }} />}
    </div>
  );
}

function ProxyDialog({ onClose, onAdd }: { onClose: () => void; onAdd: (item: ProxyItem) => void }) {
  const [host, setHost] = useState('');
  const [port, setPort] = useState('');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    const parsedPort = Number(port);
    if (!host.trim() || !Number.isInteger(parsedPort) || parsedPort < 1 || parsedPort > 65535) return;
    onAdd({ id: crypto.randomUUID(), kind: 'http', host: host.trim(), port: parsedPort, username: username.trim() || null, password: password || null });
  }
  return <div className={classes("modal-backdrop")}><form className={classes("circle-dialog proxy-dialog")} onSubmit={submit}><header><strong>Add Proxy</strong></header><div><label>Proxy Type</label><button type="button" className={classes("proxy-type")} aria-pressed="true">HTTP <Check size={18} /></button><label>Proxy Connection</label><input autoFocus placeholder="Host" value={host} maxLength={200} onChange={(event) => setHost(event.target.value)} /><input placeholder="Port" inputMode="numeric" value={port} maxLength={5} onChange={(event) => setPort(event.target.value.replace(/\D/g, ''))} /><label>Proxy Authentication</label><input placeholder="Username" value={username} maxLength={200} onChange={(event) => setUsername(event.target.value)} /><input placeholder="Password" type="password" value={password} maxLength={200} onChange={(event) => setPassword(event.target.value)} /></div><footer><button type="button" onClick={onClose}>Cancel</button><button type="submit" disabled={!host.trim() || !port}>Add</button></footer></form></div>;
}

function McpPanel({ settings, status, disabled, onChange }: { settings: McpSettings; status?: McpServerStatus; disabled: boolean; onChange: (settings: McpSettings) => Promise<void> }) {
  const masked = settings.token.length < 8 ? '••••••••' : `${settings.token.slice(0, 4)}••••${settings.token.slice(-4)}`;
  const copy = (value?: string | null) => value && void navigator.clipboard.writeText(value);
  return (
    <div className={classes("settings-stack")}>
      <p className={classes("settings-status")}>{status?.running ? 'Running on localhost' : status?.lastError ?? 'Stopped'}</p>
      <section className={classes("settings-card setting-list")}>
        <h2>Local MCP Server</h2>
        <ToggleRow label="Server" checked={settings.enabled} disabled={disabled} onChange={(enabled) => void onChange({ ...settings, enabled })} />
        {settings.enabled && <>
          <ValueRow label="Endpoint" value={status?.endpoint ?? 'Not running'} onCopy={() => copy(status?.endpoint)} />
          <ValueRow label="Access Token" value={masked} onCopy={() => copy(settings.token)} />
          <ToggleRow label="Draft Editing" checked={settings.draftToolsEnabled} disabled={disabled} onChange={(draftToolsEnabled) => void onChange({ ...settings, draftToolsEnabled })} />
          <ToggleRow label="Circle Management" checked={settings.circleManagementEnabled} disabled={disabled} onChange={(circleManagementEnabled) => void onChange({ ...settings, circleManagementEnabled })} />
        </>}
      </section>
      <p className={classes("settings-note")}>Local clients must use the bearer token. MCP never sends messages or changes account data.</p>
    </div>
  );
}

function ToggleRow({ label, checked, disabled, onChange }: { label: string; checked: boolean; disabled: boolean; onChange: (checked: boolean) => void }) {
  return <label><span>{label}</span><input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => onChange(event.target.checked)} /></label>;
}

function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return <section className={classes("settings-section")}><h2 className={classes("settings-section-title")}>{title}</h2><SettingsCellGroup>{children}</SettingsCellGroup></section>;
}

function SettingsToggleCell({ label, checked, disabled = false, onChange }: { label: string; checked: boolean; disabled?: boolean; onChange: (checked: boolean) => void }) {
  return <label className={classes("settings-toggle-cell")}><span>{label}</span><input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => onChange(event.target.checked)} /></label>;
}

function SettingsAssetIcon({ src, color }: { src: string; color?: string }) {
  return <span className={classes("settings-asset-icon")} aria-hidden="true" style={{ '--settings-icon-image': `url("${src}")`, ...(color ? { color } : {}) } as React.CSSProperties} />;
}

function ProfileField({ label, value, maxLength, multiline = false, readOnly = false, onChange }: { label: string; value: string; maxLength?: number; multiline?: boolean; readOnly?: boolean; onChange: (value: string) => void }) {
  return <label className={classes("profile-field")}>
    <span>{label}</span>
    {multiline
      ? <textarea rows={1} maxLength={maxLength} readOnly={readOnly} value={value} onChange={(event) => onChange(event.target.value)} />
      : <input maxLength={maxLength} readOnly={readOnly} value={value} onChange={(event) => onChange(event.target.value)} />}
    {maxLength !== undefined && <small className={classes("profile-field-counter")}>{value.length}/{maxLength}</small>}
  </label>;
}

function SettingsRadioCell({ name, label, checked, onChange }: { name: string; label: string; checked: boolean; onChange: () => void }) {
  return <label className={classes("settings-radio-cell")}><input type="radio" name={name} checked={checked} onChange={onChange} /><span>{label}</span></label>;
}

function ValueRow({ label, value, onCopy }: { label: string; value: string; onCopy: () => void }) {
  return <div className={classes("setting-value-row")}><span><strong>{label}</strong><small>{value}</small></span><IconButton label={`Copy ${label}`} onClick={onCopy}><Copy size={17} /></IconButton></div>;
}

function AboutPanel({ version, onDeviceTransfer }: { version: string; onDeviceTransfer: () => void }) {
  const [logsOpen, setLogsOpen] = useState(false);
  const [logoTaps, setLogoTaps] = useState(0);
  const lastLogoTap = useRef(0);
  const links = [
    ['Follow us on X', 'https://x.com/MixinMessenger'],
    ['Follow us on Facebook', 'https://fb.com/MixinMessenger'],
    ['Help Center', 'https://support.mixin.one'],
    ['Terms of Service', 'https://mixin.one/pages/terms'],
    ['Privacy Policy', 'https://mixin.one/pages/privacy'],
  ];
  return (
    <div className={classes("about-panel")}>
      <button className={classes("about-logo")} type="button" aria-label="Mixin Messenger" onClick={() => { const now = Date.now(); const next = now - lastLogoTap.current < 1_000 ? logoTaps + 1 : 1; lastLogoTap.current = now; if (next >= 5) { setLogoTaps(0); setLogsOpen(true); } else setLogoTaps(next); }}>M</button>
      <h2>Mixin Messenger Desktop</h2><p>{version || '0.1.0'}</p>
      <section className={classes("settings-card about-links")}>
        {links.map(([label, href]) => <button type="button" key={href} onClick={() => openExternal(href)}>{label}<span>›</span></button>)}
        {import.meta.env.DEV && <button type="button" onClick={onDeviceTransfer}>Chat Backup and Restore<span>›</span></button>}
      </section>
      {logsOpen && <LogsDialog onClose={() => setLogsOpen(false)} />}
    </div>
  );
}

function commandMessage(reason: unknown, fallback: string) {
  return typeof reason === 'object' && reason !== null && 'message' in reason ? String(reason.message) : fallback;
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let size = value / 1024;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit += 1; }
  return `${size >= 10 ? size.toFixed(0) : size.toFixed(1)} ${units[unit]}`;
}

function storageLabel(category: string) {
  return ({ photos: 'Photos', videos: 'Videos', audio: 'Audio', files: 'Files' } as Record<string, string>)[category] ?? category;
}
