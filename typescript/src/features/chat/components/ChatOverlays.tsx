import {
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  AtSign,
  Copy,
  Forward,
  Pin,
  Trash2,
  X,
} from 'lucide-react';
import { desktopApi } from '@/shared/api/desktopApi';
import { useDesktopUi } from '@/state/desktopUi';
import type { Conversation, Message, UserProfile } from '@/shared/model/types';
import { IconButton } from '@/shared/ui/IconButton';
import { UserProfileDialog } from '@/features/profile/components/UserProfileDialog';
import { MessageRow } from '@/features/chat/components/MessageRow';
import { showNativeContextMenu } from '@/shared/ui/nativeMenu';
import {
  canCombineForwardMessage,
  canForwardMessage,
  formatDayTime,
  sameLocalDay,
} from '@/features/chat/model/message';
import { classes } from '@/shared/styles/classes';

export function ChatOverlays({ conversation, currentUserId, showJumpToLatest, jumpToLatest }: { conversation: Conversation; currentUserId: string; showJumpToLatest: boolean; jumpToLatest: () => void }) {
  const queryClient = useQueryClient();
  const focusMessage = useDesktopUi((state) => state.focusMessage);
  const [showPinPreview, setShowPinPreview] = useState(true);
  const [showScamWarning, setShowScamWarning] = useState(true);
  const [pinsOpen, setPinsOpen] = useState(false);
  const pinnedIds = useQuery({ queryKey: ['pinned-message-ids', conversation.conversationId], queryFn: () => desktopApi.pinnedMessageIds(conversation.conversationId) });
  const preview = useQuery({ queryKey: ['pin-message-preview', conversation.conversationId], queryFn: () => desktopApi.pinMessagePreview(conversation.conversationId) });
  const mentions = useQuery({ queryKey: ['unread-mention-message-ids', conversation.conversationId], queryFn: () => desktopApi.unreadMentionMessageIds(conversation.conversationId) });
  useEffect(() => { setShowPinPreview(true); setShowScamWarning(true); }, [conversation.conversationId]);
  const pinId = pinnedIds.data?.[0];
  async function clearMentions() {
    await Promise.all((mentions.data ?? []).map((messageId) => desktopApi.markMentionRead(conversation.conversationId, messageId)));
    await queryClient.invalidateQueries({ queryKey: ['unread-mention-message-ids', conversation.conversationId] });
  }
  return <div className={classes("chat-overlays")}>
    {showPinPreview && pinId && <div className={classes("pinned-banner")}><div className={classes("pinned-preview-bubble")}><button type="button" aria-label="Dismiss pinned message" onClick={() => setShowPinPreview(false)}>×</button><button type="button" className={classes("pinned-preview")} onClick={() => focusMessage(pinId)}><span>📌</span>{preview.data ? `${preview.data.senderName}: ${preview.data.content}` : 'Pinned message'}</button></div><button type="button" className={classes("pinned-count")} aria-label="Open pinned messages" onClick={() => setPinsOpen(true)}><Pin size={17} /><span>{pinnedIds.data?.length}</span></button></div>}
    {showScamWarning && conversation.isScam && <div className={classes("scam-warning")}><AlertTriangle size={26} /><span>This contact may be a scam. Stay alert and never share your verification code.</span><button type="button" aria-label="Dismiss warning" onClick={() => setShowScamWarning(false)}>×</button></div>}
    <div className={classes("jump-controls")}>
      {(mentions.data?.length ?? 0) > 0 && <button type="button" className={classes("jump-mention")} onClick={() => focusMessage(mentions.data![0])} onContextMenu={(event) => void showNativeContextMenu(event, [{ type: 'item', id: 'clear-mentions', text: 'Clear', action: clearMentions }])}><AtSign size={18} /><b>{mentions.data!.length > 99 ? '99+' : mentions.data!.length}</b></button>}
      {showJumpToLatest && <button type="button" className={classes("jump-current")} aria-label="Jump to latest message" onClick={jumpToLatest}>↓</button>}
    </div>
    {pinsOpen && <PinnedMessagesDialog conversation={conversation} currentUserId={currentUserId} pinnedIds={pinnedIds.data ?? []} onClose={() => setPinsOpen(false)} onLocate={(messageId) => { setPinsOpen(false); focusMessage(messageId); }} onChanged={() => void queryClient.invalidateQueries({ queryKey: ['pinned-message-ids', conversation.conversationId] })} />}
  </div>;
}

function PinnedMessagesDialog({ conversation, currentUserId, pinnedIds, onClose, onLocate, onChanged }: { conversation: Conversation; currentUserId: string; pinnedIds: string[]; onClose: () => void; onLocate: (messageId: string) => void; onChanged: () => void }) {
  const [confirmingUnpinAll, setConfirmingUnpinAll] = useState(false);
  const [unpinning, setUnpinning] = useState(false);
  const messages = useQuery({
    queryKey: ['pinned-messages', conversation.conversationId, pinnedIds],
    queryFn: async () => {
      const items = await Promise.all(pinnedIds.map(async (messageId) => {
      const page = await desktopApi.messagesAround(conversation.conversationId, messageId);
      return page.find((message) => message.messageId === messageId);
      }));
      return items.filter((message): message is Message => Boolean(message)).sort((left, right) => left.createdAtMicros - right.createdAtMicros);
    },
  });
  const participants = useQuery({ queryKey: ['conversation-participants', conversation.conversationId], queryFn: () => desktopApi.conversationParticipants(conversation.conversationId), enabled: conversation.category === 'GROUP' });
  const currentUserRole = participants.data?.find((participant) => participant.userId === currentUserId)?.role;
  const mentionContents = useMemo(() => (messages.data ?? []).flatMap((message) => [message.content, message.caption].filter((value): value is string => Boolean(value))).filter((content) => /@\d{4,}/.test(content)), [messages.data]);
  const mentionNames = useQuery({ queryKey: ['mention-names', conversation.conversationId, mentionContents], queryFn: () => desktopApi.mentionNames(mentionContents), enabled: mentionContents.length > 0 });
  const showAvatar = useDesktopUi((state) => state.uiSettings.showAvatar);
  const [profileDialog, setProfileDialog] = useState<UserProfile>();
  async function openUserProfile(userId: string) {
    setProfileDialog(await desktopApi.userProfile(userId) ?? undefined);
  }
  async function unpinAll() {
    if (unpinning) return;
    setUnpinning(true);
    try {
      for (const messageId of pinnedIds) await desktopApi.setMessagePinned(conversation.conversationId, messageId, false);
      onChanged(); onClose();
    } finally { setUnpinning(false); }
  }
  const entries = messages.data ?? [];
  return <div className={classes("modal-backdrop pinned-messages-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("pinned-messages-dialog")} role="dialog" aria-modal="true" aria-label="Pinned Messages" onMouseDown={(event) => event.stopPropagation()}><header><IconButton label="Close pinned messages" onClick={onClose}><X size={20} /></IconButton><strong>Pinned Messages ({pinnedIds.length})</strong><span /></header><div className={classes("pinned-message-list")}>{messages.isPending ? <p className={classes("chat-state")}>Loading…</p> : entries.map((message, index) => { const previous = entries[index - 1]; const next = entries[index + 1]; const showDayTime = !previous || !sameLocalDay(previous.createdAtMicros, message.createdAtMicros); return <div className={classes("pinned-message-row")} key={message.messageId}>{showDayTime && <div className={classes("message-day-time")}>{formatDayTime(message.createdAtMicros)}</div>}<MessageRow message={message} previous={previous} next={next} currentUserId={currentUserId} currentUserRole={currentUserRole} showAvatar={showAvatar} conversation={conversation} mentionNames={mentionNames.data ?? {}} audioPlaylist={entries} imagePlaylist={entries} onOpenUser={(userId) => void openUserProfile(userId)} isPinnedPage onLocateToChat={onLocate} onPinnedChanged={onChanged} /></div>; })}</div><button className={classes("unpin-all")} type="button" disabled={unpinning || pinnedIds.length === 0} onClick={() => setConfirmingUnpinAll(true)}>Unpin All Messages</button>{confirmingUnpinAll && <div className={classes("pinned-confirmation")} role="alertdialog" aria-modal="true"><p>Unpin all messages?</p><footer><button type="button" onClick={() => setConfirmingUnpinAll(false)}>Cancel</button><button type="button" onClick={() => void unpinAll()}>Confirm</button></footer></div>}{profileDialog && <UserProfileDialog profile={profileDialog} currentUserId={currentUserId} onClose={() => setProfileDialog(undefined)} onChanged={async () => setProfileDialog(await desktopApi.userProfile(profileDialog.userId) ?? undefined)} onChat={async () => { const conversationId = await desktopApi.openUserConversation(profileDialog.userId); useDesktopUi.getState().selectConversation(conversationId); setProfileDialog(undefined); }} onInfo={async () => { const conversationId = await desktopApi.openUserConversation(profileDialog.userId); useDesktopUi.getState().selectConversation(conversationId); useDesktopUi.getState().toggleInfo(); setProfileDialog(undefined); }} />}</section></div>;
}

export function ChatSelectionBar({ messages, selectedMessageIds, onCombineForward, onForward, onCopy, onDelete }: { messages: Message[]; selectedMessageIds: string[]; onCombineForward: () => void; onForward: () => void; onCopy: () => void; onDelete: () => void }) {
  const hasAllMessages = messages.length === selectedMessageIds.length;
  const canForward = hasAllMessages && selectedMessageIds.length < 100 && messages.every(canForwardMessage);
  const canCombineForward = hasAllMessages && selectedMessageIds.length >= 2 && selectedMessageIds.length < 100 && messages.every(canCombineForwardMessage);
  const canCopy = hasAllMessages && selectedMessageIds.length > 0;
  return <div className={classes("selection-toolbar")}>
    <SelectionAction label="Combine and Forward" icon={<Forward size={24} />} disabled={!canCombineForward} onClick={onCombineForward} />
    <SelectionAction label="One-by-one Forward" icon={<Forward size={24} />} disabled={!canForward} onClick={onForward} />
    <SelectionAction label="Copy" icon={<Copy size={24} />} disabled={!canCopy} onClick={onCopy} />
    <SelectionAction label="Delete" icon={<Trash2 size={24} />} disabled={selectedMessageIds.length === 0} onClick={onDelete} />
  </div>;
}

function SelectionAction({ label, icon, disabled, onClick }: { label: string; icon: ReactNode; disabled: boolean; onClick: () => void }) {
  return <button type="button" disabled={disabled} onClick={onClick}>{icon}<span>{label}</span></button>;
}

export function DeleteSelectedMessagesDialog({ count, canRecall, onClose, onDelete, onRecall }: { count: number; canRecall: boolean; onClose: () => void; onDelete: () => void; onRecall: () => void }) {
  return <div className={classes("modal-backdrop")}><section className={classes("circle-dialog delete-circle-dialog")} role="alertdialog" aria-modal="true"><header><strong>Delete Messages</strong></header><p>Delete {count} selected message{count === 1 ? '' : 's'}?</p><footer><button type="button" onClick={onClose}>Cancel</button><button type="button" className={classes("is-danger")} onClick={onDelete}>Delete</button>{canRecall && <button type="button" className={classes("is-danger")} onClick={onRecall}>Delete for Everyone</button>}</footer></section></div>;
}
