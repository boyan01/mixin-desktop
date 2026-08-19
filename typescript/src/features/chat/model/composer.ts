/** `ChatInputBar` is 56px high with a 40px, seven-line input surface. */
export const CHAT_COMPOSER_MIN_HEIGHT = 56;
export const CHAT_COMPOSER_INPUT_MIN_HEIGHT = 40;
export const CHAT_COMPOSER_INPUT_MAX_HEIGHT = 140;
export const CHAT_COMPOSER_MAX_TEXT_LENGTH = 64 * 1024;

export interface AttachmentDraft {
  path: string;
  name: string;
  mimeType: string;
  kind: 'IMAGE' | 'VIDEO' | 'DATA';
  size: number;
}

export type AttachmentPreviewMode = 'media' | 'files' | 'zip';

/** Native Tauri drops expose filesystem paths, unlike ordinary browser File objects. */
export function droppedAttachmentPaths(files: readonly { path?: string }[]) {
  return [...new Set(files.map((file) => file.path).filter((path): path is string => Boolean(path)))];
}

export function composerInputHeight(scrollHeight: number) {
  return Math.min(Math.max(scrollHeight, CHAT_COMPOSER_INPUT_MIN_HEIGHT), CHAT_COMPOSER_INPUT_MAX_HEIGHT);
}

export function resizeComposerTextArea(input: HTMLTextAreaElement) {
  input.style.height = 'auto';
  const height = composerInputHeight(input.scrollHeight);
  input.style.height = `${height}px`;
  input.style.overflowY = input.scrollHeight > CHAT_COMPOSER_INPUT_MAX_HEIGHT ? 'auto' : 'hidden';
}

export function attachmentMimeType(name: string) {
  const extension = name.split('.').pop()?.toLocaleLowerCase() ?? '';
  const mimeTypes: Record<string, string> = {
    png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp',
    heic: 'image/heic', mp4: 'video/mp4', mov: 'video/quicktime', m4v: 'video/x-m4v', pdf: 'application/pdf',
    zip: 'application/zip', txt: 'text/plain', json: 'application/json',
  };
  return mimeTypes[extension] ?? 'application/octet-stream';
}

export function attachmentDraft(info: { path: string; name: string; size: number }): AttachmentDraft {
  const mimeType = attachmentMimeType(info.name);
  return { ...info, mimeType, kind: mimeType.startsWith('image/') ? 'IMAGE' : mimeType.startsWith('video/') ? 'VIDEO' : 'DATA' };
}

export function attachmentPreviewMode(files: readonly AttachmentDraft[], mode: AttachmentPreviewMode): AttachmentPreviewMode {
  if (mode === 'zip' && files.length < 2) return 'files';
  if (mode === 'media' && !files.some((file) => file.kind !== 'DATA')) return 'files';
  return mode;
}

export function formatAttachmentBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1; }
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}
