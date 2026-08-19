import type { Conversation, ConversationCategory } from '@/shared/model/types';

export function matchesCategory(
  conversation: Conversation,
  category: ConversationCategory,
  circleId?: string,
): boolean {
  switch (category) {
    case 'groups':
      return conversation.category === 'GROUP';
    case 'bots':
      return conversation.category !== 'GROUP' && conversation.isBot;
    case 'strangers':
      return conversation.category !== 'GROUP'
        && !conversation.isBot
        && conversation.relationship === 'STRANGER';
    case 'contacts':
      return conversation.category !== 'GROUP'
        && !conversation.isBot
        && conversation.relationship === 'FRIEND';
    case 'circle':
      return Boolean(circleId && conversation.circleIds.includes(circleId));
    case 'chats':
      return true;
  }
}

export function conversationPreview(conversation: Conversation, currentUserId: string): string {
  if (conversation.draft && conversation.status !== 3) return conversation.draft;
  if (!conversation.lastMessageCategory) return '';
  const category = conversation.lastMessageCategory;
  const content = conversation.lastMessage.trim();
  if (conversation.lastMessageStatus === 'FAILED') return 'Waiting for this message';
  if (conversation.lastMessageStatus === 'UNKNOWN') return 'Message not supported';
  if (category === 'SYSTEM_CONVERSATION') return systemConversationPreview(conversation, currentUserId);
  if (category === 'MESSAGE_PIN') return `${conversation.lastMessageSenderName ?? ''} pinned a message`;

  let text: string;
  if (category.includes('TEXT')) text = content;
  else if (category.includes('SNAPSHOT')) text = '[Transfer]';
  else if (category.includes('STICKER')) text = '[Sticker]';
  else if (category.includes('IMAGE')) text = '[Image]';
  else if (category.includes('VIDEO')) text = '[Video]';
  else if (category.includes('LIVE')) text = '[Live]';
  else if (category.includes('DATA')) text = '[File]';
  else if (category.includes('POST')) text = content || 'Post';
  else if (category.includes('LOCATION')) text = '[Location]';
  else if (category.includes('AUDIO')) text = '[Audio]';
  else if (category === 'APP_BUTTON_GROUP') text = appButtonGroupPreview(conversation.lastMessage);
  else if (category === 'APP_CARD') text = appCardPreview(conversation.lastMessage);
  else if (category.includes('CONTACT')) text = '[Contact]';
  else if (category.startsWith('WEBRTC') || category.startsWith('KRAKEN')) text = 'Voice call';
  else if (category.includes('RECALL')) text = conversation.lastMessageSenderId === currentUserId ? '[You deleted this message]' : '[This message was deleted]';
  else if (category.includes('TRANSCRIPT')) text = '[Transcript]';
  else if (category.includes('INSCRIPTION')) text = '[Collectible]';
  else text = 'Message not supported';

  const showSender = conversation.category === 'GROUP' || conversation.lastMessageSenderId !== conversation.ownerId;
  if (!text || !showSender) return text;
  const sender = conversation.lastMessageSenderId === currentUserId ? 'You' : conversation.lastMessageSenderName ?? '';
  return `${sender}: ${text}`;
}

function systemConversationPreview(conversation: Conversation, currentUserId: string): string {
  const participant = conversation.lastMessageParticipantId === currentUserId ? 'You' : conversation.lastMessageParticipantName ?? '';
  const sender = conversation.lastMessageSenderId === currentUserId ? 'You' : conversation.lastMessageSenderName ?? '';
  switch (conversation.lastMessageAction?.toUpperCase()) {
    case 'JOIN': return `${participant} joined the group via invite link`;
    case 'EXIT': return `${participant} left`;
    case 'ADD': return `${sender} added ${participant}`;
    case 'REMOVE': return `${sender} removed ${participant}`;
    case 'CREATE': return `${sender} created this group`;
    case 'ROLE': return `${participant} now an admin`;
    case 'EXPIRE': return expiringMessagePreview(sender, conversation.lastMessage);
    default: return 'Message not supported';
  }
}

function expiringMessagePreview(sender: string, content: string): string {
  if (!content) return `${sender} changed disappearing message settings.`;
  const seconds = Number.parseInt(content, 10);
  if (!Number.isFinite(seconds)) return `${sender} changed disappearing message settings.`;
  if (seconds <= 0) return `${sender} disabled disappearing message`;
  const unit = seconds < 60 ? ['second', seconds] : seconds < 3600 ? ['minute', Math.floor(seconds / 60)] : seconds < 86400 ? ['hour', Math.floor(seconds / 3600)] : seconds < 604800 ? ['day', Math.floor(seconds / 86400)] : ['week', Math.floor(seconds / 604800)];
  const [name, count] = unit as [string, number];
  return `${sender} set disappearing message time to ${count} ${name}${count === 1 ? '' : 's'}`;
}

function appButtonGroupPreview(content: string): string {
  try {
    const items: unknown = JSON.parse(content);
    return Array.isArray(items)
      ? items.map((item) => typeof item === 'object' && item !== null && typeof item.label === 'string' ? `[${item.label}]` : '').join('')
      : '';
  } catch {
    return '';
  }
}

function appCardPreview(content: string): string {
  try {
    const card: unknown = JSON.parse(content);
    if (typeof card === 'object' && card !== null) {
      const title = (card as Record<string, unknown>).title;
      return typeof title === 'string' ? `[${title}]` : '[Card]';
    }
    return '[Card]';
  } catch {
    return '[Card]';
  }
}
