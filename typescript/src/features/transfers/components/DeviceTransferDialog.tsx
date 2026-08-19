import { useEffect, useState } from 'react';
import { ArrowLeft, CloudDownload, CloudUpload, Smartphone, X } from 'lucide-react';
import { events } from '@/generated/bindings';
import { desktopApi } from '@/shared/api/desktopApi';
import { useDesktopUi } from '@/state/desktopUi';
import { AlertDialogLayout, MixinButton, MixinDialog } from '@/shared/ui/MixinDialog';
import type { DeviceTransferCommand } from '@/shared/model/types';
import { classes } from '@/shared/styles/classes';

type Page = 'home' | 'restore' | 'backup' | 'restoreWaiting' | 'backupWaiting';
type Processing = 'restore' | 'backup';

export function DeviceTransferDialog() {
  const open = useDesktopUi((state) => state.deviceTransferOpen);
  const close = useDesktopUi((state) => state.closeDeviceTransfer);
  const [page, setPage] = useState<Page>('home');
  const [processing, setProcessing] = useState<Processing>();
  const [progress, setProgress] = useState(0);
  const [speed, setSpeed] = useState(0);
  const [approval, setApproval] = useState<'restore' | 'backup'>();
  const [result, setResult] = useState<string>();

  useEffect(() => {
    const listener = events.accountDeviceTransfer.listen(({ payload }) => {
    switch (payload.kind) {
      case 'restoreConnected': setProcessing('restore'); setProgress(0); setSpeed(0); break;
      case 'backupServerCreated': setProcessing('backup'); setProgress(0); setSpeed(0); break;
      case 'restoreProgress': setProgress(payload.value ?? 0); break;
      case 'backupProgress': setProgress(payload.value ?? 0); break;
      case 'restoreNetworkSpeed': setSpeed(payload.value ?? 0); break;
      case 'backupNetworkSpeed': setSpeed(payload.value ?? 0); break;
      case 'restoreStart': case 'backupStart': close(); break;
      case 'restoreSucceed': case 'backupSucceed': setProcessing(undefined); setResult('Transfer completed'); break;
      case 'restoreFailed': case 'backupFailed': setProcessing(undefined); setResult('Device transfer failed'); break;
      case 'backupRequestReceived': setApproval('restore'); break;
      case 'restoreRequestReceived': setApproval('backup'); break;
      case 'connectionFailed': setResult(payload.reason === 'versionNotMatched' ? 'Transfer protocol version does not match' : 'Device transfer failed'); break;
    }
    // The handler owns the full account event stream for the app lifetime.
    });
    return () => { void listener.then((unlisten) => unlisten()); };
  }, [close]);

  async function command(value: DeviceTransferCommand) {
    await desktopApi.deviceTransferCommand(value);
  }
  async function start(kind: 'restore' | 'backup') {
    await command(kind === 'restore' ? 'pullToRemote' : 'pushToRemote');
    setPage(kind === 'restore' ? 'restoreWaiting' : 'backupWaiting');
  }
  async function cancel(kind: Processing) {
    await command(kind === 'restore' ? 'cancelRestore' : 'cancelBackup');
    setProcessing(undefined); close();
  }
  async function resolveApproval(approved: boolean) {
    const kind = approval;
    setApproval(undefined);
    if (!kind) return;
    await command(kind === 'restore' ? approved ? 'confirmRestore' : 'cancelRestoreRequest' : approved ? 'confirmBackup' : 'cancelBackupRequest');
  }
  function closeDialog() { setPage('home'); close(); }

  return <>
    {open && <div className={classes("modal-backdrop device-transfer-backdrop")} role="presentation">
      <section className={classes("device-transfer-dialog")} role="dialog" aria-modal="true" aria-label="Chat Backup and Restore">
        {page === 'home' && <><TransferHeader title="Chat Backup and Restore" onClose={closeDialog} /><TransferCell label="sync from other device" onClick={() => setPage('restore')} /><TransferCell label="sync to other device" onClick={() => setPage('backup')} /></>}
        {page === 'restore' && <TransferPage title="sync from other device" icon={<Smartphone />} description="Restore chat history from another device." action="Restore Chat" onBack={() => setPage('home')} onClose={closeDialog} onAction={() => void start('restore')} />}
        {page === 'backup' && <TransferPage title="backup to other device" icon={<Smartphone />} description="Back up chat history to another device." action="Backup Chat" onBack={() => setPage('home')} onClose={closeDialog} onAction={() => void start('backup')} />}
        {page === 'restoreWaiting' && <TransferPage title="restore from other device" icon={<CloudDownload />} description="Waiting for the other device to connect." action="Cancel" onBack={() => void cancel('restore')} onClose={() => void cancel('restore')} onAction={() => void cancel('restore')} />}
        {page === 'backupWaiting' && <TransferPage title="backup to other device" icon={<CloudUpload />} description="Waiting for the other device to connect." action="Cancel" onBack={() => void cancel('backup')} onClose={() => void cancel('backup')} onAction={() => void cancel('backup')} />}
      </section>
    </div>}
    {processing && <div className={classes("modal-backdrop device-transfer-backdrop")}><section className={classes("transfer-processing")} role="dialog" aria-modal="true"><span>{processing === 'restore' ? <CloudDownload size={72} /> : <CloudUpload size={72} />}</span><h2>{processing === 'restore' ? 'Restoring chats' : 'Backing up chats'}</h2><progress value={Math.min(1, Math.max(0, progress))} /><strong>{Math.round(progress * 100)}%</strong><small>{formatSpeed(speed)}</small><button type="button" onClick={() => void cancel(processing)}>Cancel</button></section></div>}
    {approval && <Confirmation text={approval === 'restore' ? 'Sync chats from your phone?' : 'Sync chats to your phone?'} onClose={() => void resolveApproval(false)} onConfirm={() => void resolveApproval(true)} />}
    {result && <Confirmation text={result} onClose={() => setResult(undefined)} onConfirm={() => setResult(undefined)} confirmOnly />}
  </>;
}

function TransferHeader({ title, onClose, onBack }: { title: string; onClose: () => void; onBack?: () => void }) {
  return <header>{onBack ? <button type="button" onClick={onBack}><ArrowLeft size={20} /></button> : <span /> }<strong>{title}</strong><button type="button" onClick={onClose}><X size={20} /></button></header>;
}

function TransferCell({ label, onClick }: { label: string; onClick: () => void }) {
  return <button className={classes("device-transfer-cell")} type="button" onClick={onClick}>{label}</button>;
}

function TransferPage({ title, icon, description, action, onBack, onClose, onAction }: { title: string; icon: React.ReactNode; description: string; action: string; onBack: () => void; onClose: () => void; onAction: () => void }) {
  return <><TransferHeader title={title} onBack={onBack} onClose={onClose} /><div className={classes("device-transfer-page")}><span>{icon}</span><p>{description}</p><button type="button" onClick={onAction}>{action}</button></div></>;
}

function Confirmation({ text, onClose, onConfirm, confirmOnly = false }: { text: string; onClose: () => void; onConfirm: () => void; confirmOnly?: boolean }) {
  return <MixinDialog open onClose={onClose} barrierDismissible={!confirmOnly} maxWidth={400}><AlertDialogLayout content={text} actions={<>{!confirmOnly && <MixinButton transparent onClick={onClose}>Cancel</MixinButton>}<MixinButton onClick={onConfirm}>Confirm</MixinButton></>} /></MixinDialog>;
}

function formatSpeed(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '';
  return `${(value / 1024 / 1024).toFixed(value >= 10 * 1024 * 1024 ? 0 : 1)} MB/s`;
}
