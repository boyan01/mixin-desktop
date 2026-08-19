import { openUrl } from '@tauri-apps/plugin-opener';

export function openExternal(url?: string) {
  if (!url) return;
  void openUrl(url);
}
