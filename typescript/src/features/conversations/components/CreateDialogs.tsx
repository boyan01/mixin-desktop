import { useEffect, useMemo, useState, type FormEvent } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { LoaderCircle, Search, X } from 'lucide-react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { Conversation, UserProfile } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { IconButton } from '@/shared/ui/IconButton';
import { MixinDialog } from '@/shared/ui/MixinDialog';
import { classes } from '@/shared/styles/classes';

interface CreateDialogProps {
  mode?: 'search-contact' | 'conversation' | 'group' | 'circle';
  conversations: Conversation[];
  onClose: () => void;
  onConversationOpened?: (conversationId: string) => void;
  onCircleCreated?: () => void;
}

export function CreateDialog({ mode, conversations, onClose, onConversationOpened, onCircleCreated }: CreateDialogProps) {
  return (
    <MixinDialog open={Boolean(mode)} onClose={onClose} className={classes("dialog-content")}>
          {mode === 'search-contact' && <SearchContact onClose={onClose} onOpened={onConversationOpened} />}
          {mode === 'conversation' && <ConversationSelector conversations={conversations} onClose={onClose} onOpened={onConversationOpened} />}
          {mode === 'group' && <NewGroup onClose={onClose} onOpened={onConversationOpened} />}
          {mode === 'circle' && <NewCircle conversations={conversations} onClose={onClose} onCreated={onCircleCreated} />}
    </MixinDialog>
  );
}

function DialogHeader({ title, onClose }: { title: string; onClose: () => void }) {
  return (
    <header className={classes("dialog-header")}>
      <Dialog.Title>{title}</Dialog.Title>
      <IconButton label="Close" onClick={onClose}><X size={18} /></IconButton>
    </header>
  );
}

function SearchContact({ onClose, onOpened }: {
  onClose: () => void;
  onOpened?: (conversationId: string) => void;
}) {
  const [query, setQuery] = useState('');
  const [user, setUser] = useState<UserProfile>();
  const [error, setError] = useState<string>();
  const [loading, setLoading] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (query.trim().length < 4 || loading) return;
    setLoading(true);
    setError(undefined);
    try {
      setUser(await desktopApi.searchUser(query));
    } catch {
      setUser(undefined);
      setError('User not found');
    } finally {
      setLoading(false);
    }
  }

  async function openConversation() {
    if (!user || loading) return;
    setLoading(true);
    try {
      const conversationId = await desktopApi.openUserConversation(user.userId);
      onOpened?.(conversationId);
      onClose();
    } catch {
      setError('Unable to open conversation');
      setLoading(false);
    }
  }

  return (
    <>
      <DialogHeader title="Add Contact" onClose={onClose} />
      <form className={classes("dialog-form")} onSubmit={submit}>
        <label className={classes("dialog-input")}><Search size={17} /><input autoFocus value={query} onChange={(event) => setQuery(event.target.value.replace(/[^0-9+]/g, '').slice(0, 128))} placeholder="Mixin ID or phone number" /></label>
        <button className={classes("primary-button")} disabled={query.trim().length < 4 || loading} type="submit">Search</button>
      </form>
      {loading && <div className={classes("dialog-loading")}><LoaderCircle className={classes("login-spinner")} size={24} /></div>}
      {error && <p className={classes("dialog-error")}>{error}</p>}
      {user && !loading && (
        <button className={classes("user-result")} type="button" onClick={() => void openConversation()}>
          <Avatar name={user.fullName} url={user.avatarUrl} size={48} />
          <span><strong>{user.fullName}</strong><small>Mixin ID {user.identityNumber}</small></span>
          <span className={classes("primary-button")}>Message</span>
        </button>
      )}
    </>
  );
}

function ConversationSelector({ conversations, onClose, onOpened }: {
  conversations: Conversation[];
  onClose: () => void;
  onOpened?: (conversationId: string) => void;
}) {
  const [users, setUsers] = useState<UserProfile[]>([]);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  useEffect(() => {
    void desktopApi.selectableUsers().then(setUsers).catch((reason) => { e('Unable to load contacts', reason); setError('Unable to load contacts'); }).finally(() => setLoading(false));
  }, []);
  const entries = useMemo(() => {
    const existing = conversations.filter((item) => item.category !== 'GROUP' && (item.isBot || item.relationship !== 'STRANGER'))
      .map((item) => ({ id: item.conversationId, ownerId: item.ownerId, name: item.name, avatarUrl: item.avatarUrl, identityNumber: item.identityNumber, isExisting: true, isBot: item.isBot }));
    const ownerIds = new Set(existing.map((item) => item.ownerId));
    const available = users.filter((item) => !ownerIds.has(item.userId))
      .map((item) => ({ id: item.userId, ownerId: item.userId, name: item.fullName, avatarUrl: item.avatarUrl, identityNumber: item.identityNumber, isExisting: false, isBot: item.isBot }));
    const keyword = query.trim().toLocaleLowerCase();
    return [...existing, ...available].filter((item) => !keyword || `${item.name} ${item.identityNumber}`.toLocaleLowerCase().includes(keyword));
  }, [conversations, query, users]);
  async function open(entry: (typeof entries)[number]) {
    if (loading) return;
    setLoading(true);
    try {
      onOpened?.(entry.isExisting ? entry.id : await desktopApi.openUserConversation(entry.ownerId));
      onClose();
    } catch {
      setError('Unable to open conversation');
      setLoading(false);
    }
  }
  return <>
    <DialogHeader title="New Conversation" onClose={onClose} />
    <label className={classes("dialog-input selector-search")}><Search size={17} /><input autoFocus value={query} onChange={(event) => setQuery(event.target.value.slice(0, 200))} placeholder="Search" /></label>
    {loading && users.length === 0 ? <div className={classes("dialog-loading")}><LoaderCircle className={classes("login-spinner")} size={24} /></div> : <div className={classes("conversation-selector-list")}>
      {entries.filter((item) => !item.isBot).length > 0 && <h3>Contacts</h3>}
      {entries.filter((item) => !item.isBot).map((item) => <ConversationSelectorRow key={item.id} entry={item} onClick={() => void open(item)} />)}
      {entries.filter((item) => item.isBot).length > 0 && <h3>Bots</h3>}
      {entries.filter((item) => item.isBot).map((item) => <ConversationSelectorRow key={item.id} entry={item} onClick={() => void open(item)} />)}
    </div>}
    {error && <p className={classes("dialog-error")}>{error}</p>}
  </>;
}

function ConversationSelectorRow({ entry, onClick }: { entry: { name: string; avatarUrl: string; identityNumber: string }; onClick: () => void }) {
  return <button className={classes("conversation-selector-row")} type="button" onClick={onClick}><Avatar name={entry.name} url={entry.avatarUrl} size={50} /><strong>{entry.name}</strong></button>;
}

function SelectionList({ items, selected, onToggle }: {
  items: Array<{ id: string; name: string; avatarUrl: string; detail: string }>;
  selected: Set<string>;
  onToggle: (id: string) => void;
}) {
  const [query, setQuery] = useState('');
  const visible = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase();
    return keyword ? items.filter((item) => `${item.name} ${item.detail}`.toLocaleLowerCase().includes(keyword)) : items;
  }, [items, query]);
  return (
    <>
      <label className={classes("dialog-input selector-search")}><Search size={17} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search" /></label>
      <div className={classes("selection-list")}>
        {visible.map((item) => (
          <label key={item.id} className={classes("selection-row")}>
            <input type="checkbox" checked={selected.has(item.id)} onChange={() => onToggle(item.id)} />
            <Avatar name={item.name} url={item.avatarUrl} size={38} />
            <span><strong>{item.name}</strong><small>{item.detail}</small></span>
          </label>
        ))}
      </div>
    </>
  );
}

function NewGroup({ onClose, onOpened }: { onClose: () => void; onOpened?: (id: string) => void }) {
  const [users, setUsers] = useState<UserProfile[]>([]);
  const [selected, setSelected] = useState(new Set<string>());
  const [name, setName] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  useEffect(() => {
    void desktopApi.selectableUsers().then(setUsers).catch((reason) => { e('Unable to load contacts', reason); setError('Unable to load contacts'); }).finally(() => setLoading(false));
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!name.trim() || selected.size === 0 || loading) return;
    setLoading(true);
    try {
      const id = await desktopApi.createGroup(name.trim(), [...selected]);
      onOpened?.(id);
      onClose();
    } catch {
      setError('Unable to create group');
      setLoading(false);
    }
  }

  return (
    <form onSubmit={submit}>
      <DialogHeader title="New Group" onClose={onClose} />
      <div className={classes("create-name")}><input value={name} maxLength={64} onChange={(event) => setName(event.target.value)} placeholder="Group name" /></div>
      {loading && users.length === 0 ? <div className={classes("dialog-loading")}><LoaderCircle className={classes("login-spinner")} size={24} /></div> : (
        <SelectionList items={users.map((user) => ({ id: user.userId, name: user.fullName, avatarUrl: user.avatarUrl, detail: `Mixin ID ${user.identityNumber}` }))} selected={selected} onToggle={(id) => setSelected((current) => {
          const next = new Set(current);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        })} />
      )}
      {error && <p className={classes("dialog-error")}>{error}</p>}
      <footer className={classes("dialog-footer")}><button type="button" className={classes("secondary-button")} onClick={onClose}>Cancel</button><button type="submit" className={classes("primary-button")} disabled={!name.trim() || selected.size === 0 || loading}>Create</button></footer>
    </form>
  );
}

function NewCircle({ conversations, onClose, onCreated }: { conversations: Conversation[]; onClose: () => void; onCreated?: () => void }) {
  const [name, setName] = useState('');
  const [selected, setSelected] = useState(new Set<string>());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();

  async function submit(event: FormEvent) {
    event.preventDefault();
    const value = name.trim();
    if (!value || loading) return;
    setLoading(true);
    try {
      const circle = await desktopApi.createCircle(value);
      for (const conversation of conversations.filter((item) => selected.has(item.conversationId))) {
        await desktopApi.editCircleConversation(circle.circleId, conversation.conversationId, conversation.ownerId, conversation.category === 'GROUP', true);
      }
      onCreated?.();
      onClose();
    } catch {
      setError('Unable to create circle');
      setLoading(false);
    }
  }

  return (
    <>
      <DialogHeader title="New Circle" onClose={onClose} />
      <form className={classes("circle-form")} onSubmit={submit}>
        <input autoFocus value={name} maxLength={64} onChange={(event) => setName(event.target.value)} placeholder="Circle name" />
        <SelectionList items={conversations.map((item) => ({ id: item.conversationId, name: item.name, avatarUrl: item.avatarUrl, detail: item.category === 'GROUP' ? `${item.participantCount} participants` : `Mixin ID ${item.identityNumber}` }))} selected={selected} onToggle={(id) => setSelected((current) => {
          const next = new Set(current);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        })} />
        {error && <p className={classes("dialog-error")}>{error}</p>}
        <footer><button type="button" className={classes("secondary-button")} onClick={onClose}>Cancel</button><button type="submit" className={classes("primary-button")} disabled={!name.trim() || loading}>Create</button></footer>
      </form>
    </>
  );
}
