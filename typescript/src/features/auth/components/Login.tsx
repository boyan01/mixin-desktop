import { useCallback, useEffect, useRef, useState } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { LoaderCircle, RotateCw } from 'lucide-react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { BootstrapData, CommandError, LoginSession } from '@/shared/model/types';
import { LogsDialog } from '@/features/diagnostics/components/LogsDialog';
import mixinLogoUrl from '@flutter-assets/images/logo.png';
import { classes } from '@/shared/styles/classes';

type LoginStatus = 'loading' | 'ready' | 'provisioning' | 'failed';

interface LoginProps {
  onAuthenticated: (bootstrap: BootstrapData) => void;
}

function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'message' in error) {
    return String((error as CommandError).message);
  }
  return String(error);
}

export function Login({ onAuthenticated }: LoginProps) {
  const [status, setStatus] = useState<LoginStatus>('loading');
  const [session, setSession] = useState<LoginSession>();
  const [error, setError] = useState<string>();
  const [version, setVersion] = useState('');
  const versionClicks = useRef(0);
  const [logsOpen, setLogsOpen] = useState(false);
  const requestVersion = useRef(0);
  const mounted = useRef(true);
  const activeSession = useRef<LoginSession | undefined>(undefined);

  const refresh = useCallback(async () => {
    const version = ++requestVersion.current;
    const previous = activeSession.current;
    activeSession.current = undefined;
    setSession(undefined);
    setError(undefined);
    setStatus('loading');
    if (previous) void desktopApi.cancelLogin(previous.requestId);

    try {
      const next = await desktopApi.beginLogin();
      if (!mounted.current || version !== requestVersion.current) {
        void desktopApi.cancelLogin(next.requestId);
        return;
      }
      activeSession.current = next;
      setSession(next);
      setStatus('ready');
      try {
        const bootstrap = await desktopApi.waitLogin(next.requestId);
        if (!mounted.current || version !== requestVersion.current) return;
        activeSession.current = undefined;
        setStatus('provisioning');
        onAuthenticated(bootstrap);
      } catch (waitError) {
        if (!mounted.current || version !== requestVersion.current) return;
        e('Login session failed', waitError);
        const message = errorMessage(waitError);
        if (message.includes('authorization timed out')) {
          void refresh();
          return;
        }
        if ((waitError as Partial<CommandError>)?.kind === 'cancelled') return;
        setError(message);
        setStatus('failed');
      }
    } catch (beginError) {
      if (!mounted.current || version !== requestVersion.current) return;
      e('Unable to begin login', beginError);
      setError(errorMessage(beginError));
      setStatus('failed');
    }
  }, [onAuthenticated]);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => {
      mounted.current = false;
      requestVersion.current += 1;
      const active = activeSession.current;
      activeSession.current = undefined;
      if (active) void desktopApi.cancelLogin(active.requestId);
    };
    // Login starts once for this signed-out surface. Refresh owns later sessions.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    void desktopApi.appVersion().then(setVersion).catch((reason) => e('Unable to read app version', reason));
  }, []);

  const loading = status === 'loading' || status === 'provisioning';
  function revealLogs() {
    versionClicks.current += 1;
    if (versionClicks.current < 5) return;
    versionClicks.current = 0;
    setLogsOpen(true);
  }

  return (
    <main className={classes("login-page")}>
      <div data-tauri-drag-region className={classes("launch-titlebar")} />
      <section className={classes("login-card")}>
        {loading ? (
          <div className={classes("login-loading")}>
            <LoaderCircle className={classes("login-spinner")} size={42} />
            <h1>{status === 'provisioning' ? 'Loading' : 'Initializing'}</h1>
            <p>Messages are end-to-end encrypted.</p>
          </div>
        ) : (
          <div className={classes("login-content")}>
            <button
              className={classes(`login-qr ${error ? 'has-error' : ''}`)}
              type="button"
              onClick={() => error && void refresh()}
              aria-label={error ? 'Reload QR code' : 'Login QR code'}
            >
              {session?.authUrl && (
                <QRCodeSVG value={session.authUrl} size={144} level="Q" marginSize={0} />
              )}
              <img className={classes("qr-logo")} src={mixinLogoUrl} alt="" aria-hidden="true" />
              {error && (
                <span className={classes("qr-error")} title={error}>
                  <RotateCw size={40} />
                  <span>Click to reload the QR code</span>
                </span>
              )}
            </button>
            <h1>Log in to Mixin Messenger with a QR code</h1>
            <ol>
              <li>Open Mixin Messenger on your phone.</li>
              <li>Scan the QR code on the screen and confirm your sign-in.</li>
            </ol>
          </div>
        )}
      </section>
      <button className={classes("login-version")} type="button" onClick={revealLogs}>{version}</button>
      {logsOpen && <LogsDialog onClose={() => setLogsOpen(false)} />}
    </main>
  );
}
