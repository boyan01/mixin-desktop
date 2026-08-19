import { useEffect, useRef, useState } from 'react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { Conversation, UserProfile } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { classes } from '@/shared/styles/classes';

export function movePaletteSelection(index: number, itemCount: number, delta: number): number {
  if (itemCount === 0) return 0;
  return Math.min(itemCount - 1, Math.max(0, index + delta));
}

export function CommandPalette({ conversations, recentConversationIds, onClose, onSelect }: { conversations: Conversation[]; recentConversationIds: string[]; onClose: () => void; onSelect: (conversationId?: string, userId?: string) => Promise<void> }) {
  const [query, setQuery] = useState('');
  const [users, setUsers] = useState<UserProfile[]>([]);
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const recent = recentConversationIds.map((id) => conversations.find((item) => item.conversationId === id)).filter((item): item is Conversation => item !== undefined);
  const keyword = query.trim().toLocaleLowerCase();
  useEffect(() => { input.current?.focus(); }, []);
  useEffect(() => {
    if (!keyword) { setUsers([]); return; }
    let active = true;
    const timer = window.setTimeout(() => void desktopApi.searchLocalUsers(keyword, 'chats', 100).then((items) => { if (active) setUsers(items); }).catch((reason) => { e('Unable to search local users', reason); if (active) setUsers([]); }), 100);
    return () => { active = false; window.clearTimeout(timer); };
  }, [keyword]);
  const entries = [...(keyword ? conversations.filter((item) => `${item.name} ${item.identityNumber}`.toLocaleLowerCase().includes(keyword)) : recent).map((item) => ({ id: item.conversationId, userId: undefined as string | undefined, name: item.name, identityNumber: item.identityNumber, avatarUrl: item.avatarUrl, kind: item.category, isVerified: item.isVerified, isBot: item.isBot, membership: item.membership })), ...users.map((item) => ({ id: item.userId, userId: item.userId, name: item.fullName, identityNumber: item.identityNumber, avatarUrl: item.avatarUrl, kind: item.isBot ? 'BOT' : 'USER', isVerified: item.isVerified, isBot: item.isBot, membership: item.membership }))].sort((left, right) => paletteScore(keyword, right.name, right.identityNumber) - paletteScore(keyword, left.name, left.identityNumber));
  useEffect(() => setSelected(0), [keyword]);
  useEffect(() => { list.current?.querySelector(`[data-palette-index="${selected}"]`)?.scrollIntoView({ block: 'nearest' }); }, [selected, entries.length]);
  function keyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Escape') { onClose(); return; }
    const delta = event.key === 'ArrowDown' || event.key === 'Tab' || (event.ctrlKey && event.key.toLowerCase() === 'n')
      ? 1
      : event.key === 'ArrowUp' || (event.ctrlKey && event.key.toLowerCase() === 'p')
        ? -1
        : 0;
    if (delta) {
      event.preventDefault();
      event.stopPropagation();
      setSelected((value) => movePaletteSelection(value, entries.length, delta));
      return;
    }
    if (event.key === 'Enter' && entries[selected]) {
      event.preventDefault();
      event.stopPropagation();
      void onSelect(entries[selected].userId ? undefined : entries[selected].id, entries[selected].userId);
    }
  }
  return <div className={classes("modal-backdrop command-palette-backdrop")} onMouseDown={onClose}><section className={classes("command-palette")} role="dialog" aria-modal="true" onMouseDown={(event) => event.stopPropagation()}><header><label><span>⌕</span><input ref={input} value={query} onKeyDown={keyDown} onChange={(event) => setQuery(event.target.value)} placeholder="Search" />{query && <button type="button" onClick={() => setQuery('')}>×</button>}</label><button type="button" onClick={onClose}>×</button></header>{entries.length ? <div ref={list}>{entries.map((entry, index) => <button type="button" data-palette-index={index} className={classes(index === selected ? 'is-selected' : '')} key={`${entry.kind}-${entry.id}`} onClick={() => void onSelect(entry.userId ? undefined : entry.id, entry.userId)}><Avatar name={entry.name} url={entry.avatarUrl} size={40} /><span><PaletteHighlightedText text={entry.name} keyword={query.trim()} /></span><Badges verified={entry.isVerified} isBot={entry.isBot} membership={entry.membership} /></button>)}</div> : <p>No results</p>}</section></div>;
}

function PaletteHighlightedText({ text, keyword }: { text: string; keyword: string }) {
  const index = text.toLocaleLowerCase().indexOf(keyword.toLocaleLowerCase());
  if (!keyword || index < 0) return text;
  return <>{text.slice(0, index)}<b>{text.slice(index, index + keyword.length)}</b>{text.slice(index + keyword.length)}</>;
}

function paletteScore(keyword: string, name: string, identityNumber: string) {
  if (!keyword) return 0;
  const value = name.toLocaleLowerCase();
  const identity = identityNumber.toLocaleLowerCase();
  return value === keyword || identity === keyword ? 100 : value.startsWith(keyword) || identity.startsWith(keyword) ? 80 : value.includes(keyword) || identity.includes(keyword) ? 60 : 0;
}
