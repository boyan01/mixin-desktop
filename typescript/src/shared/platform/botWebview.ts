import { desktopApi } from '@/shared/api/desktopApi';

export type BotWebviewOptions = {
  url: string;
  title: string;
  conversationId: string;
};

function appearance() {
  if (document.documentElement.classList.contains('theme-light')) return 'light';
  if (document.documentElement.classList.contains('theme-dark')) return 'dark';
  return matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

/** Opens a bot page with the same MixinContext contract used by Flutter. */
export function openBotWebview({ url, title, conversationId }: BotWebviewOptions) {
  return desktopApi.openBotWebview(url, title, conversationId, navigator.language, appearance());
}
