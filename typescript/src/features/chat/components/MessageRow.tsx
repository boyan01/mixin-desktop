import {
  memo,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { save } from '@tauri-apps/plugin-dialog';
import { QRCodeSVG } from 'qrcode.react';
import {
  Check,
  CheckCheck,
  Pin,
  X,
} from 'lucide-react';
import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { useDesktopUi } from '@/state/desktopUi';
import type { Conversation, Message } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { IconButton } from '@/shared/ui/IconButton';
import { flutterMenuIcon } from '@/shared/ui/flutterMenuIcon';
import { showNativeContextMenu, type NativeMenuEntry } from '@/shared/ui/nativeMenu';
import { showToastFailed, showToastLoading, showToastSuccessful } from '@/shared/ui/toast';
import { MessageContent } from '@/features/chat/components/MessageContent';
import {
  canForwardMessage,
  canReplyToCategory,
  completedStatuses,
  formatMessageTime,
  hasInvalidSpecialPayload,
  isCategory,
  messageBubblePresentation,
  messageCopyPolicy,
  messageNameColor,
  sameLocalDay,
} from '@/features/chat/model/message';
import { classes } from '@/shared/styles/classes';

async function copyMessageImage(message: Message) {
  const source = resolveAssetUrl(message.mediaUrl);
  if (!source) return;
  const response = await fetch(source);
  const blob = await response.blob();
  await navigator.clipboard.write([new ClipboardItem({ [blob.type || 'image/png']: blob })]);
}

type MessageRowProps = {
  message: Message;
  previous?: Message;
  next?: Message;
  currentUserId: string;
  currentUserRole?: string | null;
  showAvatar: boolean;
  conversation: Conversation;
  mentionNames: Record<string, string>;
  onOpenUser: (userId: string) => void;
  audioPlaylist?: Message[];
  imagePlaylist?: Message[];
  isPinnedPage?: boolean;
  onLocateToChat?: (messageId: string) => void;
  onPinnedChanged?: () => void;
};

export const MessageRow = memo(function MessageRow({ message, previous, next, currentUserId, currentUserRole, showAvatar, conversation, mentionNames, onOpenUser, audioPlaylist = [], imagePlaylist = [], isPinnedPage = false, onLocateToChat, onPinnedChanged }: MessageRowProps) {
  const mine = message.senderId === currentUserId;
  const queryClient = useQueryClient();
  const setQuote = useDesktopUi((state) => state.setQuote);
  const openForward = useDesktopUi((state) => state.openForward);
  const selectionConversationId = useDesktopUi((state) => state.selectionConversationId);
  const selectedMessageIds = useDesktopUi((state) => state.selectedMessageIds);
  const beginMessageSelection = useDesktopUi((state) => state.beginMessageSelection);
  const toggleMessageSelection = useDesktopUi((state) => state.toggleMessageSelection);
  const showIdentityNumber = useDesktopUi((state) => state.uiSettings.showIdentityNumber);
  const focusMessageId = useDesktopUi((state) => state.focusMessageId);
  const focusMessage = useDesktopUi((state) => state.focusMessage);
  const messageElementRef = useRef<HTMLDivElement>(null);
  const mentionMarkedRead = useRef(false);
  const [showQrCode, setShowQrCode] = useState(false);
  const [showAddImageSticker, setShowAddImageSticker] = useState(false);
  const bypassActions = ['SYSTEM_CONVERSATION', 'MESSAGE_PIN', 'SECRET', 'STRANGER'].includes(message.category);
  const groupedConversation = conversation.category === 'GROUP' || conversation.isBotGroup || message.senderId !== (message.conversationOwnerId || conversation.ownerId);
  const sameDayPrevious = previous != null && sameLocalDay(previous.createdAtMicros, message.createdAtMicros);
  const sameUserPrevious = previous != null && !previous.category.startsWith('SYSTEM_') && previous.category !== 'MESSAGE_PIN' && previous.senderId === message.senderId;
  const shouldShowAvatar = groupedConversation && showAvatar;
  const sameDayNext = next != null && sameLocalDay(next.createdAtMicros, message.createdAtMicros);
  const sameUserNext = next?.senderId === message.senderId;
  const showNip = !(sameUserNext && sameDayNext) && (!shouldShowAvatar || mine);
  const showSender = groupedConversation && !mine && (!sameUserPrevious || !sameDayPrevious);
  const selecting = selectionConversationId === message.conversationId;
  const completed = completedStatuses.has(message.status.toUpperCase());
  const canReply = !isPinnedPage && canReplyToCategory(message.category);
  const canForward = canForwardMessage(message);
  const canPin = canReply && completed && Boolean(currentUserRole);
  const recallBase = completed
    && canReply
    && message.category !== 'APP_BUTTON_GROUP'
    && Date.now() < message.createdAtMicros / 1000 + 30 * 24 * 60 * 60 * 1000;
  const canRecall = !isPinnedPage && recallBase && (mine
    || message.conversationCategory?.toUpperCase() === 'CONTACT'
    || (message.conversationCategory?.toUpperCase() === 'GROUP'
      && (message.conversationOwnerId === currentUserId
        || currentUserRole === 'OWNER'
        || (currentUserRole === 'ADMIN'
          && message.senderId !== message.conversationOwnerId
          && Boolean(message.senderParticipantId)
          && !message.senderRole))));
  const unresolved = ['UNKNOWN', 'FAILED'].includes(message.status.toUpperCase());
  const invalidSpecial = hasInvalidSpecialPayload(message)
    || (message.category.endsWith('_IMAGE') && (message.mediaWidth == null || message.mediaHeight == null));
  const bubbleless = !unresolved && !invalidSpecial && (message.category.includes('STICKER') || message.category === 'APP_CARD' || message.category === 'APP_BUTTON_GROUP' || message.category === 'STRANGER');
  const bubblePresentation = messageBubblePresentation(message);
  const copyPolicy = messageCopyPolicy(message);
  const canSave = message.mediaStatus.toUpperCase() === 'DONE'
    && Boolean(message.mediaUrl)
    && ['IMAGE', 'VIDEO', 'AUDIO', 'DATA'].some((suffix) => isCategory(message.category, suffix));
  const canAddSticker = Boolean(message.stickerId) || copyPolicy.image;
  useEffect(() => {
    const element = messageElementRef.current;
    if (!element || message.mentionRead !== false || mentionMarkedRead.current) return;
    const observer = new IntersectionObserver((entries) => {
      const entry = entries[0];
      if (!entry?.isIntersecting || entry.intersectionRatio < 1 || mentionMarkedRead.current) return;
      mentionMarkedRead.current = true;
      void desktopApi.markMentionRead(message.conversationId, message.messageId).then(() =>
        Promise.all([
          queryClient.invalidateQueries({ queryKey: ['messages', message.conversationId] }),
          queryClient.invalidateQueries({ queryKey: ['unread-mention-message-ids', message.conversationId] }),
        ]),
      );
      observer.disconnect();
    }, { threshold: 1 });
    observer.observe(element);
    return () => observer.disconnect();
  }, [message.conversationId, message.mentionRead, message.messageId, queryClient]);
  if (bypassActions) {
    return <div className={classes("system-message")}><MessageContent message={message} currentUserId={currentUserId} mentionNames={mentionNames} /></div>;
  }
  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: ['messages', message.conversationId] });
    await queryClient.invalidateQueries({ queryKey: ['pinned-message-ids', message.conversationId] });
    await queryClient.invalidateQueries({ queryKey: ['pin-message-preview', message.conversationId] });
  }
  async function saveAttachment() {
    const destination = await save({ defaultPath: message.mediaName || undefined });
    if (destination) await desktopApi.saveAttachmentAs(message.messageId, destination);
  }
  async function addSticker() {
    try {
      if (message.stickerId) {
        await desktopApi.addSticker(message.stickerId);
        showToastSuccessful();
      } else {
        setShowAddImageSticker(true);
      }
    } catch (error) {
      showToastFailed(error);
    }
  }
  function handleContextMenu(event: MouseEvent<HTMLDivElement>) {
    if (selecting) {
      event.preventDefault();
      return;
    }
    const selectedText = window.getSelection()?.toString() || undefined;
    const menuCopyPolicy = messageCopyPolicy(message, selectedText);
    const entries: NativeMenuEntry[] = [
      ...(canReply ? [{ type: 'item' as const, id: 'reply', text: 'Reply', icon: flutterMenuIcon('reply'), action: () => setQuote(message) }] : []),
      ...(canReply && (menuCopyPolicy.text || menuCopyPolicy.image || menuCopyPolicy.caption || menuCopyPolicy.appCardSelection || isPinnedPage || canForward || canPin) ? [{ type: 'separator' as const }] : []),
      ...(menuCopyPolicy.text ? [{ type: 'item' as const, id: 'copy-text', text: menuCopyPolicy.selectedText ? 'Copy Selected Text' : 'Copy', icon: flutterMenuIcon('copy'), action: () => navigator.clipboard.writeText(menuCopyPolicy.selectedText || message.content) }] : []),
      ...(menuCopyPolicy.image ? [{ type: 'item' as const, id: 'copy-image', text: 'Copy Image', icon: flutterMenuIcon('copy'), action: () => copyMessageImage(message) }] : []),
      ...(menuCopyPolicy.caption ? [{ type: 'item' as const, id: 'copy-caption', text: menuCopyPolicy.captionSelectedText ? 'Copy Selected Text' : 'Copy Text', icon: flutterMenuIcon('copy'), action: () => navigator.clipboard.writeText(menuCopyPolicy.captionSelectedText || message.caption!) }] : []),
      ...(menuCopyPolicy.appCardSelection ? [{ type: 'item' as const, id: 'copy-app-card-selection', text: 'Copy Selected Text', icon: flutterMenuIcon('copy'), action: () => navigator.clipboard.writeText(menuCopyPolicy.selectedText!) }] : []),
      ...(isCategory(message.category, 'TEXT') ? [{ type: 'item' as const, id: 'generate-qr-code', text: 'Generate QR code', action: () => setShowQrCode(true) }] : []),
      ...((menuCopyPolicy.text || menuCopyPolicy.image || menuCopyPolicy.caption || menuCopyPolicy.appCardSelection || isCategory(message.category, 'TEXT')) && (isPinnedPage || canForward || canPin || canSave || canAddSticker || message.mediaStatus.toUpperCase() === 'CANCELED' || message.mediaStatus.toUpperCase() === 'PENDING' || canRecall || !isPinnedPage) ? [{ type: 'separator' as const }] : []),
      ...(isPinnedPage && onLocateToChat ? [{ type: 'item' as const, id: 'locate-to-chat', text: 'Locate to Chat', icon: flutterMenuIcon('positionToChat'), action: () => onLocateToChat(message.messageId) }] : []),
      ...(canForward ? [{ type: 'item' as const, id: 'forward', text: 'Forward', icon: flutterMenuIcon('forward'), action: () => openForward([message.messageId]) }] : []),
      { type: 'item' as const, id: 'select-message', text: 'Select', icon: flutterMenuIcon('select'), action: () => beginMessageSelection(message.conversationId, message.messageId) },
      ...(canPin ? [{ type: 'item' as const, id: 'toggle-pin-message', text: message.pinned ? 'Unpin' : 'Pin', icon: flutterMenuIcon(message.pinned ? 'unPin' : 'pin'), action: () => desktopApi.setMessagePinned(message.conversationId, message.messageId, !message.pinned).then(async () => { await refresh(); onPinnedChanged?.(); }) }] : []),
      ...((isPinnedPage || canForward || canPin) && (canSave || canAddSticker || message.mediaStatus.toUpperCase() === 'CANCELED' || message.mediaStatus.toUpperCase() === 'PENDING' || canRecall || !isPinnedPage) ? [{ type: 'separator' as const }] : []),
      ...(canSave ? [{ type: 'item' as const, id: 'save-attachment', text: 'Save as', icon: flutterMenuIcon('download'), action: () => saveAttachment() }] : []),
      ...(message.mediaStatus.toUpperCase() === 'CANCELED' && !mine ? [{ type: 'item' as const, id: 'download-attachment', text: 'Download', icon: flutterMenuIcon('download'), action: () => desktopApi.downloadAttachment(message.messageId).then(refresh) }] : []),
      ...(message.mediaStatus.toUpperCase() === 'CANCELED' && mine ? [{ type: 'item' as const, id: 'retry-attachment', text: 'Retry', action: () => desktopApi.retryAttachment(message.messageId).then(refresh) }] : []),
      ...(message.mediaStatus.toUpperCase() === 'PENDING' ? [{ type: 'item' as const, id: 'cancel-transfer', text: 'Cancel Transfer', action: () => desktopApi.cancelAttachment(message.messageId).then(refresh) }] : []),
      ...(canSave && canAddSticker ? [{ type: 'separator' as const }] : []),
      ...(canAddSticker ? [{ type: 'item' as const, id: 'add-sticker', text: 'Add Sticker', icon: flutterMenuIcon('sticker'), action: () => addSticker() }] : []),
      ...((canSave || canAddSticker || message.mediaStatus.toUpperCase() === 'CANCELED' || message.mediaStatus.toUpperCase() === 'PENDING') && (canRecall || !isPinnedPage) ? [{ type: 'separator' as const }] : []),
      ...(canRecall ? [{ type: 'item' as const, id: 'delete-for-everyone', text: 'Delete for Everyone', icon: flutterMenuIcon('recall'), action: () => desktopApi.recallMessages(message.conversationId, [message.messageId]).then(refresh) }] : []),
      ...(!isPinnedPage ? [{ type: 'item' as const, id: 'delete-for-me', text: 'Delete for Me', icon: flutterMenuIcon('delete'), action: () => desktopApi.deleteMessages(message.conversationId, [message.messageId]).then(refresh) }] : []),
    ];
    void showNativeContextMenu(event, entries);
  }
  return (
    <div ref={messageElementRef} className={classes(`message-row ${mine ? 'is-mine' : ''} ${groupedConversation ? 'is-group-row' : ''} ${sameUserPrevious ? 'is-continuation' : 'is-group-start'} ${focusMessageId === message.messageId ? 'is-highlighted' : ''} ${selecting ? 'is-selecting' : ''}`)} onContextMenu={handleContextMenu} onClick={selecting ? () => toggleMessageSelection(message.messageId) : undefined}>
      {selecting && <span className={classes(`message-select ${selectedMessageIds.includes(message.messageId) ? 'is-selected' : ''}`)}>{selectedMessageIds.includes(message.messageId) && '✓'}</span>}
      {!mine && shouldShowAvatar && (
        <button type="button" className={classes("message-avatar-slot")} aria-label={`${message.senderName} profile`} disabled={selecting || !showSender} onClick={() => onOpenUser(message.senderId)}>
          {showSender && <Avatar name={message.senderName} url={message.senderAvatarUrl} size={32} />}
        </button>
      )}
      <div className={classes("message-column")}>
        {showSender && <button type="button" className={classes("message-sender")} onClick={() => onOpenUser(message.senderId)} style={{ '--sender-color': messageNameColor(message.senderId) } as CSSProperties}><span>{message.senderName}</span><Badges verified={message.senderIsVerified} isBot={Boolean(message.senderIsBot)} membership={message.senderMembership} />{showIdentityNumber && message.senderIdentityNumber && message.senderIdentityNumber !== '0' && <small>@{message.senderIdentityNumber}</small>}</button>}
        <div className={classes(`message-bubble ${bubbleless ? 'is-transparent' : ''} ${message.category.includes('STICKER') ? 'is-sticker' : ''} ${showNip && !bubbleless ? 'has-nip' : ''} ${bubblePresentation.forceOtherBubbleColor ? 'uses-other-bubble' : ''}`)}>
          <MessageContent message={message} currentUserId={currentUserId} mentionNames={mentionNames} audioPlaylist={audioPlaylist} imagePlaylist={imagePlaylist} onLocateQuote={message.quoteMessageId ? () => (onLocateToChat ? onLocateToChat(message.quoteMessageId!) : focusMessage(message.quoteMessageId!)) : undefined} onOpenUser={onOpenUser} />
          {!bubbleless && !bubblePresentation.outerMetadata && (
            <span className={classes("message-meta")}>
              {message.pinned && <Pin size={8} aria-label="Pinned" />}
              {(message.category.startsWith('SIGNAL_') || message.category.startsWith('ENCRYPTED_')) && <SecretMetadataIcon />}
              <time>{formatMessageTime(message.createdAtMicros)}</time>
              {mine && <MessageStatus status={message.status} />}
            </span>
          )}
        </div>
        {bubblePresentation.outerMetadata && (
          <span className={classes("message-meta message-meta--outer")}>
            {message.pinned && <Pin size={8} aria-label="Pinned" />}
            {(message.category.startsWith('SIGNAL_') || message.category.startsWith('ENCRYPTED_')) && <SecretMetadataIcon />}
            <time>{formatMessageTime(message.createdAtMicros)}</time>
            {mine && !bubblePresentation.hideOuterStatus && <MessageStatus status={message.status} />}
          </span>
        )}
      </div>
      {showQrCode && <MessageQrDialog content={message.content} onClose={() => setShowQrCode(false)} />}
      {showAddImageSticker && <AddImageStickerDialog message={message} onClose={() => setShowAddImageSticker(false)} />}
    </div>
  );
});

function MessageQrDialog({ content, onClose }: { content: string; onClose: () => void }) {
  return <div className={classes("modal-backdrop message-qr-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("message-qr-dialog")} role="dialog" aria-modal="true" aria-label="Message QR code" onMouseDown={(event) => event.stopPropagation()}><div className={classes("message-qr-code")}><QRCodeSVG value={content} size={224} level="Q" marginSize={0} /></div><button type="button" onClick={onClose}>Confirm</button></section></div>;
}

function AddImageStickerDialog({ message, onClose }: { message: Message; onClose: () => void }) {
  const [saving, setSaving] = useState(false);
  const source = resolveAssetUrl(message.mediaUrl);
  async function confirm() {
    if (saving) return;
    setSaving(true);
    showToastLoading();
    try {
      await desktopApi.addStickerFromFile(message.messageId);
      showToastSuccessful();
      onClose();
    } catch (error) {
      showToastFailed(error);
    } finally {
      setSaving(false);
    }
  }
  return <div className={classes("modal-backdrop add-image-sticker-backdrop")} role="presentation" onMouseDown={() => !saving && onClose()}><section className={classes("add-image-sticker-dialog")} role="dialog" aria-modal="true" aria-label="Add Sticker" onMouseDown={(event) => event.stopPropagation()}><header><span>Add Sticker</span><IconButton label="Close add sticker" onClick={onClose} disabled={saving}><X size={20} /></IconButton></header><main><div>{source && <img src={source} alt={message.mediaName || 'Image sticker'} />}</div></main><footer><button type="button" disabled={saving} onClick={() => void confirm()}>Save</button></footer></section></div>;
}

function MessageStatus({ status }: { status: string }) {
  const normalized = status.toUpperCase();
  if (normalized === 'SENT') return <Check className={classes("message-status")} size={13} />;
  if (normalized === 'DELIVERED' || normalized === 'READ') return <CheckCheck className={classes(`message-status ${normalized === 'READ' ? 'is-read' : ''}`)} size={13} />;
  return <span className={classes("message-status is-sending")} />;
}

function SecretMetadataIcon() {
  return <svg className={classes("message-secret-icon")} width="8" height="8" viewBox="0 0 8 8" aria-label="Encrypted"><path d="M2.1 3.4V2.6a1.9 1.9 0 1 1 3.8 0v.8M1.3 3.4h5.4v3.3H1.3zM4 4.7v1.1" fill="none" stroke="currentColor" strokeWidth=".8" strokeLinecap="round" strokeLinejoin="round" /></svg>;
}
