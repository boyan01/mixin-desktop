import { useEffect, useState } from 'react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { Conversation, UserProfile } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { classes } from '@/shared/styles/classes';

export function ForwardDialog({ conversations, messageIds, combined, onClose }: { conversations: Conversation[]; messageIds: string[]; combined: boolean; onClose: () => void }) {
  const [query, setQuery] = useState('');
  const [sending, setSending] = useState(false);
  const [selected, setSelected] = useState(new Set<string>());
  const [users, setUsers] = useState<UserProfile[]>([]);
  useEffect(() => { void desktopApi.selectableUsers().then(setUsers).catch((reason) => { e('Unable to load contacts for forwarding', reason); setUsers([]); }); }, []);
  const existingOwners = new Set(conversations.map((item) => item.ownerId));
  const values = [...conversations, ...users.filter((user) => !existingOwners.has(user.userId)).map((user) => ({ conversationId: `user:${user.userId}`, ownerId: user.userId, name: user.fullName, avatarUrl: user.avatarUrl, identityNumber: user.identityNumber }))].filter((item) => !query || `${item.name} ${item.identityNumber}`.toLowerCase().includes(query.toLowerCase())).slice(0, 100);
  async function forward() {
    if (sending || selected.size === 0) return;
    setSending(true);
    try {
      await Promise.all([...selected].map(async (id) => {
        const conversationId = id.startsWith('user:') ? await desktopApi.openUserConversation(id.slice(5)) : id;
        await desktopApi.forwardMessages(conversationId, messageIds, combined);
      }));
      onClose();
    }
    finally { setSending(false); }
  }
  const selectedValues = values.filter((item) => selected.has(item.conversationId));
  return <div className={classes("modal-backdrop")}><section className={classes("forward-dialog")}><header><button type="button" onClick={onClose}>×</button><strong>{combined ? 'Combine and Forward' : 'Forward Messages'}<small>{selected.size} / {values.length}</small></strong><button type="button" className={classes("forward-confirm")} disabled={selected.size === 0 || sending} onClick={() => void forward()}>Next</button></header><label><span>⌕</span><input autoFocus placeholder="Search" value={query} onChange={(event) => setQuery(event.target.value.slice(0, 200))} /></label>{selectedValues.length > 0 && <div className={classes("forward-selected")}>{selectedValues.map((conversation) => <button type="button" key={conversation.conversationId} onClick={() => setSelected((current) => { const next = new Set(current); next.delete(conversation.conversationId); return next; })}><Avatar name={conversation.name} url={conversation.avatarUrl} size={50} /><small>{conversation.name}</small><i>×</i></button>)}</div>}<div className={classes("forward-list")}>{values.map((conversation) => { const active = selected.has(conversation.conversationId); return <button type="button" className={classes(active ? 'is-selected' : '')} key={conversation.conversationId} disabled={sending} onClick={() => setSelected((current) => { const next = new Set(current); if (active) next.delete(conversation.conversationId); else next.add(conversation.conversationId); return next; })}><Avatar name={conversation.name} url={conversation.avatarUrl} size={50} /><span><strong>{conversation.name}</strong><small>{conversation.identityNumber}</small></span><i>{active ? '✓' : ''}</i></button>; })}</div></section></div>;
}
