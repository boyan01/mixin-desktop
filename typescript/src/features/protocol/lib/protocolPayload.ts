import { desktopApi } from '@/shared/api/desktopApi';

export type ProtocolSendPayload = {
  category: 'text' | 'image' | 'sticker' | 'contact' | 'post' | 'app_card';
  data: string;
};

export function decodeProtocolSendPayload(category: string | null, encoded: string | null): ProtocolSendPayload | undefined {
  if (!encoded || !category || !['text', 'image', 'sticker', 'contact', 'post', 'app_card'].includes(category)) return undefined;
  try {
    const normalized = encoded.replace(/-/g, '+').replace(/_/g, '/');
    const padded = normalized + '='.repeat((4 - normalized.length % 4) % 4);
    const decoded = new TextDecoder().decode(Uint8Array.from(atob(padded), (character) => character.charCodeAt(0)));
    if (!decoded) return undefined;
    if (category === 'image') {
      const url = JSON.parse(decoded).url;
      if (typeof url !== 'string' || !url) return undefined;
      return { category, data: url };
    }
    if (category === 'contact') {
      const userId = JSON.parse(decoded).user_id;
      if (typeof userId !== 'string' || !userId) return undefined;
      return { category, data: userId };
    }
    return { category: category as ProtocolSendPayload['category'], data: decoded };
  } catch { return undefined; }
}

export async function sendProtocolPayload(conversationId: string, payload: ProtocolSendPayload) {
  switch (payload.category) {
    case 'text': return desktopApi.sendText(conversationId, payload.data);
    case 'post': return desktopApi.sendPost(conversationId, payload.data);
    case 'app_card': return desktopApi.sendAppCard(conversationId, payload.data);
    case 'sticker': return desktopApi.sendSticker(conversationId, payload.data);
    case 'contact': return desktopApi.sendContact(conversationId, payload.data);
    case 'image': return desktopApi.sendRemoteImage(conversationId, payload.data, payload.data, undefined, undefined, false, 'image/jpeg');
  }
}
