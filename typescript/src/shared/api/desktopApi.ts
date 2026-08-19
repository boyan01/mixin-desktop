import { convertFileSrc } from '@tauri-apps/api/core';
import { commands } from '@/generated/bindings';
import type { DownloadSettings, VoiceRecording } from '@/generated/bindings';

export const desktopApi = {
  ...commands,
  searchLocalUsers: (query: string, category: string, limit = 64) =>
    commands.searchLocalUsers(query, category, limit),
  searchGlobalMessages: (query: string, anchorMessageId?: string, limit = 32) =>
    commands.searchGlobalMessages(query, anchorMessageId ?? null, limit),
  editConversation: (conversationId: string, name?: string, announcement?: string) =>
    commands.editConversation(conversationId, name ?? null, announcement ?? null),
  messages: (
    conversationId: string,
    beforeCreatedAtMicros?: number,
    beforeMessageId?: string,
    limit = 60,
  ) => commands.messages(
    conversationId,
    beforeCreatedAtMicros ?? null,
    beforeMessageId ?? null,
    limit,
  ),
  messagesAround: (conversationId: string, targetMessageId: string, before = 30, after = 30) =>
    commands.messagesAround(conversationId, targetMessageId, before, after),
  sendText: (
    conversationId: string,
    content: string,
    quoteMessageId?: string,
    silent = false,
  ) => commands.sendText(conversationId, content, quoteMessageId ?? null, silent),
  createAttachmentArchive: (paths: string[], password?: string) =>
    commands.createAttachmentArchive(paths, password ?? null),
  sendAttachment: (
    conversationId: string,
    path: string,
    kind: string,
    mimeType: string,
    name?: string,
    width?: number,
    height?: number,
    durationMillis?: number,
    thumbnail?: string,
    caption?: string,
    quoteMessageId?: string,
    silent = false,
  ) => commands.sendAttachment({
    conversationId,
    path,
    kind,
    mimeType,
    name: name ?? null,
    width: width ?? null,
    height: height ?? null,
    durationMillis: durationMillis ?? null,
    thumbnail: thumbnail ?? null,
    caption: caption ?? null,
    quoteMessageId: quoteMessageId ?? null,
    silent,
  }),
  sendRemoteImage: (
    conversationId: string,
    url: string,
    previewUrl: string,
    width?: number,
    height?: number,
    silent = false,
    mimeType = 'image/gif',
  ) => commands.sendRemoteImage({
    conversationId,
    url,
    previewUrl,
    width: width ?? null,
    height: height ?? null,
    mimeType,
    silent,
  }),
  searchMessages: (
    conversationId: string,
    query: string,
    senderId?: string,
    categories?: string[],
    anchorMessageId?: string,
    limit = 60,
  ) => commands.searchMessages(
    conversationId,
    query,
    senderId ?? null,
    categories ?? null,
    anchorMessageId ?? null,
    limit,
  ),
  setDownloadSetting: (kind: keyof DownloadSettings, value: boolean) =>
    commands.setDownloadSetting(kind, value),
  updateParticipants: (
    conversationId: string,
    action: string,
    userIds: string[],
    role?: string,
  ) => commands.updateParticipants(conversationId, action, userIds, role ?? null),
  sharedMessages: (conversationId: string, kind: string, offset = 0, limit = 100) =>
    commands.sharedMessages(conversationId, kind, offset, limit),
  sendContact: (conversationId: string, sharedUserId: string, quoteMessageId?: string) =>
    commands.sendContact(conversationId, sharedUserId, quoteMessageId ?? null),
  stopAndSendVoice: (conversationId: string, quoteMessageId?: string) =>
    commands.stopAndSendVoice(conversationId, quoteMessageId ?? null),
  sendVoiceRecording: (
    conversationId: string,
    recording: VoiceRecording,
    quoteMessageId?: string,
  ) => commands.sendVoiceRecording(conversationId, recording, quoteMessageId ?? null),
  forwardMessages: (
    targetConversationId: string,
    sourceMessageIds: string[],
    combined = false,
  ) => commands.forwardMessages(targetConversationId, sourceMessageIds, combined),
};

export function resolveAssetUrl(value?: string | null): string | undefined {
  if (!value) return undefined;
  if (/^(https?:|data:|blob:|asset:)/.test(value)) return value;
  return convertFileSrc(value);
}
