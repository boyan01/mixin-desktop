import { cloneElement, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Check, CirclePlus, Menu, MessageSquarePlus, Pause, Play, Search, UsersRound, X } from 'lucide-react';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { IconButton } from '@/shared/ui/IconButton';
import { SearchTextField } from '@/shared/ui/SearchTextField';
import { CreateDialog } from '@/features/conversations/components/CreateDialogs';
import { CustomPopupMenuButton } from '@/shared/ui/CustomPopupMenu';
import { AlertDialogLayout, MixinButton, MixinDialog } from '@/shared/ui/MixinDialog';
import { useDesktopUi } from '@/state/desktopUi';
import { desktopApi } from '@/shared/api/desktopApi';
import type { Circle, Conversation, Message, UserProfile } from '@/shared/model/types';
import { conversationPreview, matchesCategory } from '@/features/conversations/lib/conversationPresentation';
import { useAudioPlayback } from '@/shared/media/audioPlayback';
import emptyAsset from '@flutter-assets/images/empty_file.svg';
import filterUnseenAsset from '@flutter-assets/images/filter_unseen.svg';
import addAsset from '@flutter-assets/images/ic_add.svg';
import imageAsset from '@flutter-assets/images/image.svg';
import videoAsset from '@flutter-assets/images/video.svg';
import liveAsset from '@flutter-assets/images/live.svg';
import audioAsset from '@flutter-assets/images/audio.svg';
import stickerAsset from '@flutter-assets/images/sticker.svg';
import fileAsset from '@flutter-assets/images/file.svg';
import contactAsset from '@flutter-assets/images/contact.svg';
import transferAsset from '@flutter-assets/images/transfer.svg';
import locationAsset from '@flutter-assets/images/location.svg';
import videoCallAsset from '@flutter-assets/images/video_call.svg';
import recallAsset from '@flutter-assets/images/recall.svg';
import sentAsset from '@flutter-assets/images/sent.svg';
import deliveredAsset from '@flutter-assets/images/delivered.svg';
import readAsset from '@flutter-assets/images/read.svg';
import muteAsset from '@flutter-assets/images/mute.svg';
import pinAsset from '@flutter-assets/images/pin.svg';
import { classes } from '@/shared/styles/classes';
import { flutterMenuIcon } from '@/shared/ui/flutterMenuIcon';
import { showNativeContextMenu } from '@/shared/ui/nativeMenu';

interface ConversationListProps {
  conversations: Conversation[];
  currentUserId: string;
  connected?: boolean;
  onOpenUrl?: (url: string) => void;
  onOpenSidebar?: () => void;
}

function formatTime(timestamp: number): string {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  const now = new Date();
  if (date.toDateString() === now.toDateString()) {
    return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(date);
  }
  const days = Math.floor((now.getTime() - date.getTime()) / 86_400_000);
  if (days < 7) return new Intl.DateTimeFormat(undefined, { weekday: 'short' }).format(date);
  return new Intl.DateTimeFormat(undefined, { month: 'numeric', day: 'numeric' }).format(date);
}

export function ConversationList({ conversations, currentUserId, connected = false, onOpenUrl, onOpenSidebar }: ConversationListProps) {
  const queryClient = useQueryClient();
  const createMode = useDesktopUi((state) => state.createMode);
  const openCreate = useDesktopUi((state) => state.openCreate);
  const closeCreate = useDesktopUi((state) => state.closeCreate);
  const selectedId = useDesktopUi((state) => state.selectedConversationId);
  const selectConversation = useDesktopUi((state) => state.selectConversation);
  const focusMessage = useDesktopUi((state) => state.focusMessage);
  const category = useDesktopUi((state) => state.category);
  const circleId = useDesktopUi((state) => state.circleId);
  const query = useDesktopUi((state) => state.query);
  const setQuery = useDesktopUi((state) => state.setQuery);
  const unseenOnly = useDesktopUi((state) => state.unseenOnly);
  const toggleUnseenOnly = useDesktopUi((state) => state.toggleUnseenOnly);
  const menuConversationAction = useDesktopUi((state) => state.menuConversationAction);
  const clearMenuConversationAction = useDesktopUi((state) => state.clearMenuConversationAction);
  const circles = useQuery({ queryKey: ['circles'], queryFn: desktopApi.circles }).data ?? [];
  const [muting, setMuting] = useState<Conversation>();
  const [deleting, setDeleting] = useState<Conversation>();
  const connectedBefore = useRef(false);
  if (connected) connectedBefore.current = true;

  const visible = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase();
    return conversations
      .filter((conversation) => matchesCategory(conversation, category, circleId))
      .filter((conversation) => !unseenOnly || conversation.unseenCount > 0)
      .filter((conversation) => {
        if (!keyword) return true;
        return conversation.name.toLocaleLowerCase().includes(keyword)
          || conversation.identityNumber.includes(keyword)
          || conversation.lastMessage.toLocaleLowerCase().includes(keyword);
      })
      .sort((left, right) => {
        if (left.isPinned !== right.isPinned) return left.isPinned ? -1 : 1;
        const leftTime = left.pinTimeMillis || left.updatedAtMillis;
        const rightTime = right.pinTimeMillis || right.updatedAtMillis;
        return rightTime - leftTime;
      });
  }, [category, circleId, conversations, query, unseenOnly]);
  const searching = query.trim().length > 0;

  useEffect(() => {
    if (!menuConversationAction || !selectedId) return;
    const conversation = conversations.find((item) => item.conversationId === selectedId);
    clearMenuConversationAction();
    if (!conversation) return;
    if (menuConversationAction === 'mute') setMuting(conversation);
    else setDeleting(conversation);
  }, [clearMenuConversationAction, conversations, menuConversationAction, selectedId]);

  return (
    <section className={classes("conversation-pane")}>
      <div className={classes("conversation-toolbar")} data-tauri-drag-region="deep">
        {onOpenSidebar
          ? <IconButton className={classes("mobile-drawer-button")} label="Open navigation" onClick={onOpenSidebar}><Menu size={20} /></IconButton>
          : <span className={classes("mobile-drawer-placeholder")} aria-hidden="true" />}
        <SearchTextField
          className={classes("conversation-search-field")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onClear={() => setQuery('')}
          onKeyDown={(event) => { if (event.key === 'Escape') { setQuery(''); event.currentTarget.blur(); } }}
          placeholder={unseenOnly ? 'Search unread' : `Search (${navigator.platform.includes('Mac') ? '⌘' : 'Ctrl '}K)`}
          aria-label="Search conversations"
        />
        <IconButton
          label={unseenOnly ? 'Show all conversations' : 'Show unread conversations'}
          className={classes(unseenOnly ? 'is-active' : '')}
          onClick={toggleUnseenOnly}
        >
          <ConversationToolbarAsset src={filterUnseenAsset} />
        </IconButton>
        <CustomPopupMenuButton
          align="end"
          trigger={<span><IconButton label="New conversation"><ConversationToolbarAsset src={addAsset} /></IconButton></span>}
          onSelected={openCreate}
          items={[
            { value: 'search-contact', title: 'Search Contacts', icon: <Search size={20} /> },
            { value: 'conversation', title: 'New Conversation', icon: <MessageSquarePlus size={20} /> },
            { value: 'group', title: 'New Group', icon: <UsersRound size={20} /> },
            { value: 'circle', title: 'New Circle', icon: <CirclePlus size={20} /> },
          ]}
        />
      </div>
      <NetworkStatus connected={connected} connectedBefore={connectedBefore.current} />
      <div className={classes("conversation-list")} role="listbox" aria-label="Conversations">
        {searching ? <ConversationSearchResults query={query} category={category} conversations={visible} messageConversations={conversations} currentUserId={currentUserId} onOpenUrl={onOpenUrl} onConversationSelected={selectConversation} onMessageSelected={(message) => { focusMessage(message.messageId); selectConversation(message.conversationId); }} /> : visible.map((conversation) => {
          const selected = selectedId === conversation.conversationId;
          const text = conversationPreview(conversation, currentUserId);
          return <ConversationContextMenu key={conversation.conversationId} conversation={conversation} circles={circles} currentCircleId={category === 'circle' ? circleId : undefined} onMute={() => setMuting(conversation)} onDelete={() => setDeleting(conversation)} onChanged={() => void queryClient.invalidateQueries({ queryKey: ['bootstrap'] })}>
            <button type="button" role="option" aria-selected={selected} className={classes("conversation-row")} onClick={() => selectConversation(conversation.conversationId)}>
              <span className={classes(selected ? 'conversation-row-surface is-selected' : 'conversation-row-surface')}>
                <Avatar name={conversation.name} url={conversation.avatarUrl} size={50} />
                  <span className={classes("conversation-copy")}>
                  <span className={classes("conversation-title-line")}><strong>{conversation.name}<Badges verified={conversation.isVerified} isBot={conversation.isBot} membership={conversation.membership} /></strong><time>{formatTime(conversation.updatedAtMillis)}</time></span>
                  <span className={classes("conversation-subtitle-line")}>
                    <ConversationPreview conversation={conversation} currentUserId={currentUserId} text={text} />
                    <ConversationIndicators conversation={conversation} />
                  </span>
                </span>
              </span>
            </button>
          </ConversationContextMenu>;
        })}
        {visible.length === 0 && !searching && <EmptyConversationState text={unseenOnly ? 'No chats, contacts or messages found.' : 'No Data'} />}
      </div>
      <AudioPlayerBar conversations={conversations} selectedConversationId={selectedId} onSelect={selectConversation} />
      <CreateDialog
        mode={createMode}
        conversations={conversations}
        onClose={closeCreate}
        onConversationOpened={(conversationId) => {
          selectConversation(conversationId);
          void queryClient.invalidateQueries({ queryKey: ['bootstrap'] });
        }}
        onCircleCreated={() => void queryClient.invalidateQueries({ queryKey: ['circles'] })}
      />
      {muting && <MuteDialog conversation={muting} onClose={() => setMuting(undefined)} onMuted={() => void queryClient.invalidateQueries({ queryKey: ['bootstrap'] })} />}
      {deleting && <DeleteConversationDialog conversation={deleting} onClose={() => setDeleting(undefined)} onDeleted={() => { if (selectedId === deleting.conversationId) selectConversation(); void queryClient.invalidateQueries({ queryKey: ['bootstrap'] }); }} />}
    </section>
  );
}

function EmptyConversationState({ text }: { text: string }) {
  return <div className={classes("conversation-empty-state")}><span className={classes("conversation-empty-icon")} style={{ '--conversation-empty-image': `url("${emptyAsset}")` } as React.CSSProperties} /><span>{text}</span></div>;
}

function ConversationToolbarAsset({ src }: { src: string }) {
  return <span className={classes("conversation-toolbar-asset")} aria-hidden="true" style={{ '--conversation-toolbar-icon-image': `url("${src}")` } as React.CSSProperties} />;
}

function ConversationPreview({ conversation, currentUserId, text }: { conversation: Conversation; currentUserId: string; text: string }) {
  const hasDraft = Boolean(conversation.draft && conversation.status !== 3);
  const asset = hasDraft ? undefined : messagePreviewAsset(conversation);
  return <span className={classes("conversation-preview")}>
    {!hasDraft && <ConversationMessageStatus conversation={conversation} currentUserId={currentUserId} />}
    {asset && <span className={classes("conversation-preview-icon")} aria-hidden="true" style={{ '--conversation-preview-icon-image': `url("${asset}")` } as React.CSSProperties} />}
    <span>{hasDraft && <><b className={classes("draft-preview")}>Draft:</b> </>}{text}</span>
  </span>;
}

function messagePreviewAsset(conversation: Conversation): string | undefined {
  const category = conversation.lastMessageCategory || '';
  if (category === 'SYSTEM_SAFE_INSCRIPTION' || category.includes('SNAPSHOT')) return transferAsset;
  if (category.includes('TRANSCRIPT') || category.includes('DATA') || category.includes('POST')) return fileAsset;
  if (category.includes('IMAGE')) return imageAsset;
  if (category.includes('VIDEO')) return videoAsset;
  if (category.includes('LIVE')) return liveAsset;
  if (category.includes('AUDIO')) return audioAsset;
  if (category.includes('STICKER')) return stickerAsset;
  if (category.includes('CONTACT')) return contactAsset;
  if (category.includes('LOCATION')) return locationAsset;
  if (category.startsWith('WEBRTC') || category.startsWith('KRAKEN')) return videoCallAsset;
  if (category.includes('RECALL')) return recallAsset;
  return undefined;
}

function completeMao(value: string): string | undefined {
  const text = value.trim();
  const candidate = text.replace(/\.$/, '');
  if (!candidate || [...candidate].length > 128 || /^\d+$/.test(candidate) || /[\sA-Z]/.test(candidate)) return undefined;
  if (text.endsWith('.mao')) return text;
  if (text.endsWith('.ma')) return `${text}o`;
  if (text.endsWith('.m')) return `${text}ao`;
  if (text.endsWith('.')) return `${text}mao`;
  return `${text}.mao`;
}

function ConversationSearchResults({ query, category, conversations, messageConversations, currentUserId, onOpenUrl, onConversationSelected, onMessageSelected }: { query: string; category: string; conversations: Conversation[]; messageConversations: Conversation[]; currentUserId: string; onOpenUrl?: (url: string) => void; onConversationSelected: (conversationId?: string) => void; onMessageSelected: (message: Message) => void }) {
  const queryClient = useQueryClient();
  const [debouncedQuery, setDebouncedQuery] = useState(query.trim());
  const [expanded, setExpanded] = useState<'contacts' | 'conversations' | 'messages'>();
  useEffect(() => {
    const timeout = window.setTimeout(() => setDebouncedQuery(query.trim()), 150);
    return () => window.clearTimeout(timeout);
  }, [query]);
  useEffect(() => setExpanded(undefined), [query]);
  const enabled = debouncedQuery.length > 0;
  const users = useQuery({ queryKey: ['search-local-users', debouncedQuery, category], queryFn: () => desktopApi.searchLocalUsers(debouncedQuery, category), enabled });
  const messages = useQuery({ queryKey: ['search-global-messages', debouncedQuery], queryFn: () => desktopApi.searchGlobalMessages(debouncedQuery), enabled });
  const mao = completeMao(debouncedQuery);
  const maoUser = useQuery({ queryKey: ['search-mao-user', debouncedQuery], queryFn: () => desktopApi.searchMaoUser(debouncedQuery), enabled: Boolean(mao) });
  const loading = users.isFetching || messages.isFetching || maoUser.isFetching;
  async function openUser(user: UserProfile) {
    const conversationId = await desktopApi.openUserConversation(user.userId);
    onConversationSelected(conversationId);
    await queryClient.invalidateQueries({ queryKey: ['bootstrap'] });
  }
  const isMixinNumber = /^\+?\d+$/.test(debouncedQuery);
  const isUrl = /^(https?:\/\/|mixin:\/\/)/i.test(debouncedQuery);
  return <div className={classes("conversation-search-results")}>
    {maoUser.data && <SearchUserRow user={maoUser.data} description={mao} onClick={() => void openUser(maoUser.data!)} />}
    {isMixinNumber && <button type="button" className={classes("search-action-row")} onClick={() => void desktopApi.searchUser(debouncedQuery).then(openUser)}><Search size={18} /><span>Search user {debouncedQuery}</span></button>}
    {isUrl && <button type="button" className={classes("search-action-row")} onClick={() => onOpenUrl ? onOpenUrl(debouncedQuery) : window.open(debouncedQuery, '_blank', 'noopener,noreferrer')}><Search size={18} /><span>Open link {debouncedQuery}</span></button>}
    {(users.data?.length ?? 0) > 0 && <SearchSection title="Contacts" expanded={expanded === 'contacts'} showMore={users.data!.length > 3} onToggle={() => setExpanded(expanded === 'contacts' ? undefined : 'contacts')}>{users.data!.slice(0, expanded === 'contacts' ? undefined : 3).map((user) => <SearchUserRow key={user.userId} user={user} onClick={() => void openUser(user)} />)}</SearchSection>}
    {conversations.length > 0 && <SearchSection title="Conversations" expanded={expanded === 'conversations'} showMore={conversations.length > 3} onToggle={() => setExpanded(expanded === 'conversations' ? undefined : 'conversations')}>{conversations.slice(0, expanded === 'conversations' ? undefined : 3).map((conversation) => <button key={conversation.conversationId} type="button" className={classes("search-conversation-row")} onClick={() => onConversationSelected(conversation.conversationId)}><Avatar name={conversation.name} url={conversation.avatarUrl} size={50} /><span><strong>{conversation.name}</strong><small>{conversationPreview(conversation, currentUserId)}</small></span></button>)}</SearchSection>}
    {(messages.data?.length ?? 0) > 0 && <SearchSection title="Messages" expanded={expanded === 'messages'} showMore={messages.data!.length > 3} onToggle={() => setExpanded(expanded === 'messages' ? undefined : 'messages')}>{messages.data!.slice(0, expanded === 'messages' ? undefined : 3).map((message) => {
      const conversation = messageConversations.find((item) => item.conversationId === message.conversationId);
      const name = conversation?.name ?? message.senderName;
      const avatarUrl = conversation?.avatarUrl ?? message.senderAvatarUrl;
      return <button key={message.messageId} type="button" className={classes("search-conversation-row")} onClick={() => onMessageSelected(message)}><Avatar name={name} url={avatarUrl} size={50} /><span><strong>{name}</strong><small>{messageSearchPreview(message)}</small></span><time>{formatTime(Math.floor(message.createdAtMicros / 1_000))}</time></button>;
    })}</SearchSection>}
    {enabled && !loading && !maoUser.data && !(users.data?.length) && conversations.length === 0 && !(messages.data?.length) && !isMixinNumber && !isUrl && <div className={classes("empty-list")}>No results</div>}
    {loading && <div className={classes("search-loading")}>Loading…</div>}
  </div>;
}

function messageSearchPreview(message: Message): string {
  if (message.category.includes('TEXT') || message.category.includes('POST')) return message.content;
  return message.caption?.trim() || message.mediaName || message.content;
}

function SearchSection({ title, children, expanded, showMore, onToggle }: { title: string; children: React.ReactNode; expanded: boolean; showMore: boolean; onToggle: () => void }) {
  return <section className={classes("conversation-search-section")}><h2>{title}{showMore && <button type="button" onClick={onToggle}>{expanded ? 'Less' : 'More'}</button>}</h2>{children}</section>;
}

function SearchUserRow({ user, description, onClick }: { user: UserProfile; description?: string; onClick: () => void }) {
  return <button type="button" className={classes("search-conversation-row")} onClick={onClick}><Avatar name={user.fullName} url={user.avatarUrl} size={50} /><span><strong>{user.fullName}<Badges verified={user.isVerified} isBot={user.isBot} membership={user.membership} /></strong><small>{description ?? user.identityNumber}</small></span></button>;
}

function AudioPlayerBar({ conversations, selectedConversationId, onSelect }: { conversations: Conversation[]; selectedConversationId?: string; onSelect: (conversationId?: string) => void }) {
  const message = useAudioPlayback((state) => state.message);
  const isPlaying = useAudioPlayback((state) => state.isPlaying);
  const progress = useAudioPlayback((state) => state.progress);
  const speed = useAudioPlayback((state) => state.speed);
  const pause = useAudioPlayback((state) => state.pause);
  const resume = useAudioPlayback((state) => state.resume);
  const stop = useAudioPlayback((state) => state.stop);
  const toggleSpeed = useAudioPlayback((state) => state.toggleSpeed);
  const conversation = conversations.find((item) => item.conversationId === message?.conversationId);
  if (!message || message.conversationId === selectedConversationId) return null;
  return <button type="button" className={classes("audio-player-bar")} onClick={() => onSelect(message.conversationId)}><span className={classes(speed === 2 ? 'is-active' : '')} onClick={(event) => { event.stopPropagation(); toggleSpeed(); }}>2X</span><i onClick={(event) => { event.stopPropagation(); if (isPlaying) pause(); else void resume(); }}>{isPlaying ? <Pause size={18} /> : <Play size={18} />}</i><strong><Avatar name={conversation?.name ?? ''} url={conversation?.avatarUrl} size={32} />{conversation?.name}</strong><i onClick={(event) => { event.stopPropagation(); stop(); }}><X size={18} /></i><em style={{ '--audio-bar-progress': progress } as React.CSSProperties} /></button>;
}

function NetworkStatus({ connected, connectedBefore }: { connected: boolean; connectedBefore: boolean }) {
  if (connected) return null;
  return <div className={classes("network-status")}>{connectedBefore && <div className={classes("network-warning")}><span><AlertTriangle size={11} fill="currentColor" /></span><p>Network connection failed</p><button type="button" onClick={() => void desktopApi.retryConnection()}>Retry</button></div>}<i /></div>;
}

function ConversationMessageStatus({ conversation, currentUserId }: { conversation: Conversation; currentUserId: string }) {
  const category = conversation.lastMessageCategory || '';
  if ((conversation.draft && conversation.status !== 3) || conversation.lastMessageSenderId !== currentUserId || !['TEXT', 'IMAGE', 'VIDEO', 'LIVE', 'DATA', 'POST', 'LOCATION', 'AUDIO', 'STICKER', 'CONTACT', 'TRANSCRIPT'].some((kind) => category.endsWith(`_${kind}`))) return null;
  const status = conversation.lastMessageStatus?.toUpperCase();
  const asset = status === 'SENT' ? sentAsset : status === 'DELIVERED' ? deliveredAsset : status === 'READ' ? readAsset : undefined;
  return asset ? <span className={classes(`conversation-message-status ${status === 'READ' ? 'is-read' : ''}`)} aria-hidden="true" style={{ '--conversation-status-image': `url("${asset}")` } as React.CSSProperties} /> : null;
}

function ConversationIndicators({ conversation }: { conversation: Conversation }) {
  return <span className={classes("conversation-indicators")}>
    {conversation.mentionCount > 0 && <span className={classes("unseen-badge")}>@</span>}
    {conversation.unseenCount > 0 ? <span className={classes(`unseen-badge ${conversation.isMuted ? 'is-muted' : ''}`)}>{conversation.unseenCount}</span> : <>{conversation.isMuted && <ConversationIndicatorIcon src={muteAsset} />}{conversation.isPinned && <ConversationIndicatorIcon src={pinAsset} />}</>}
  </span>;
}

function ConversationIndicatorIcon({ src }: { src: string }) {
  return <span className={classes("conversation-indicator-icon")} aria-hidden="true" style={{ '--conversation-indicator-image': `url("${src}")` } as React.CSSProperties} />;
}

function ConversationContextMenu({ conversation, circles, currentCircleId, onMute, onDelete, onChanged, children }: { conversation: Conversation; circles: Circle[]; currentCircleId?: string; onMute: () => void; onDelete: () => void; onChanged: () => void; children: React.ReactElement<React.HTMLAttributes<HTMLElement>> }) {
  async function setPinned() { await desktopApi.setConversationPinned(conversation.conversationId, !conversation.isPinned); onChanged(); }
  async function changeCircle(circleId: string, add: boolean) { await desktopApi.editCircleConversation(circleId, conversation.conversationId, conversation.ownerId, conversation.category === 'GROUP', add); onChanged(); }
  const availableCircles = circles.filter((circle) => !conversation.circleIds.includes(circle.circleId));
  const entries = [
    { type: 'item' as const, id: 'toggle-pin-conversation', text: conversation.isPinned ? 'Unpin' : 'Pin', icon: flutterMenuIcon(conversation.isPinned ? 'unPin' : 'pin'), action: setPinned },
    { type: 'item' as const, id: 'toggle-mute-conversation', text: conversation.isMuted ? 'Unmute' : 'Mute', icon: flutterMenuIcon(conversation.isMuted ? 'unMute' : 'mute'), action: conversation.isMuted ? () => desktopApi.setConversationMuted(conversation.conversationId, conversation.ownerId, conversation.category, 0).then(onChanged) : onMute },
    { type: 'separator' as const },
    ...(availableCircles.length > 0 ? [
      { type: 'submenu' as const, id: 'add-conversation-to-circle', text: 'Add to Circle', items: availableCircles.map((circle) => ({ type: 'item' as const, id: `add-to-circle-${circle.circleId}`, text: circle.name, icon: flutterMenuIcon('circle'), action: () => changeCircle(circle.circleId, true) })) },
      { type: 'separator' as const },
    ] : []),
    { type: 'item' as const, id: 'delete-conversation', text: 'Delete Chat', icon: flutterMenuIcon('delete'), action: onDelete },
    ...(currentCircleId && conversation.circleIds.includes(currentCircleId) ? [{ type: 'item' as const, id: 'remove-from-circle', text: 'Remove from Circle', icon: flutterMenuIcon('circle'), action: () => changeCircle(currentCircleId, false) }] : []),
  ];
  return cloneElement(children, { onContextMenu: (event: React.MouseEvent<HTMLElement>) => void showNativeContextMenu(event, entries) });
}

function MuteDialog({ conversation, onClose, onMuted }: { conversation: Conversation; onClose: () => void; onMuted: () => void }) {
  const [duration, setDuration] = useState<number>();
  const [saving, setSaving] = useState(false);
  const options = [['1 hour', 3600], ['8 hours', 28800], ['1 week', 604800], ['1 year', 31536000]] as const;
  async function confirm() { if (duration == null || saving) return; setSaving(true); try { await desktopApi.setConversationMuted(conversation.conversationId, conversation.ownerId, conversation.category, duration); onMuted(); onClose(); } finally { setSaving(false); } }
  return <MixinDialog open onClose={onClose} maxWidth={400}><AlertDialogLayout title="Mute Notifications" minHeight={210} content={<div className={classes("mixin-dialog-options")}>{options.map(([label, value]) => <button type="button" key={value} className={classes(duration === value ? 'is-selected' : '')} onClick={() => setDuration(value)}><span>{duration === value && <Check size={10} />}</span>{label}</button>)}</div>} actions={<><MixinButton transparent onClick={onClose}>Cancel</MixinButton><MixinButton disabled={duration == null || saving} onClick={() => void confirm()}>Confirm</MixinButton></>} /></MixinDialog>;
}

function DeleteConversationDialog({ conversation, onClose, onDeleted }: { conversation: Conversation; onClose: () => void; onDeleted: () => void }) {
  const [deleting, setDeleting] = useState(false);
  async function confirm() { if (deleting) return; setDeleting(true); try { await desktopApi.deleteConversation(conversation.conversationId); onDeleted(); onClose(); } finally { setDeleting(false); } }
  return <MixinDialog open onClose={onClose} maxWidth={400}><AlertDialogLayout title="Delete Chat" content={<>Delete the chat with “{conversation.name}”?<p className={classes("mixin-alert-description")}>This only removes it from this device.</p></>} actions={<><MixinButton transparent disabled={deleting} onClick={onClose}>Cancel</MixinButton><MixinButton className={classes("is-danger")} disabled={deleting} onClick={() => void confirm()}>Delete</MixinButton></>} /></MixinDialog>;
}
