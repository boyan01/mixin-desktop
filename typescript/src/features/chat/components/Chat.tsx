import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent,
} from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { openBotWebview } from '@/shared/platform/botWebview';
import { useDesktopUi } from '@/state/desktopUi';
import type { Conversation, Message } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { IconButton } from '@/shared/ui/IconButton';
import { ChatComposer } from '@/features/chat/components/ChatComposer';
import {
  ChatOverlays,
  ChatSelectionBar,
  DeleteSelectedMessagesDialog,
} from '@/features/chat/components/ChatOverlays';
import { ChatTimeline } from '@/features/chat/components/ChatTimeline';
import { canRecallSelectionMessage } from '@/features/chat/model/message';
import { droppedAttachmentPaths } from '@/features/chat/model/composer';
import chatBackgroundUrl from '@flutter-assets/images/chat_background.png';
import chatSearchAsset from '@flutter-assets/images/ic_search.svg';
import chatInfoAsset from '@flutter-assets/images/ic_screen.svg';
import backAsset from '@flutter-assets/images/ic_back.svg';
import botAsset from '@flutter-assets/images/bot.svg';
import { classes } from '@/shared/styles/classes';

export {
  CHAT_COMPOSER_INPUT_MAX_HEIGHT,
  CHAT_COMPOSER_INPUT_MIN_HEIGHT,
  CHAT_COMPOSER_MAX_TEXT_LENGTH,
  CHAT_COMPOSER_MIN_HEIGHT,
  attachmentMimeType,
  attachmentPreviewMode,
  composerInputHeight,
  droppedAttachmentPaths,
  formatAttachmentBytes,
  resizeComposerTextArea,
} from '@/features/chat/model/composer';
export {
  CHAT_MESSAGE_FOCUS_ANCHOR,
  CHAT_PRELOAD_VIEWPORT_COUNT,
  CHAT_VIEWPORT_CACHE_EXTENT,
  centeredTimelineWindow,
  centeredViewportBounds,
  chatFocusScrollOffset,
  chatTailState,
  chatWheelBoundaryPolicy,
  firstVisibleTimelineIndex,
  preservedPrependScrollTop,
  renderedTimelineMessageIds,
  shouldPreloadTimelinePage,
  viewportCacheOverscanRows,
} from '@/features/chat/model/timeline';
export {
  actionButtonRows,
  mapTilePosition,
  messageBubblePresentation,
  messageCopyPolicy,
} from '@/features/chat/model/message';
export { imagePreviewEntries } from '@/features/chat/components/MessageContent';

interface ChatProps {
  conversation?: Conversation;
  currentUserId: string;
}

export function Chat({ conversation, currentUserId }: ChatProps) {
  const selectConversation = useDesktopUi((state) => state.selectConversation);
  const toggleInfo = useDesktopUi((state) => state.toggleInfo);
  const openMessageSearch = useDesktopUi((state) => state.openMessageSearch);
  const infoOpen = useDesktopUi((state) => state.infoOpen);
  const messageSearchOpen = useDesktopUi((state) => state.messageSearchOpen);
  const selectedMessageIds = useDesktopUi((state) => state.selectedMessageIds);
  const selectionConversationId = useDesktopUi((state) => state.selectionConversationId);
  const clearMessageSelection = useDesktopUi((state) => state.clearMessageSelection);
  const openForward = useDesktopUi((state) => state.openForward);
  const queryClient = useQueryClient();
  const [confirmingSelectionDelete, setConfirmingSelectionDelete] = useState(false);
  const [selectionMessages, setSelectionMessages] = useState<Message[]>([]);
  const [dragging, setDragging] = useState(false);
  const [droppedPaths, setDroppedPaths] = useState<string[]>();
  const [dropDialogEnabled, setDropDialogEnabled] = useState(true);
  const dragDepth = useRef(0);

  useEffect(() => {
    if (conversation?.unseenCount) {
      void desktopApi.markConversationRead(conversation.conversationId);
    }
  }, [conversation?.conversationId, conversation?.unseenCount]);

  useEffect(() => {
    if (!conversation || selectionConversationId !== conversation.conversationId || selectedMessageIds.length === 0) {
      setSelectionMessages([]);
      return;
    }
    let cancelled = false;
    void Promise.all(selectedMessageIds.map(async (messageId) => {
      const page = await desktopApi.messagesAround(conversation.conversationId, messageId);
      return page.find((message) => message.messageId === messageId);
    })).then((messages) => {
      if (!cancelled) setSelectionMessages(messages.filter((message): message is Message => Boolean(message)));
    }).catch((reason) => {
      e('Unable to load selected messages', reason);
      if (!cancelled) setSelectionMessages([]);
    });
    return () => { cancelled = true; };
  }, [conversation, selectedMessageIds, selectionConversationId]);

  if (!conversation) {
    return (
      <main className={classes("chat-pane empty-chat")}>
        <p>Pick a conversation</p>
      </main>
    );
  }
  const activeConversation = conversation;

  const subtitle = activeConversation.category === 'GROUP'
    ? `${activeConversation.participantCount} participants`
    : activeConversation.identityNumber;
  function onDragEnter(event: DragEvent<HTMLDivElement>) {
    if (!dropDialogEnabled || selectionConversationId === activeConversation.conversationId || !event.dataTransfer.types.includes('Files')) return;
    dragDepth.current += 1;
    setDragging(true);
  }
  function onDragLeave(event: DragEvent<HTMLDivElement>) {
    if (!dropDialogEnabled || !event.dataTransfer.types.includes('Files')) return;
    dragDepth.current = Math.max(0, dragDepth.current - 1);
    if (dragDepth.current === 0) setDragging(false);
  }
  function onDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    dragDepth.current = 0;
    setDragging(false);
    if (!dropDialogEnabled || selectionConversationId === activeConversation.conversationId) return;
    const paths = droppedAttachmentPaths(Array.from(event.dataTransfer.files) as Array<File & { path?: string }>);
    if (paths.length) {
      setDropDialogEnabled(false);
      setDroppedPaths(paths);
    }
  }
  return (
    <main className={classes("chat-pane")}>
      <header className={classes("chat-header")} data-tauri-drag-region="deep">
        <IconButton label="Back" className={classes("chat-back")} onClick={() => selectConversation()}>
          <FlutterAssetIcon src={backAsset} />
        </IconButton>
        <button type="button" className={classes("chat-conversation-avatar")} aria-label="Conversation info" onClick={toggleInfo}><Avatar name={conversation.name} url={conversation.avatarUrl} size={36} /></button>
        <div className={classes("chat-title")}>
          <strong>{conversation.name}<Badges verified={conversation.isVerified} isBot={conversation.isBot} membership={conversation.membership} /></strong>
          {subtitle && <small>{subtitle}</small>}
        </div>
        {conversation.isBot && <IconButton label="Open bot home page" onClick={() => void desktopApi.botHomeUri(conversation.ownerId).then((uri) => { if (uri) return openBotWebview({ url: uri, title: conversation.name, conversationId: conversation.conversationId }); })}><FlutterAssetIcon src={botAsset} /></IconButton>}
        {selectionConversationId === conversation.conversationId ? <button type="button" className={classes("chat-selection-cancel")} onClick={clearMessageSelection}>Cancel</button> : <><IconButton label="Search messages" onClick={openMessageSearch}><FlutterAssetIcon src={chatSearchAsset} /></IconButton>{!infoOpen && !messageSearchOpen && <IconButton label="Conversation info" onClick={toggleInfo}><FlutterAssetIcon src={chatInfoAsset} /></IconButton>}</>}
      </header>
      <div className={classes("chat-drop-target")} style={{ '--chat-background-image': `url("${chatBackgroundUrl}")` } as CSSProperties} onDragEnter={onDragEnter} onDragOver={(event) => { if (dropDialogEnabled) event.preventDefault(); }} onDragLeave={onDragLeave} onDrop={onDrop}>
        <ChatTimeline conversation={conversation} currentUserId={currentUserId} renderOverlays={(controls) => <ChatOverlays conversation={conversation} currentUserId={currentUserId} {...controls} />} />
        {selectionConversationId === conversation.conversationId ? <ChatSelectionBar messages={selectionMessages} selectedMessageIds={selectedMessageIds} onCombineForward={() => { openForward(selectedMessageIds, true); clearMessageSelection(); }} onForward={() => { openForward(selectedMessageIds); clearMessageSelection(); }} onCopy={() => { void navigator.clipboard.writeText(selectionMessages.map((message) => `${message.senderName}, (${new Intl.DateTimeFormat(undefined, { dateStyle: 'short', timeStyle: 'medium' }).format(new Date(message.createdAtMicros / 1000))}):\n${message.category.endsWith('_TEXT') ? message.content : `[${message.category}]`}`).join('\n\n')).then(clearMessageSelection); }} onDelete={() => setConfirmingSelectionDelete(true)} /> : conversation.category === 'GROUP' && conversation.participantCount === 0 ? <div className={classes("group-cannot-send")}>You can no longer send messages in this group.</div> : <ChatComposer conversation={conversation} currentUserId={currentUserId} droppedPaths={droppedPaths} onDroppedPathsHandled={() => { setDroppedPaths(undefined); setDropDialogEnabled(true); }} />}
        {dragging && <div className={classes("chat-drop-indicator")}><span>Add File</span></div>}
      </div>
      {confirmingSelectionDelete && <DeleteSelectedMessagesDialog count={selectedMessageIds.length} canRecall={selectionMessages.length === selectedMessageIds.length && selectionMessages.every((message) => canRecallSelectionMessage(message, conversation, currentUserId))} onClose={() => setConfirmingSelectionDelete(false)} onDelete={() => void desktopApi.deleteMessages(conversation.conversationId, selectedMessageIds).then(async () => { clearMessageSelection(); setConfirmingSelectionDelete(false); await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] }); })} onRecall={() => void desktopApi.recallMessages(conversation.conversationId, selectedMessageIds).then(async () => { clearMessageSelection(); setConfirmingSelectionDelete(false); await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] }); })} />}
    </main>
  );
}

function FlutterAssetIcon({ src }: { src: string }) {
  return <span className={classes("flutter-asset-icon")} aria-hidden="true" style={{ '--flutter-icon-image': `url("${src}")` } as CSSProperties} />;
}
