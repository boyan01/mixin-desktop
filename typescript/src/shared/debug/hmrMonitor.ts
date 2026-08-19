interface HmrError {
  message: string;
  stack?: string;
}

interface HmrState {
  revision: number;
  lastUpdateAt?: number;
  lastUpdateKind?: 'ready' | 'update';
  error?: HmrError;
  listenerAttached: boolean;
}

declare global {
  interface Window {
    __MIXIN_HMR_STATE__?: HmrState;
  }
}

function notifyBridge(kind: 'ready' | 'update' | 'error', message?: string) {
  void emit('ui-debug-hmr', { kind, message }).catch(() => undefined);
}

if (import.meta.env.DEV && import.meta.hot) {
  const state = window.__MIXIN_HMR_STATE__ ??= {
    revision: 0,
    listenerAttached: false,
  };

  notifyBridge('ready');
  state.lastUpdateKind = 'ready';

  if (!state.listenerAttached) {
    state.listenerAttached = true;
    import.meta.hot.on('vite:afterUpdate', () => {
      state.revision += 1;
      state.lastUpdateAt = Date.now();
      state.lastUpdateKind = 'update';
      state.error = undefined;
      notifyBridge('update');
      window.dispatchEvent(new CustomEvent('mixin:hmr-update', { detail: state }));
    });
    import.meta.hot.on('vite:error', (payload) => {
      state.error = {
        message: payload.err.message,
        stack: payload.err.stack,
      };
      notifyBridge('error', state.error.message);
      window.dispatchEvent(new CustomEvent('mixin:hmr-error', { detail: state.error }));
    });
  }
  import.meta.hot.accept();
}

export {};
import { emit } from '@tauri-apps/api/event';
