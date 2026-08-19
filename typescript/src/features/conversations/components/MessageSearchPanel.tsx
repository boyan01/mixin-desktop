import { useEffect, useState } from 'react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { resolveDesktopShellLayout } from '@/shared/layout/desktopShellLayout';
import type { ConversationParticipant, Message } from '@/shared/model/types';
import { useDesktopUi } from '@/state/desktopUi';
import { Avatar } from '@/shared/ui/Avatar';
import { classes } from '@/shared/styles/classes';

export function MessageSearchPanel({ conversationId }: { conversationId: string }) {
  const open = useDesktopUi((state) => state.messageSearchOpen);
  const close = useDesktopUi((state) => state.closeMessageSearch);
  const focusMessage = useDesktopUi((state) => state.focusMessage);
  const sidebarCollapsed = useDesktopUi((state) => state.sidebarCollapsed);
  const [query, setQuery] = useState('');
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(false);
  const [userMode, setUserMode] = useState(false);
  const [selectedUser, setSelectedUser] = useState<ConversationParticipant>();
  const [participants, setParticipants] = useState<ConversationParticipant[]>([]);
  const [categories, setCategories] = useState<string[]>();

  useEffect(() => {
    setQuery(''); setMessages([]); setUserMode(false); setSelectedUser(undefined); setCategories(undefined);
  }, [conversationId]);

  useEffect(() => {
    if (!open || !userMode) return;
    void desktopApi.conversationParticipants(conversationId).then(setParticipants).catch((reason) => { e('Unable to load conversation participants', reason); setParticipants([]); });
  }, [conversationId, open, userMode]);

  useEffect(() => {
    const keyword = query.trim();
    if (!open || userMode && !selectedUser || !keyword) { setMessages([]); setLoading(false); return; }
    let active = true;
    const timer = window.setTimeout(() => {
      setLoading(true);
      void desktopApi.searchMessages(conversationId, keyword, selectedUser?.userId, categories).then((values) => {
        if (active) setMessages(values);
      }).catch((reason) => {
        e('Unable to search conversation messages', reason);
        if (active) setMessages([]);
      }).finally(() => { if (active) setLoading(false); });
    }, 150);
    return () => { active = false; window.clearTimeout(timer); };
  }, [categories, conversationId, open, query, selectedUser, userMode]);

  const filteredParticipants = participants.filter((participant) => !query.trim() || `${participant.fullName} ${participant.identityNumber}`.toLowerCase().includes(query.trim().toLowerCase()));
  const filterOptions: Array<{ label: string; values: string[] }> = [{ label: 'Text', values: ['PLAIN_TEXT', 'SIGNAL_TEXT', 'ENCRYPTED_TEXT'] }, { label: 'Post', values: ['PLAIN_POST', 'SIGNAL_POST', 'ENCRYPTED_POST'] }];
  function closeAfterContentJump() {
    const layout = resolveDesktopShellLayout(window.innerWidth, sidebarCollapsed);
    if (layout.routeMode || layout.sideRouteMode) close(false);
  }

  if (!open) return null;
  return (
    <aside className={classes("message-search-panel")}>
      <header><strong>Search Conversation</strong><button type="button" onClick={() => close()}>×</button></header>
      <div className={classes("message-search-field")}><span>{userMode ? 'From:' : ''}</span>{selectedUser && <button type="button" className={classes("selected-search-user")} onClick={() => setSelectedUser(undefined)}><Avatar name={selectedUser.fullName} url={selectedUser.avatarUrl} size={20} /> {selectedUser.fullName} ×</button>}<input autoFocus value={query} onKeyDown={(event) => { if (event.key === 'Backspace' && !query && selectedUser) setSelectedUser(undefined); else if (event.key === 'Backspace' && !query && userMode) setUserMode(false); }} onChange={(event) => setQuery(event.target.value)} placeholder="Search" />{!userMode && <button type="button" className={classes("search-user-button")} aria-label="Search by sender" onClick={() => { setQuery(''); setUserMode(true); }}>⌕</button>}</div>
      {!userMode || selectedUser ? <div className={classes("message-search-filters")}>{query.trim() && filterOptions.map((filter) => <button type="button" key={filter.label} className={classes(categories?.join() === filter.values.join() ? 'is-selected' : '')} onClick={() => setCategories((current) => current?.join() === filter.values.join() ? undefined : filter.values)}>{filter.label}</button>)}</div> : null}
      <div className={classes("message-search-results")}>
        {userMode && !selectedUser && filteredParticipants.map((participant) => <button type="button" key={participant.userId} onClick={() => { setSelectedUser(participant); setQuery(''); }}><Avatar name={participant.fullName} url={participant.avatarUrl} size={38} /><span><strong>{participant.fullName}</strong><p>{participant.identityNumber}</p></span></button>)}
        {messages.map((message) => (
          <button type="button" key={message.messageId} onClick={() => { focusMessage(message.messageId); closeAfterContentJump(); }}>
            <Avatar name={message.senderName} url={message.senderAvatarUrl} size={34} />
            <span><strong>{message.senderName}</strong><p>{message.content || `[${message.category}]`}</p></span>
          </button>
        ))}
        {!userMode && !loading && query && messages.length === 0 && <p className={classes("empty-list")}>No messages found</p>}
      </div>
    </aside>
  );
}
