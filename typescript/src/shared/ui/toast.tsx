import { useEffect } from 'react';
import { Check, LoaderCircle, X } from 'lucide-react';
import { create } from 'zustand';
import { e } from '@/shared/lib/logger';
import { classes } from '@/shared/styles/classes';

type ToastKind = 'success' | 'failed' | 'loading' | 'message';

interface ToastState {
  kind?: ToastKind;
  text: string;
  show: (kind: ToastKind, text: string, persistent?: boolean) => void;
  dismiss: () => void;
}

let timer: number | undefined;

export const useToast = create<ToastState>((set) => ({
  text: '',
  show(kind, text, persistent = false) {
    if (timer) window.clearTimeout(timer);
    set({ kind, text });
    if (!persistent) timer = window.setTimeout(() => set({ kind: undefined, text: '' }), 1_000);
  },
  dismiss() {
    if (timer) window.clearTimeout(timer);
    timer = undefined;
    set({ kind: undefined, text: '' });
  },
}));

export const showToast = (text: string) => useToast.getState().show('message', text);
export const showToastSuccessful = () => useToast.getState().show('success', 'Successful');
export const showToastFailed = (reason?: unknown) => {
  e('UI operation failed', reason);
  useToast.getState().show('failed', toastErrorMessage(reason));
};
export const showToastLoading = () => useToast.getState().show('loading', 'Loading', true);
export const dismissToast = () => useToast.getState().dismiss();

export function ToastHost() {
  const kind = useToast((state) => state.kind);
  const text = useToast((state) => state.text);
  useEffect(() => {
    const onUnhandled = (event: PromiseRejectionEvent) => { event.preventDefault(); showToastFailed(event.reason); };
    window.addEventListener('unhandledrejection', onUnhandled);
    return () => window.removeEventListener('unhandledrejection', onUnhandled);
  }, []);
  if (!kind) return null;
  return <div className={classes(`toast-host ${kind === 'loading' ? 'is-blocking' : ''}`)}><div className={classes("toast-card")}>{kind === 'success' && <Check size={30} />}{kind === 'failed' && <X size={30} />}{kind === 'loading' && <LoaderCircle className={classes("toast-loading")} size={30} />}{text && <span>{text}</span>}</div></div>;
}

function toastErrorMessage(reason?: unknown) {
  if (typeof reason === 'object' && reason !== null && 'message' in reason) return String(reason.message);
  if (typeof reason === 'string' && reason) return reason;
  return 'Failed';
}
