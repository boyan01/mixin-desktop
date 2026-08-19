import { useEffect, useState } from 'react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { classes } from '@/shared/styles/classes';

interface LogsDialogProps {
  onClose: () => void;
}

export function LogsDialog({ onClose }: LogsDialogProps) {
  const [lines, setLines] = useState<string[]>();
  const [error, setError] = useState<string>();

  async function reload() {
    setError(undefined);
    try {
      setLines(await desktopApi.logs());
    } catch (reason) {
      e('Unable to load application logs', reason);
      setError(commandMessage(reason, 'Unable to load logs'));
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  return (
    <div className={classes("modal-backdrop")}>
      <section className={classes("logs-dialog")} role="dialog" aria-modal="true">
        <header>
          <span />
          <strong>Logs</strong>
          <button type="button" onClick={() => void reload()}>↻</button>
          <button type="button" onClick={onClose}>×</button>
        </header>
        <pre>{error ?? (lines ? [...lines].reverse().join('\n') : '')}</pre>
      </section>
    </div>
  );
}

function commandMessage(reason: unknown, fallback: string) {
  return typeof reason === 'object' && reason !== null && 'message' in reason
    ? String(reason.message)
    : fallback;
}
