import type { Conversation, Message } from '@/shared/model/types';

export const completedStatuses = new Set(['SENT', 'DELIVERED', 'READ']);

export function isCategory(category: string, suffix: string) {
  return category.endsWith(`_${suffix}`);
}

/**
 * `MessageBubble` renders attachment metadata outside its painted surface for
 * these categories. Keeping this decision here makes the DOM row follow the
 * same category contract as Flutter's `useOuterMessageDateAndStatus`.
 */
export function messageBubblePresentation(message: Pick<Message, 'category' | 'status'>) {
  const category = message.category;
  const unresolved = ['UNKNOWN', 'FAILED'].includes(message.status.toUpperCase());
  const outerMetadata = !unresolved && (
    isCategory(category, 'AUDIO')
    || isCategory(category, 'STICKER')
    || isCategory(category, 'DATA')
    || isCategory(category, 'CONTACT')
    || isCategory(category, 'LOCATION')
    || category === 'SYSTEM_ACCOUNT_SNAPSHOT'
    || category === 'SYSTEM_SAFE_SNAPSHOT'
    || category === 'SYSTEM_SAFE_INSCRIPTION'
  );
  return {
    outerMetadata,
    hideOuterStatus: category.startsWith('SYSTEM_'),
    forceOtherBubbleColor: isCategory(category, 'AUDIO')
      || category === 'SYSTEM_SAFE_SNAPSHOT'
      || category === 'SYSTEM_SAFE_INSCRIPTION',
  };
}

export function canReplyToCategory(category: string) {
  return ['TEXT', 'IMAGE', 'VIDEO', 'LIVE', 'DATA', 'POST', 'LOCATION', 'AUDIO', 'STICKER', 'CONTACT', 'TRANSCRIPT']
    .some((suffix) => isCategory(category, suffix))
    || category === 'APP_CARD'
    || category === 'APP_BUTTON_GROUP';
}

function jsonShareable(content: string, fallback: boolean) {
  try {
    const value = JSON.parse(content) as { shareable?: unknown };
    return typeof value.shareable === 'boolean' ? value.shareable : fallback;
  } catch {
    return fallback;
  }
}

export function hasInvalidSpecialPayload(message: Message) {
  try {
    const value = JSON.parse(message.content) as unknown;
    if (message.category.endsWith('_LOCATION')) {
      return typeof value !== 'object' || value === null
        || typeof (value as { latitude?: unknown }).latitude !== 'number'
        || typeof (value as { longitude?: unknown }).longitude !== 'number';
    }
    if (message.category.endsWith('_TRANSCRIPT') || message.category === 'APP_BUTTON_GROUP') {
      return !Array.isArray(value) || value.some((item) => typeof item !== 'object' || item === null);
    }
    if (message.category === 'APP_CARD') {
      return typeof value !== 'object' || value === null
        || ('actions' in value && (value as { actions?: unknown }).actions !== undefined
          && (!Array.isArray((value as { actions: unknown }).actions)
            || (value as { actions: unknown[] }).actions.some((item) => typeof item !== 'object' || item === null)));
    }
    return false;
  } catch {
    return message.category.endsWith('_LOCATION') || message.category.endsWith('_TRANSCRIPT') || message.category === 'APP_BUTTON_GROUP' || message.category === 'APP_CARD';
  }
}

export function canForwardMessage(message: Message) {
  if (!completedStatuses.has(message.status.toUpperCase())) return false;
  const category = message.category;
  if (category === 'APP_CARD') return jsonShareable(message.content, false);
  const attachment = ['IMAGE', 'VIDEO', 'AUDIO', 'DATA'].some((suffix) => isCategory(category, suffix));
  const finishedAttachment = attachment
    && ['DONE', 'READ'].includes(message.mediaStatus.toUpperCase())
    && Boolean(message.mediaUrl)
    && jsonShareable(message.content, true);
  return isCategory(category, 'TEXT')
    || finishedAttachment
    || ['STICKER', 'CONTACT', 'POST', 'LOCATION'].some((suffix) => isCategory(category, suffix))
    || (isCategory(category, 'LIVE') && jsonShareable(message.content, false))
    || (isCategory(category, 'TRANSCRIPT') && (message.mediaSize ?? 0) <= 0);
}

export function canCombineForwardMessage(message: Message) {
  if (!completedStatuses.has(message.status.toUpperCase()) || isCategory(message.category, 'TRANSCRIPT')) return false;
  if (message.category === 'APP_CARD') return jsonShareable(message.content, false);
  const attachment = ['IMAGE', 'VIDEO', 'AUDIO', 'DATA'].some((suffix) => isCategory(message.category, suffix));
  if (attachment) {
    if (!['DONE', 'READ'].includes(message.mediaStatus.toUpperCase())) return false;
    return !isCategory(message.category, 'AUDIO') || jsonShareable(message.content, true);
  }
  return ['TEXT', 'STICKER', 'CONTACT', 'LIVE', 'POST', 'LOCATION'].some((suffix) => isCategory(message.category, suffix));
}

export function messageCopyPolicy(message: Message, selectedText?: string) {
  const selected = selectedText?.trim();
  const text = isCategory(message.category, 'TEXT');
  const image = isCategory(message.category, 'IMAGE');
  return {
    text: text || isCategory(message.category, 'POST'),
    image: image && ['DONE', 'READ'].includes(message.mediaStatus.toUpperCase()) && Boolean(message.mediaUrl),
    caption: image && Boolean(message.caption?.trim()),
    appCardSelection: message.category === 'APP_CARD' && Boolean(selected),
    selectedText: text ? selected : undefined,
    captionSelectedText: image ? selected : undefined,
  };
}

export function canRecallSelectionMessage(message: Message, conversation: Conversation, currentUserId: string) {
  if (!completedStatuses.has(message.status.toUpperCase()) || !canReplyToCategory(message.category) || message.category === 'APP_BUTTON_GROUP' || message.category.startsWith('SYSTEM_')) return false;
  if (Date.now() - message.createdAtMicros / 1000 > 30 * 86_400_000) return false;
  return message.senderId === currentUserId || conversation.category === 'CONTACT';
}

export function formatDuration(raw: string): string {
  const milliseconds = Number(raw);
  if (!Number.isFinite(milliseconds) || milliseconds <= 0) return '0:00';
  const seconds = Math.round(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

export function formatMessageTime(micros: number): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(micros / 1000));
}

export function sameLocalDay(leftMicros: number, rightMicros: number): boolean {
  const left = new Date(leftMicros / 1000);
  const right = new Date(rightMicros / 1000);
  return left.getFullYear() === right.getFullYear() && left.getMonth() === right.getMonth() && left.getDate() === right.getDate();
}

export function formatDayTime(micros: number): string {
  const date = new Date(micros / 1000);
  const today = new Date();
  if (sameLocalDay(micros, Date.now() * 1000)) return 'Today';
  return date.getFullYear() === today.getFullYear()
    ? new Intl.DateTimeFormat(undefined, { month: 'short', weekday: 'short', day: 'numeric' }).format(date)
    : new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', weekday: 'short', day: 'numeric' }).format(date);
}

export function messageNameColor(value: string) {
  const colors = ['#8C8DFF', '#7983C2', '#6D8DDE', '#5979F0', '#6695DF', '#8F7AC5', '#9D77A5', '#8A64D0', '#AA66C3', '#A75C96', '#C8697D', '#B74D62', '#BD637C', '#B3798E', '#9B6D77', '#B87F7F', '#C5595A', '#AA4848', '#B0665E', '#B76753', '#BB5334', '#C97B46', '#BE6C2C', '#CB7F40', '#A47758', '#B69370', '#A49373', '#AA8A46', '#AA8220', '#76A048', '#9CAD23', '#A19431', '#AA9100', '#A09555', '#C49B4B', '#5FB05F', '#6AB48F', '#71B15C', '#B3B357', '#A3B561', '#909F45', '#93B289', '#3D98D0', '#429AB6', '#4EABAA', '#6BC0CE', '#64B5D9', '#3E9CCB', '#2887C4', '#52A98B'];
  const parts = value.trim().split('-');
  let hash: bigint;
  if (parts.length === 5 && parts.every((part) => /^[0-9a-f]+$/i.test(part))) {
    const high = (BigInt(`0x${parts[0]}`) << 32n) | (BigInt(`0x${parts[1]}`) << 16n) | BigInt(`0x${parts[2]}`);
    const low = (BigInt(`0x${parts[3]}`) << 48n) | BigInt(`0x${parts[4]}`);
    const hilo = high ^ low;
    hash = (hilo >> 32n) ^ BigInt.asIntN(32, hilo);
  } else hash = [...value].reduce((result, character) => BigInt.asIntN(32, result * 31n + BigInt(character.charCodeAt(0))), 0n);
  return colors[Number((hash < 0n ? -hash : hash) % BigInt(colors.length))];
}

export type AppButtonAction = { label?: string; action?: string; color?: string };

/** Mirrors Flutter's six-unit ActionButtonLayout at its 340px maximum width. */
export function actionButtonRows(actions: readonly AppButtonAction[]) {
  const rows: Array<Array<{ action: AppButtonAction; index: number }>> = [];
  let row: Array<{ action: AppButtonAction; index: number }> = [];
  let units = 0;
  for (const [index, action] of actions.entries()) {
    // Flutter measures the padded label before assigning a one-, two- or
    // three-column slot. The canvas-free estimate uses the same 28px padding.
    const labelWidth = [...(action.label || 'Open')].length * 7.4 + 28;
    const nextUnits = labelWidth <= 108 ? 2 : labelWidth <= 166 ? 3 : 6;
    if (units + nextUnits > 6) {
      if (row.length) rows.push(row);
      row = [];
      units = 0;
    }
    row.push({ action, index });
    units += nextUnits;
    if (units === 6) {
      rows.push(row);
      row = [];
      units = 0;
    }
  }
  if (row.length) rows.push(row);
  return rows;
}

export function mapTilePosition(latitude: number, longitude: number, zoom: number) {
  const scale = 2 ** zoom;
  const horizontal = (longitude + 180) / 360 * scale;
  const radians = latitude * Math.PI / 180;
  const vertical = (1 - Math.asinh(Math.tan(radians)) / Math.PI) / 2 * scale;
  return {
    x: Math.floor(horizontal),
    y: Math.floor(vertical),
    fractionX: horizontal - Math.floor(horizontal),
    fractionY: vertical - Math.floor(vertical),
    scale,
  };
}
