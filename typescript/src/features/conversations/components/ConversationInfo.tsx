import { useEffect, useRef, useState, type FormEvent } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { openBotWebview } from '@/shared/platform/botWebview';
import { useDesktopUi } from '@/state/desktopUi';
import type {
  Circle,
  Conversation,
  ConversationDetail,
  ConversationParticipant,
  GroupConversation,
  Message,
  SharedApp,
  UserProfile,
} from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { AlertDialogLayout, MixinButton, MixinDialog } from '@/shared/ui/MixinDialog';
import { UserProfileDialog } from '@/features/profile/components/UserProfileDialog';
import { classes } from '@/shared/styles/classes';
import { flutterMenuIcon } from '@/shared/ui/flutterMenuIcon';
import { showNativeContextMenu } from '@/shared/ui/nativeMenu';

type ConversationInfoDestination = 'info' | 'participants' | 'media' | 'apps' | 'groups' | 'circles' | 'disappearing';

export function ConversationInfo({ conversation, currentUserId }: { conversation: Conversation; currentUserId: string }) {
  const queryClient = useQueryClient();
  const closeInfo = useDesktopUi((state) => state.closeInfo);
  const selectConversation = useDesktopUi((state) => state.selectConversation);
  const focusMessage = useDesktopUi((state) => state.focusMessage);
  const openMessageSearch = useDesktopUi((state) => state.openMessageSearch);
  const [destinationStack, setDestinationStack] = useState<ConversationInfoDestination[]>(['info']);
  const destination = destinationStack[destinationStack.length - 1] ?? 'info';
  const [participants, setParticipants] = useState<ConversationParticipant[]>([]);
  const [shared, setShared] = useState<Message[]>([]);
  const [sharedHasMore, setSharedHasMore] = useState(true);
  const [sharedLoadingMore, setSharedLoadingMore] = useState(false);
  const sharedRequest = useRef(0);
  const [apps, setApps] = useState<SharedApp[]>([]);
  const [commonGroups, setCommonGroups] = useState<GroupConversation[]>([]);
  const [circles, setCircles] = useState<Circle[]>([]);
  const [selectedCircleIds, setSelectedCircleIds] = useState<Set<string>>(new Set());
  const [newCircleOpen, setNewCircleOpen] = useState(false);
  const [newCircleName, setNewCircleName] = useState('');
  const [detail, setDetail] = useState<ConversationDetail>();
  const [search, setSearch] = useState('');
  const [mediaKind, setMediaKind] = useState<'media' | 'post' | 'file'>('media');
  const [addingParticipants, setAddingParticipants] = useState(false);
  const [participantActionsOpen, setParticipantActionsOpen] = useState(false);
  const [inviteOpen, setInviteOpen] = useState(false);
  const [candidateUsers, setCandidateUsers] = useState<UserProfile[]>([]);
  const [selectedCandidateUserIds, setSelectedCandidateUserIds] = useState(new Set<string>());
  const [participantsLoaded, setParticipantsLoaded] = useState(false);
  const [customDurationOpen, setCustomDurationOpen] = useState(false);
  const [customDuration, setCustomDuration] = useState('');
  const [customUnit, setCustomUnit] = useState<'second' | 'minute' | 'hour' | 'day' | 'week'>('second');
  const [customDurationError, setCustomDurationError] = useState('');
  const [muteOpen, setMuteOpen] = useState(false);
  const [editingConversation, setEditingConversation] = useState<{ title: string; value: string; announcement?: boolean }>();
  const [profileDialog, setProfileDialog] = useState<UserProfile>();
  const [shareContactOpen, setShareContactOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<{ title: string; description: string; confirmLabel: string; action: () => Promise<void> }>();
  const isGroup = conversation.category === 'GROUP';

  useEffect(() => {
    setDestinationStack(['info']); setParticipants([]); setParticipantsLoaded(false); setShared([]); setSharedHasMore(true); setSharedLoadingMore(false); setApps([]); setCommonGroups([]); setCircles([]); setSelectedCircleIds(new Set(conversation.circleIds)); setNewCircleOpen(false); setNewCircleName(''); setDetail(undefined);
    void desktopApi.conversationDetail(conversation.conversationId).then(setDetail).catch((reason) => e('Unable to load conversation details', reason));
    if (!isGroup) void desktopApi.sharedApps(conversation.ownerId).then(setApps).catch((reason) => e('Unable to load shared apps', reason));
    if (isGroup) void desktopApi.conversationParticipants(conversation.conversationId).then((values) => { setParticipants(values); setParticipantsLoaded(true); }).catch((reason) => e('Unable to load group participants', reason));
  }, [conversation.circleIds, conversation.conversationId, conversation.ownerId, isGroup]);

  async function openDestination(next: ConversationInfoDestination) {
    setDestinationStack((current) => {
      const index = current.indexOf(next);
      if (index === current.length - 1) return current;
      return [...(index === -1 ? current : current.slice(0, index)), next];
    });
    if (next === 'participants') { setParticipants(await desktopApi.conversationParticipants(conversation.conversationId)); setParticipantsLoaded(true); }
    if (next === 'apps') setApps(await desktopApi.sharedApps(conversation.ownerId));
    if (next === 'groups') setCommonGroups(await desktopApi.groupsInCommon(conversation.ownerId));
    if (next === 'circles') {
      setSelectedCircleIds(new Set(conversation.circleIds));
      setCircles(await desktopApi.circles());
    }
    if (next === 'media') await loadShared(mediaKind, true);
  }

  function popDestination() {
    setDestinationStack((current) => current.length > 1 ? current.slice(0, -1) : current);
  }

  async function changeMediaKind(kind: typeof mediaKind) {
    setMediaKind(kind);
    await loadShared(kind, true);
  }

  async function loadShared(kind: typeof mediaKind, reset = false) {
    if (sharedLoadingMore && !reset) return;
    const offset = reset ? 0 : shared.length;
    if (!reset && !sharedHasMore) return;
    setSharedLoadingMore(true);
    const request = ++sharedRequest.current;
    try {
      const page = await desktopApi.sharedMessages(conversation.conversationId, kind, offset, 100);
      if (request !== sharedRequest.current) return;
      setShared((current) => reset ? page : [...current, ...page]);
      setSharedHasMore(page.length === 100);
    } finally {
      if (request === sharedRequest.current) setSharedLoadingMore(false);
    }
  }

  const title = ({ info: 'Conversation Info', participants: 'Group Participants', media: 'Shared Media', apps: 'Shared Apps', groups: 'Groups in Common', circles: 'Circles', disappearing: 'Disappearing Messages' })[destination];
  const filteredParticipants = participants.filter((item) => !search || `${item.fullName} ${item.identityNumber}`.toLowerCase().includes(search.toLowerCase()));
  const currentParticipant = participants.find((item) => item.userId === currentUserId);
  const canManage = currentParticipant?.role === 'OWNER' || currentParticipant?.role === 'ADMIN';
  const isExited = isGroup && participantsLoaded && !currentParticipant;
  const canModifyDisappearing = !isGroup || canManage;
  const muted = (detail?.muteUntilMillis ?? 0) > Date.now();

  async function refreshParticipants() {
    setParticipants(await desktopApi.conversationParticipants(conversation.conversationId));
  }

  async function addParticipants(userIds: string[]) {
    if (userIds.length === 0) return;
    await desktopApi.updateParticipants(conversation.conversationId, 'ADD', userIds);
    setAddingParticipants(false); await refreshParticipants();
  }

  async function updateParticipant(action: string, userId: string, role?: string) {
    await desktopApi.updateParticipants(conversation.conversationId, action, [userId], role);
    await refreshParticipants();
  }
  async function openParticipant(userId: string) {
    const profile = await desktopApi.userProfile(userId);
    if (profile) setProfileDialog(profile);
  }
  async function setDisappearing(seconds: number) {
    const previous = detail?.expireIn;
    setDetail((current) => current ? { ...current, expireIn: seconds } : current);
    try {
      await desktopApi.setDisappearingMessages(conversation.conversationId, seconds);
    } catch {
      setDetail((current) => current && previous !== undefined ? { ...current, expireIn: previous } : current);
    }
  }
  function submitCustomDuration(event: FormEvent) {
    event.preventDefault();
    const value = Number(customDuration);
    const unitSeconds = { second: 1, minute: 60, hour: 3600, day: 86400, week: 604800 }[customUnit];
    const maximum = { second: 59, minute: 59, hour: 23, day: 6, week: 4 }[customUnit];
    if (!Number.isInteger(value) || value <= 0 || value > maximum) {
      setCustomDurationError(`Enter a value from 1 to ${maximum}.`);
      return;
    }
    setCustomDurationOpen(false); setCustomDuration(''); setCustomDurationError('');
    void setDisappearing(value * unitSeconds);
  }
  async function refreshConversations() {
    await queryClient.invalidateQueries({ queryKey: ['bootstrap'] });
  }
  async function saveConversationEdit(value: string, announcement = false) {
    const name = value.trim();
    if (!announcement && !name) return;
    if (announcement) await desktopApi.editConversation(conversation.conversationId, undefined, value.trim());
    else if (isGroup) await desktopApi.editConversation(conversation.conversationId, name);
    else await desktopApi.addContact(conversation.ownerId, name);
    setEditingConversation(undefined);
    setDetail(await desktopApi.conversationDetail(conversation.conversationId));
    await refreshConversations();
  }
  function confirm(title: string, description: string, action: () => Promise<void>) {
    setConfirmation({ title, description, confirmLabel: title, action });
  }
  return (
    <aside className={classes("conversation-info")}>
      <header>{destination !== 'info' && <button type="button" onClick={popDestination}>‹</button>}<strong>{title}</strong>{destination === 'circles' ? <button type="button" aria-label="Create circle" onClick={() => setNewCircleOpen(true)}>+</button> : <button type="button" onClick={closeInfo}>×</button>}</header>
      {destination === 'info' && <><div className={classes("conversation-info-profile")}><Avatar name={conversation.name} url={conversation.avatarUrl} size={90} /><h2>{detail?.name || conversation.name}</h2><small>{isGroup ? `${conversation.participantCount} participants` : `Mixin ID: ${conversation.identityNumber}`}</small>{!isGroup && conversation.relationship === 'STRANGER' && <button className={classes("add-contact")} type="button" onClick={() => void desktopApi.addContact(conversation.ownerId, conversation.name).then(refreshConversations)}>+ {conversation.isBot ? 'Add Bot' : 'Add Contact'}</button>}{isGroup && detail?.announcement && <p className={classes("conversation-description")}>{detail.announcement}</p>}</div>{isGroup && !isExited && <div className={classes("info-cells")}><button type="button" onClick={() => void openDestination('participants')}>Group Participants <span>›</span></button></div>}{!isGroup && <div className={classes("info-cells")}><button type="button" onClick={() => setShareContactOpen(true)}>Share Contact</button></div>}<div className={classes("info-cells")}><button type="button" onClick={() => void openDestination('media')}>Shared Media <span>›</span></button>{!isGroup && apps.length > 0 && <button type="button" onClick={() => void openDestination('apps')}>Shared Apps <span>›</span></button>}<button type="button" onClick={openMessageSearch}>Search Conversation <span>›</span></button></div>{!isExited && <div className={classes("info-cells")}><button type="button" disabled={!canModifyDisappearing} onClick={() => canModifyDisappearing && void openDestination('disappearing')}>Disappearing Messages <span>{durationLabel(detail?.expireIn ?? 0)}{canModifyDisappearing && ' ›'}</span></button></div>}{isGroup && canManage && <div className={classes("info-cells")}><button type="button" onClick={() => setEditingConversation({ title: detail?.announcement ? 'Edit Group Description' : 'Add Group Description', value: detail?.announcement ?? '', announcement: true })}>{detail?.announcement ? 'Edit Group Description' : 'Add Group Description'}</button></div>}<div className={classes("info-cells")}><button type="button" onClick={() => muted ? void desktopApi.setConversationMuted(conversation.conversationId, conversation.ownerId, conversation.category, 0).then(async () => { setDetail(await desktopApi.conversationDetail(conversation.conversationId)); await refreshConversations(); }) : setMuteOpen(true)}>{muted ? 'Unmute' : 'Mute'}{muted && <span>{new Intl.DateTimeFormat(undefined, { dateStyle: 'short', timeStyle: 'short' }).format(new Date(detail!.muteUntilMillis))}</span>}</button>{(!isGroup || canManage) && <button type="button" onClick={() => setEditingConversation({ title: 'Edit Name', value: detail?.name || conversation.name })}>Edit Name</button>}</div>{!isGroup && <div className={classes("info-cells")}><button type="button" onClick={() => void openDestination('groups')}>Groups in Common <span>›</span></button></div>}<div className={classes("info-cells")}><button type="button" onClick={() => void openDestination('circles')}>Edit Conversations <span>›</span></button></div><div className={classes("info-cells info-actions")}>{!isGroup && conversation.relationship === 'BLOCKED' && <button className={classes("is-danger")} type="button" onClick={() => confirm('Unblock', `Unblock ${conversation.name}?`, async () => { await desktopApi.unblockUser(conversation.ownerId); await refreshConversations(); })}>Unblock</button>}{!isGroup && conversation.relationship !== 'STRANGER' && conversation.relationship !== 'BLOCKED' && <button className={classes("is-danger")} type="button" onClick={() => confirm(conversation.isBot ? 'Remove Bot' : 'Remove Contact', `Remove ${conversation.name}?`, async () => { await desktopApi.removeContact(conversation.ownerId); await refreshConversations(); })}>{conversation.isBot ? 'Remove Bot' : 'Remove Contact'}</button>}{!isGroup && conversation.relationship === 'STRANGER' && <button className={classes("is-danger")} type="button" onClick={() => confirm('Block', `Block ${conversation.name}?`, async () => { await desktopApi.blockUser(conversation.ownerId); await refreshConversations(); })}>Block</button>}<button className={classes("is-danger")} type="button" onClick={() => confirm('Clear Chat', `Clear all messages in ${conversation.name}?`, async () => { await desktopApi.clearConversation(conversation.conversationId); await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] }); })}>Clear Chat</button>{isGroup && <button className={classes("is-danger")} type="button" onClick={() => confirm(isExited ? 'Delete Group' : 'Exit Group', `${isExited ? 'Delete' : 'Exit'} ${conversation.name}?`, async () => { if (isExited) await desktopApi.deleteConversation(conversation.conversationId); else await desktopApi.exitGroup(conversation.conversationId); closeInfo(); await refreshConversations(); })}>{isExited ? 'Delete Group' : 'Exit Group'}</button>}</div>{!isGroup && <div className={classes("info-cells info-actions")}><button className={classes("is-danger")} type="button" onClick={() => confirm('Report and Block', `Report and block ${conversation.name}?`, async () => { await desktopApi.reportUser(conversation.ownerId); await refreshConversations(); })}>Report</button></div>}{isGroup && detail && <p className={classes("conversation-created-at")}>Created {new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(new Date(detail.createdAtMillis))}</p>}</>}
      {destination === 'groups' && <div className={classes("side-list common-groups")}>{commonGroups.length === 0 ? <p className={classes("empty-list")}>No results</p> : commonGroups.map((group) => <button type="button" key={group.conversationId} onClick={() => void desktopApi.conversationsByIds([group.conversationId]).then(([item]) => { if (item) { selectConversation(item.conversationId); closeInfo(); } })}><Avatar name={group.name} url={group.avatarUrl} size={50} /><span><strong>{group.name}</strong><small>{group.participantCount} participants</small></span></button>)}</div>}
      {destination === 'circles' && <div className={classes("circle-manager")}>{[...circles].sort((left, right) => Number(selectedCircleIds.has(right.circleId)) - Number(selectedCircleIds.has(left.circleId))).map((circle, index, values) => { const selected = selectedCircleIds.has(circle.circleId); const hasSelected = values.some((item) => selectedCircleIds.has(item.circleId)); const previousSelected = index > 0 && selectedCircleIds.has(values[index - 1].circleId); return <div className={classes(!selected && hasSelected && previousSelected ? 'circle-manager-gap' : undefined)} key={circle.circleId}><button type="button" onClick={() => { const add = !selected; setSelectedCircleIds((current) => { const next = new Set(current); if (add) next.add(circle.circleId); else next.delete(circle.circleId); return next; }); void desktopApi.editCircleConversation(circle.circleId, conversation.conversationId, conversation.ownerId, isGroup, add).then(refreshConversations).catch((reason) => { e('Unable to update circle membership', reason); setSelectedCircleIds((current) => { const next = new Set(current); if (add) next.delete(circle.circleId); else next.add(circle.circleId); return next; }); }); }}><span className={classes("circle-toggle")}>{selected ? <CircleMembershipIcon selected /> : <CircleMembershipIcon />}</span><CircleManagerIcon color={circleColor(circle.circleId)} /><span><strong>{circle.name}</strong><small>{circle.conversationCount} conversations</small></span></button></div>; })}</div>}
      {newCircleOpen && <div className={classes("modal-backdrop")}><form className={classes("circle-dialog")} onSubmit={(event) => { event.preventDefault(); const name = newCircleName.trim(); if (!name) return; void desktopApi.createCircle(name).then(async (circle) => { await desktopApi.editCircleConversation(circle.circleId, conversation.conversationId, conversation.ownerId, isGroup, true); setSelectedCircleIds((current) => new Set(current).add(circle.circleId)); setCircles(await desktopApi.circles()); setNewCircleOpen(false); setNewCircleName(''); await refreshConversations(); }); }}><header><strong>Circles</strong></header><input autoFocus maxLength={64} value={newCircleName} placeholder="Circle name" onChange={(event) => setNewCircleName(event.target.value)} /><footer><button type="button" onClick={() => { setNewCircleOpen(false); setNewCircleName(''); }}>Cancel</button><button type="submit" disabled={!newCircleName.trim()}>Create</button></footer></form></div>}
      {muteOpen && <MuteConversationDialog conversation={conversation} onClose={() => setMuteOpen(false)} onMuted={async () => { setDetail(await desktopApi.conversationDetail(conversation.conversationId)); await refreshConversations(); }} />}
      {editingConversation && <ConversationEditDialog {...editingConversation} onClose={() => setEditingConversation(undefined)} onSave={(value) => saveConversationEdit(value, editingConversation.announcement)} />}
      {destination === 'participants' && <div className={classes("side-list participants-list")}><div className={classes("participant-search")}><input autoFocus placeholder="Search" value={search} onChange={(event) => setSearch(event.target.value)} />{canManage && <button type="button" onClick={() => setParticipantActionsOpen((open) => !open)}>+</button>}</div>{participantActionsOpen && <div className={classes("participant-actions")}><button type="button" onClick={() => { setParticipantActionsOpen(false); void desktopApi.selectableUsers().then((users) => { setCandidateUsers(users.filter((user) => !participants.some((item) => item.userId === user.userId))); setSelectedCandidateUserIds(new Set()); setAddingParticipants(true); }); }}>Add Participants</button><button type="button" onClick={() => { setParticipantActionsOpen(false); setInviteOpen(true); }}>Invite to Group via Link</button></div>}{filteredParticipants.map((participant) => <GroupParticipantRow key={participant.userId} participant={participant} currentParticipant={currentParticipant} currentUserId={currentUserId} onOpen={openParticipant} onUpdate={updateParticipant} />)}{addingParticipants && <div className={classes("participant-candidates")}><header><strong>Add Participants</strong><button type="button" onClick={() => setAddingParticipants(false)}>×</button></header><div>{candidateUsers.map((user) => { const selected = selectedCandidateUserIds.has(user.userId); return <button type="button" className={classes(selected ? 'is-selected' : '')} key={user.userId} onClick={() => setSelectedCandidateUserIds((current) => { const next = new Set(current); if (selected) next.delete(user.userId); else next.add(user.userId); return next; })}><Avatar name={user.fullName} url={user.avatarUrl} size={50} /><span><strong>{user.fullName}</strong><small>{user.identityNumber}</small></span><i>{selected ? '✓' : ''}</i></button>; })}</div><footer><button type="button" onClick={() => setAddingParticipants(false)}>Cancel</button><button type="button" className={classes("primary-button")} disabled={selectedCandidateUserIds.size === 0} onClick={() => void addParticipants([...selectedCandidateUserIds])}>Add{selectedCandidateUserIds.size ? ` (${selectedCandidateUserIds.size})` : ''}</button></footer></div>}</div>}
      {destination === 'media' && <div className={classes("shared-media")}><div className={classes(`shared-${mediaKind}`)} onScroll={(event) => { const element = event.currentTarget; if (element.scrollHeight - element.scrollTop - element.clientHeight < 240) void loadShared(mediaKind); }}>{shared.map((message, index) => <SharedMessage key={message.messageId} message={message} showDate={!shared[index - 1] || !sameSharedDay(shared[index - 1].createdAtMicros, message.createdAtMicros)} onLocate={() => { focusMessage(message.messageId); closeInfo(); }} />)}{sharedLoadingMore && <p className={classes("shared-loading")}>Loading…</p>}{shared.length === 0 && !sharedLoadingMore && <p className={classes("empty-list")}>No shared {mediaKind}</p>}</div><div className={classes("side-tabs")}>{(['media', 'post', 'file'] as const).map((kind) => <button className={classes(mediaKind === kind ? 'is-selected' : '')} type="button" key={kind} onClick={() => void changeMediaKind(kind)}>{kind[0].toUpperCase() + kind.slice(1)}</button>)}</div></div>}
      {destination === 'apps' && <div className={classes("side-list")}>{apps.map((app) => <button type="button" onClick={() => void openBotWebview({ url: app.homeUri, title: app.name, conversationId: conversation.conversationId })} key={app.appId}><Avatar name={app.name} url={app.iconUrl} size={46} /><span><strong>{app.name}</strong><small>{app.description}</small></span></button>)}</div>}
      {destination === 'disappearing' && <div className={classes("disappearing-panel")}><p>New messages disappear after the selected duration.</p><div className={classes("info-cells")}>{[0, 30, 600, 7200, 86400, 604800].map((seconds) => <button type="button" key={seconds} disabled={detail?.expireIn === seconds} onClick={() => void setDisappearing(seconds)}>{durationLabel(seconds)}<span>{detail?.expireIn === seconds ? '✓' : ''}</span></button>)}<button type="button" onClick={() => setCustomDurationOpen(true)}>Custom Time <span>›</span></button></div></div>}
      {customDurationOpen && <div className={classes("modal-backdrop")}><form className={classes("custom-disappearing-dialog")} onSubmit={submitCustomDuration}><header><strong>Custom Time</strong><button type="button" onClick={() => setCustomDurationOpen(false)}>×</button></header><div><input autoFocus inputMode="numeric" maxLength={2} value={customDuration} onChange={(event) => { setCustomDuration(event.target.value.replace(/\D/g, '')); setCustomDurationError(''); }} /><select value={customUnit} onChange={(event) => { setCustomUnit(event.target.value as typeof customUnit); setCustomDurationError(''); }}><option value="second">Second</option><option value="minute">Minute</option><option value="hour">Hour</option><option value="day">Day</option><option value="week">Week</option></select></div>{customDurationError && <p className={classes("dialog-error")}>{customDurationError}</p>}<footer><button type="submit" className={classes("primary-button")}>Set</button></footer></form></div>}
      {confirmation && <SideActionConfirmation {...confirmation} onClose={() => setConfirmation(undefined)} />}
      {profileDialog && <UserProfileDialog profile={profileDialog} currentUserId={currentUserId} onClose={() => setProfileDialog(undefined)} onChanged={async () => { await refreshConversations(); setProfileDialog(await desktopApi.userProfile(profileDialog.userId) ?? undefined); }} onChat={async () => { const conversationId = await desktopApi.openUserConversation(profileDialog.userId); await refreshConversations(); selectConversation(conversationId); setProfileDialog(undefined); closeInfo(); }} onInfo={async () => { const conversationId = await desktopApi.openUserConversation(profileDialog.userId); await refreshConversations(); selectConversation(conversationId); useDesktopUi.getState().toggleInfo(); setProfileDialog(undefined); }} />}
      {shareContactOpen && <ContactShareDialog userId={conversation.ownerId} onClose={() => setShareContactOpen(false)} />}
      {inviteOpen && <GroupInviteDialog conversation={conversation} onClose={() => setInviteOpen(false)} />}
    </aside>
  );
}

function GroupParticipantRow({ participant, currentParticipant, currentUserId, onOpen, onUpdate }: { participant: ConversationParticipant; currentParticipant?: ConversationParticipant; currentUserId: string; onOpen: (userId: string) => Promise<void>; onUpdate: (action: string, userId: string, role?: string) => Promise<void> }) {
  const canOpenMenu = participant.userId !== currentUserId && Boolean(currentParticipant);
  const owner = currentParticipant?.role === 'OWNER';
  const canRemove = currentParticipant != null && ((currentParticipant.role != null && participant.role == null) || owner);
  const item = <article onClick={() => void onOpen(participant.userId)} onContextMenu={canOpenMenu ? (event) => void showNativeContextMenu(event, [
    { type: 'item', id: 'message-participant', text: `Message ${participant.fullName}`, icon: flutterMenuIcon('chat'), action: () => onOpen(participant.userId) },
    ...(owner ? [
      { type: 'separator' as const },
      { type: 'item' as const, id: 'change-participant-role', text: participant.role === 'ADMIN' ? 'Dismiss as Admin' : 'Make Group Admin', icon: flutterMenuIcon('manageUser'), action: () => onUpdate('ROLE', participant.userId, participant.role === 'ADMIN' ? undefined : 'ADMIN') },
    ] : []),
    ...(canRemove ? [
      { type: 'separator' as const },
      { type: 'item' as const, id: 'remove-participant', text: `Remove ${participant.fullName}`, icon: flutterMenuIcon('delete'), action: () => onUpdate('REMOVE', participant.userId) },
    ] : []),
  ]) : undefined}><Avatar name={participant.fullName || '?'} url={participant.avatarUrl} size={50} /><span><strong>{participant.fullName || '?'}<Badges verified={participant.isVerified} isBot={participant.isBot} membership={participant.membership} /></strong><small>{participant.identityNumber}</small></span>{participant.role && <em>{participant.role === 'OWNER' ? 'Owner' : 'Admin'}</em>}</article>;
  return item;
}

function MuteConversationDialog({ conversation, onClose, onMuted }: { conversation: Conversation; onClose: () => void; onMuted: () => Promise<void> }) {
  const [duration, setDuration] = useState<number>();
  const [saving, setSaving] = useState(false);
  const options: Array<[string, number]> = [['For 1 hour', 3600], ['For 8 hours', 28800], ['For 1 week', 604800], ['Forever', 2147483647]];
  async function confirm() {
    if (duration === undefined || saving) return;
    setSaving(true);
    try { await desktopApi.setConversationMuted(conversation.conversationId, conversation.ownerId, conversation.category, duration); await onMuted(); onClose(); } finally { setSaving(false); }
  }
  return <MixinDialog open onClose={onClose} maxWidth={400}><AlertDialogLayout title="Mute Notifications" minHeight={210} content={<div className={classes("mixin-dialog-options")}>{options.map(([label, value]) => <button type="button" key={value} className={classes(duration === value ? 'is-selected' : '')} onClick={() => setDuration(value)}><span>{duration === value ? '✓' : ''}</span>{label}</button>)}</div>} actions={<><MixinButton transparent onClick={onClose}>Cancel</MixinButton><MixinButton disabled={duration === undefined || saving} onClick={() => void confirm()}>Confirm</MixinButton></>} /></MixinDialog>;
}

function ConversationEditDialog({ title, value, announcement, onClose, onSave }: { title: string; value: string; announcement?: boolean; onClose: () => void; onSave: (value: string) => Promise<void> }) {
  const [next, setNext] = useState(value);
  const [saving, setSaving] = useState(false);
  async function save(event: FormEvent) {
    event.preventDefault();
    if ((!announcement && !next.trim()) || saving) return;
    setSaving(true);
    try { await onSave(next); } finally { setSaving(false); }
  }
  return <div className={classes("modal-backdrop")}><form className={classes("circle-dialog conversation-edit-dialog")} onSubmit={(event) => void save(event)}><header><strong>{title}</strong></header>{announcement ? <textarea autoFocus rows={7} maxLength={512} value={next} onChange={(event) => setNext(event.target.value)} /> : <input autoFocus maxLength={40} value={next} onChange={(event) => setNext(event.target.value)} />}<footer><button type="button" onClick={onClose}>Cancel</button><button type="submit" disabled={(!announcement && !next.trim()) || saving}>Change</button></footer></form></div>;
}

function GroupInviteDialog({ conversation, onClose }: { conversation: Conversation; onClose: () => void }) {
  const [detail, setDetail] = useState<ConversationDetail>();
  const [targets, setTargets] = useState<Conversation[]>();
  const [acting, setActing] = useState(false);
  useEffect(() => { void desktopApi.conversationDetail(conversation.conversationId).then(setDetail).catch((reason) => e('Unable to load conversation details', reason)); }, [conversation.conversationId]);
  async function rotate() {
    if (acting) return;
    setActing(true);
    try { await desktopApi.rotateGroupInvite(conversation.conversationId); setDetail(await desktopApi.conversationDetail(conversation.conversationId)); } finally { setActing(false); }
  }
  async function share(targetConversationId: string) {
    if (!detail?.codeUrl || acting) return;
    setActing(true);
    try { await desktopApi.sendText(targetConversationId, detail.codeUrl); setTargets(undefined); } finally { setActing(false); }
  }
  return <div className={classes("modal-backdrop")}><section className={classes("group-invite-dialog")} role="dialog" aria-modal="true"><button className={classes("user-profile-close")} type="button" onClick={onClose}>×</button><h2>Invite to Group via Link</h2>{detail && <><Avatar name={detail.name} url={conversation.avatarUrl} size={90} /><h3>{detail.name}</h3><p className={classes("group-invite-url")}>{detail.codeUrl}</p><p className={classes("group-invite-note")}>Anyone with this link can join this group.</p><footer><button type="button" disabled={!detail.codeUrl || acting} onClick={() => void desktopApi.bootstrap().then((data) => setTargets(data.conversations))}><span>↗</span>Share Link</button><button type="button" disabled={!detail.codeUrl} onClick={() => void navigator.clipboard.writeText(detail.codeUrl)}><span>⧉</span>Copy Invite</button><button type="button" disabled={acting} onClick={() => void rotate()}><span>↻</span>Reset Link</button></footer></>}{targets && <div className={classes("group-invite-targets")}><header><strong>Forward</strong><button type="button" onClick={() => setTargets(undefined)}>×</button></header>{targets.map((target) => <button type="button" key={target.conversationId} disabled={acting} onClick={() => void share(target.conversationId)}><Avatar name={target.name} url={target.avatarUrl} size={38} /><span>{target.name}</span></button>)}</div>}</section></div>;
}

function ContactShareDialog({ userId, onClose }: { userId: string; onClose: () => void }) {
  const [conversations, setConversations] = useState<Conversation[]>();
  const [query, setQuery] = useState('');
  const [sending, setSending] = useState(false);
  useEffect(() => { void desktopApi.bootstrap().then((data) => setConversations(data.conversations)).catch((reason) => { e('Unable to load conversations for sharing', reason); setConversations([]); }); }, []);
  const values = (conversations ?? []).filter((item) => !query || item.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  async function share(conversationId: string) {
    if (sending) return;
    setSending(true);
    try { await desktopApi.sendContact(conversationId, userId); onClose(); } finally { setSending(false); }
  }
  return <div className={classes("modal-backdrop")}><section className={classes("forward-dialog")} role="dialog" aria-modal="true"><header><strong>Share Contact</strong><button type="button" onClick={onClose}>×</button></header><input autoFocus placeholder="Search conversations" value={query} onChange={(event) => setQuery(event.target.value)} /><div>{values.map((conversation) => <button type="button" key={conversation.conversationId} disabled={sending} onClick={() => void share(conversation.conversationId)}><Avatar name={conversation.name} url={conversation.avatarUrl} size={42} /><span>{conversation.name}</span></button>)}</div></section></div>;
}

function SideActionConfirmation({ title, description, confirmLabel, action, onClose }: { title: string; description: string; confirmLabel: string; action: () => Promise<void>; onClose: () => void }) {
  const [saving, setSaving] = useState(false);
  async function confirm() {
    if (saving) return;
    setSaving(true);
    try { await action(); onClose(); } finally { setSaving(false); }
  }
  return <MixinDialog open onClose={onClose} maxWidth={400}><AlertDialogLayout title={title} content={description} actions={<><MixinButton transparent disabled={saving} onClick={onClose}>Cancel</MixinButton><MixinButton className={classes("is-danger")} disabled={saving} onClick={() => void confirm()}>{confirmLabel}</MixinButton></>} /></MixinDialog>;
}

function sameSharedDay(leftMicros: number, rightMicros: number) {
  const left = new Date(leftMicros / 1000);
  const right = new Date(rightMicros / 1000);
  return left.getFullYear() === right.getFullYear() && left.getMonth() === right.getMonth() && left.getDate() === right.getDate();
}

function SharedMessage({ message, showDate, onLocate }: { message: Message; showDate: boolean; onLocate: () => void }) {
  const source = resolveAssetUrl(message.thumbUrl || message.mediaUrl);
  const child = message.category.includes('IMAGE') || message.category.includes('VIDEO')
    ? source ? <img src={source} alt={message.caption || ''} /> : null
    : <article><strong>{message.mediaName || message.senderName}</strong><small>{message.caption || message.content || `[${message.category}]`}</small></article>;
  if (!child) return null;
  const date = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric' }).format(new Date(message.createdAtMicros / 1000));
  return <>{showDate && <div className={classes("shared-day")}>{date}</div>}<div className={classes("shared-message-item")} onContextMenu={(event) => void showNativeContextMenu(event, [{ type: 'item', id: 'locate-shared-message', text: 'Locate to Chat', icon: flutterMenuIcon('positionToChat'), action: onLocate }])}>{child}</div></>;
}

function CircleMembershipIcon({ selected = false }: { selected?: boolean }) {
  return selected
    ? <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="8" fill="#E57874" /><rect x="4" y="7" width="8" height="2" rx="1" fill="white" /></svg>
    : <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="8" fill="#50BD5C" /><rect x="4" y="7" width="8" height="2" rx="1" fill="white" /><rect x="7" y="4" width="2" height="8" rx="1" fill="white" /></svg>;
}

function CircleManagerIcon({ color }: { color: string }) {
  return <span className={classes("circle-manager-icon")}><svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M10.3333 4.79395C6.99335 5.55212 4.5 8.54023 4.5 12.1109C4.5 12.7684 4.58454 13.4062 4.74334 14.0139M13.6667 4.79395C17.0066 5.55212 19.5 8.54023 19.5 12.1109C19.5 12.7047 19.431 13.2824 19.3007 13.8364M6.27255 16.9552C7.64831 18.5814 9.70363 19.6138 12 19.6138C14.3561 19.6138 16.4584 18.527 17.8333 16.8272" stroke={color} strokeWidth="2" strokeLinecap="round" /></svg></span>;
}

function circleColor(id: string): string {
  const colors = ['#8E7BFF', '#657CFB', '#A739C2', '#BD6DDA', '#FD89F1', '#FA7B95', '#E94156', '#FA9652', '#F1D22B', '#BAE361', '#5EDD5E', '#4BE6FF', '#45B7FE', '#00ECD0', '#FFCCC0', '#CEA06B'];
  const components = id.trim().split('-');
  let hash: bigint;
  if (components.length === 5 && components.every((component) => /^[0-9a-f]+$/i.test(component))) {
    const high = (BigInt(`0x${components[0]}`) << 32n) | (BigInt(`0x${components[1]}`) << 16n) | BigInt(`0x${components[2]}`);
    const low = (BigInt(`0x${components[3]}`) << 48n) | BigInt(`0x${components[4]}`);
    const hilo = high ^ low;
    hash = (hilo >> 32n) ^ BigInt.asIntN(32, hilo);
  } else {
    hash = [...id].reduce((value, character) => BigInt.asIntN(32, value * 31n + BigInt(character.charCodeAt(0))), 0n);
  }
  const index = Number((hash < 0n ? -hash : hash) % BigInt(colors.length));
  return colors[index];
}

function durationLabel(seconds: number) {
  if (seconds === 0) return 'Off';
  if (seconds < 60) return `${seconds} seconds`;
  if (seconds < 3600) return `${seconds / 60} minutes`;
  if (seconds < 86400) return `${seconds / 3600} hours`;
  return `${seconds / 86400} days`;
}
