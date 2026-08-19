import { useEffect, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Avatar } from '@/shared/ui/Avatar';
import { AlertDialogLayout, MixinButton, MixinDialog } from '@/shared/ui/MixinDialog';
import { useDesktopUi } from '@/state/desktopUi';
import { desktopApi } from '@/shared/api/desktopApi';
import type { Circle, Conversation, ConversationCategory, Profile, UnseenCount } from '@/shared/model/types';
import { matchesCategory } from '@/features/conversations/lib/conversationPresentation';
import chatAsset from '@flutter-assets/images/chat.svg';
import contactsAsset from '@flutter-assets/images/slide_contacts.svg';
import groupsAsset from '@flutter-assets/images/group.svg';
import botsAsset from '@flutter-assets/images/bot.svg';
import strangersAsset from '@flutter-assets/images/strangers.svg';
import circleAsset from '@flutter-assets/images/circle.svg';
import collapseAsset from '@flutter-assets/images/collapse.svg';
import expandedAsset from '@flutter-assets/images/expanded.svg';
import { classes } from '@/shared/styles/classes';
import { flutterMenuIcon } from '@/shared/ui/flutterMenuIcon';
import { showNativeContextMenu } from '@/shared/ui/nativeMenu';

interface SidebarProps {
  profile: Profile;
  conversations: Conversation[];
  collapsed?: boolean;
  showCollapseControl?: boolean;
  onClose?: () => void;
}

const categories: Array<{
  id: ConversationCategory;
  label: string;
  asset: string;
}> = [
  { id: 'chats', label: 'All Chats', asset: chatAsset },
  { id: 'contacts', label: 'Contacts', asset: contactsAsset },
  { id: 'groups', label: 'Groups', asset: groupsAsset },
  { id: 'bots', label: 'Bots', asset: botsAsset },
  { id: 'strangers', label: 'Strangers', asset: strangersAsset },
];

function badgeCount(counts: UnseenCount[], category: ConversationCategory, circleId?: string) {
  return counts.find((item) => item.category === category && item.circleId === circleId)?.count ?? 0;
}

function mutedBadgeCount(counts: UnseenCount[], category: ConversationCategory, circleId?: string) {
  return counts.find((item) => item.category === category && item.circleId === circleId)?.mutedCount ?? 0;
}

export function Sidebar({
  profile,
  conversations,
  collapsed: forcedCollapsed,
  showCollapseControl = true,
  onClose,
}: SidebarProps) {
  const queryClient = useQueryClient();
  const storedCollapsed = useDesktopUi((state) => state.sidebarCollapsed);
  const collapsed = forcedCollapsed ?? storedCollapsed;
  const category = useDesktopUi((state) => state.category);
  const circleId = useDesktopUi((state) => state.circleId);
  const setCategory = useDesktopUi((state) => state.setCategory);
  const toggleSidebar = useDesktopUi((state) => state.toggleSidebar);
  const openSettings = useDesktopUi((state) => state.openSettings);
  const settingsOpen = useDesktopUi((state) => state.settingsOpen);
  const circles = useQuery({ queryKey: ['circles'], queryFn: desktopApi.circles });
  const [orderedCircles, setOrderedCircles] = useState<Circle[]>([]);
  const [editing, setEditing] = useState<Circle>();
  const [managing, setManaging] = useState<Circle>();
  const [deleting, setDeleting] = useState<Circle>();

  useEffect(() => setOrderedCircles(circles.data ?? []), [circles.data]);

  const unreadConversations = conversations.filter((conversation) => conversation.unseenCount > 0);
  const unseenTotal = (items: Conversation[]) => items.reduce((total, item) => total + item.unseenCount, 0);
  const derivedCounts: UnseenCount[] = [
    { category: 'chats', circleId: null, count: unseenTotal(unreadConversations), mutedCount: unseenTotal(unreadConversations.filter((item) => item.isMuted)) },
    ...categories.slice(1).map(({ id }) => ({
      category: id,
      circleId: null,
      count: unseenTotal(unreadConversations.filter((conversation) => matchesCategory(conversation, id))),
      mutedCount: unseenTotal(unreadConversations.filter((conversation) => conversation.isMuted && matchesCategory(conversation, id))),
    })),
  ];

  async function reorder(from: number, to: number) {
    if (from === to) return;
    const previous = orderedCircles;
    const next = [...previous];
    next.splice(to, 0, next.splice(from, 1)[0]);
    setOrderedCircles(next);
    try {
      await desktopApi.reorderCircles(next.map((circle) => circle.circleId));
    } catch {
      setOrderedCircles(previous);
    }
  }

  async function deleteCircle(circle: Circle) {
    await desktopApi.deleteCircle(circle.circleId);
    if (category === 'circle' && circleId === circle.circleId) setCategory('chats');
    await queryClient.invalidateQueries({ queryKey: ['circles'] });
  }

  return (
    <aside className={classes(`sidebar ${collapsed ? 'sidebar--collapsed' : ''}`)}>
      <div className={classes("titlebar-drag")} data-tauri-drag-region />
      <span className={classes("sidebar-item-wrap profile-item-wrap")}>
        <button className={classes(`profile-row ${settingsOpen ? 'is-selected' : ''}`)} type="button" aria-label={collapsed ? profile.fullName : undefined} onClick={() => { openSettings(); onClose?.(); }}>
          <span className={classes("profile-avatar")}><Avatar name={profile.fullName} url={profile.avatarUrl} size={24} /></span>
          {!collapsed && <span className={classes("profile-copy")}><strong>{profile.fullName}</strong><small>{profile.identityNumber}</small></span>}
        </button>
        {collapsed && <SidebarTooltip label={profile.fullName} subtitle={profile.identityNumber} />}
      </span>
      <nav className={classes("sidebar-nav")} aria-label="Conversation categories">
        {categories.map(({ id, label, asset }, index) => index === 1 ? (
          <span key={`divider-${id}`} className={classes("sidebar-category-group")}>
            <span className={classes("sidebar-category-divider")} aria-hidden="true" />
            <SidebarItem
              label={label}
              asset={asset}
              collapsed={collapsed}
              selected={category === id}
              count={badgeCount(derivedCounts, id)}
              mutedCount={mutedBadgeCount(derivedCounts, id)}
              onClick={() => { setCategory(id); onClose?.(); }}
            />
          </span>
        ) : (
          <SidebarItem
            key={id}
            label={label}
            asset={asset}
            collapsed={collapsed}
            selected={category === id}
            count={badgeCount(derivedCounts, id)}
            mutedCount={mutedBadgeCount(derivedCounts, id)}
            onClick={() => { setCategory(id); onClose?.(); }}
          />
        ))}
      </nav>
      {orderedCircles.length > 0 && <div className={classes("sidebar-divider")} />}
      <nav className={classes("sidebar-circles")} aria-label="Circles">
        {orderedCircles.map((circle, index) => {
          const unseen = conversations.filter((conversation) => conversation.unseenCount > 0 && conversation.circleIds.includes(circle.circleId));
          const count = unseenTotal(unseen);
          const mutedCount = unseenTotal(unseen.filter((conversation) => conversation.isMuted));
          return (
            <SidebarItem
              key={circle.circleId}
              label={circle.name}
              asset={circleAsset}
              iconColor={circleColor(circle.circleId)}
              collapsed={collapsed}
              selected={category === 'circle' && circleId === circle.circleId}
              count={count}
              mutedCount={mutedCount}
              draggable
              onDragStart={(event) => event.dataTransfer.setData('application/x-mixin-circle-index', String(index))}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => { const from = Number(event.dataTransfer.getData('application/x-mixin-circle-index')); if (Number.isInteger(from)) void reorder(from, index); }}
              onClick={() => { setCategory('circle', circle.circleId); onClose?.(); }}
              onContextMenu={(event) => void showNativeContextMenu(event, [
                { type: 'item', id: 'edit-circle-name', text: 'Edit Circle Name', icon: flutterMenuIcon('edit'), action: () => setEditing(circle) },
                { type: 'item', id: 'edit-circle-conversations', text: 'Edit Conversations', icon: flutterMenuIcon('manageCircle'), action: () => setManaging(circle) },
                { type: 'separator' },
                { type: 'item', id: 'delete-circle', text: 'Delete Circle', icon: flutterMenuIcon('delete'), action: () => setDeleting(circle) },
              ])}
            />
          );
        })}
      </nav>
      {showCollapseControl && <div className={classes("sidebar-footer")}><SidebarItem label={collapsed ? 'Expand' : 'Collapse'} asset={collapsed ? expandedAsset : collapseAsset} collapsed={collapsed} selected={false} count={0} mutedCount={0} onClick={toggleSidebar} /></div>}
      {editing && <CircleNameDialog circle={editing} onClose={() => setEditing(undefined)} onSaved={() => void queryClient.invalidateQueries({ queryKey: ['circles'] })} />}
      {managing && <CircleConversationsDialog circle={managing} conversations={conversations} onClose={() => setManaging(undefined)} onSaved={() => void queryClient.invalidateQueries({ queryKey: ['bootstrap'] })} />}
      {deleting && <CircleDeleteDialog circle={deleting} onClose={() => setDeleting(undefined)} onDelete={() => void deleteCircle(deleting)} />}
    </aside>
  );
}

interface SidebarItemProps {
  label: string;
  asset: string;
  iconColor?: string;
  collapsed: boolean;
  selected: boolean;
  count: number;
  mutedCount: number;
  onClick: () => void;
  draggable?: boolean;
  onDragStart?: React.DragEventHandler<HTMLButtonElement>;
  onDragOver?: React.DragEventHandler<HTMLButtonElement>;
  onDrop?: React.DragEventHandler<HTMLButtonElement>;
  onContextMenu?: React.MouseEventHandler<HTMLButtonElement>;
}

function SidebarItem({ label, asset, iconColor, collapsed, selected, count, mutedCount, onClick, draggable, onDragStart, onDragOver, onDrop, onContextMenu }: SidebarItemProps) {
  return (
    <span className={classes("sidebar-item-wrap")}>
      <button
        type="button"
        draggable={draggable}
        className={classes(`sidebar-item ${selected ? 'is-selected' : ''}`)}
        aria-current={selected ? 'page' : undefined}
        aria-label={collapsed ? label : undefined}
        onClick={onClick}
        onDragStart={onDragStart}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onContextMenu={onContextMenu}
      >
        <SidebarIcon src={asset} color={iconColor} />
        {!collapsed && <span className={classes("sidebar-label")}>{label}</span>}
        {count > 0 && <span className={classes(`sidebar-badge ${count === mutedCount ? 'is-muted' : ''}`)}>{count}</span>}
      </button>
      {collapsed && <SidebarTooltip label={label} count={count} />}
    </span>
  );
}

function SidebarTooltip({ label, subtitle, count }: { label: string; subtitle?: string; count?: number }) {
  return <span className={classes("sidebar-tooltip")} role="tooltip"><span className={classes("sidebar-tooltip-copy")}><strong>{label}</strong>{subtitle && <small>{subtitle}</small>}</span>{count ? <span className={classes("sidebar-tooltip-badge")}>{count}</span> : null}</span>;
}

function SidebarIcon({ src, color }: { src: string; color?: string }) {
  return (
    <span
      aria-hidden="true"
      className={classes("sidebar-icon")}
      style={{
        '--sidebar-icon-image': `url("${src}")`,
        ...(color ? { backgroundColor: color } : {}),
      } as React.CSSProperties}
    />
  );
}

function CircleNameDialog({ circle, onClose, onSaved }: { circle: Circle; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(circle.name);
  const [saving, setSaving] = useState(false);
  async function save() {
    const value = name.trim();
    if (!value || saving) return;
    setSaving(true);
    try { await desktopApi.updateCircle(circle.circleId, value); onSaved(); onClose(); } finally { setSaving(false); }
  }
  return <div className={classes("modal-backdrop")}><form className={classes("circle-dialog")} onSubmit={(event) => { event.preventDefault(); void save(); }}><header><strong>Edit Circle Name</strong></header><input autoFocus value={name} maxLength={64} onChange={(event) => setName(event.target.value)} /><footer><button type="button" onClick={onClose}>Cancel</button><button type="submit" disabled={!name.trim() || saving}>Save</button></footer></form></div>;
}

function CircleConversationsDialog({ circle, conversations, onClose, onSaved }: { circle: Circle; conversations: Conversation[]; onClose: () => void; onSaved: () => void }) {
  const [selected, setSelected] = useState(() => new Set(conversations.filter((item) => item.circleIds.includes(circle.circleId)).map((item) => item.conversationId)));
  const [saving, setSaving] = useState(false);
  async function save() {
    if (saving) return;
    setSaving(true);
    try {
      await Promise.all(conversations.flatMap((conversation) => {
        const currentlyIncluded = conversation.circleIds.includes(circle.circleId);
        const include = selected.has(conversation.conversationId);
        return currentlyIncluded === include ? [] : [desktopApi.editCircleConversation(circle.circleId, conversation.conversationId, conversation.ownerId, conversation.category === 'GROUP', include)];
      }));
      onSaved(); onClose();
    } finally { setSaving(false); }
  }
  return <div className={classes("modal-backdrop")}><form className={classes("circle-dialog circle-conversations-dialog")} onSubmit={(event) => { event.preventDefault(); void save(); }}><header><strong>{circle.name}</strong></header><div>{conversations.map((conversation) => <label key={conversation.conversationId}><input type="checkbox" checked={selected.has(conversation.conversationId)} onChange={() => setSelected((current) => { const next = new Set(current); if (next.has(conversation.conversationId)) next.delete(conversation.conversationId); else next.add(conversation.conversationId); return next; })} /><Avatar name={conversation.name} url={conversation.avatarUrl} size={38} /><span>{conversation.name}</span></label>)}</div><footer><button type="button" onClick={onClose}>Cancel</button><button type="submit" disabled={saving}>Done</button></footer></form></div>;
}

function CircleDeleteDialog({ circle, onClose, onDelete }: { circle: Circle; onClose: () => void; onDelete: () => void }) {
  const [deleting, setDeleting] = useState(false);
  async function confirm() {
    if (deleting) return;
    setDeleting(true);
    try { await onDelete(); onClose(); } finally { setDeleting(false); }
  }
  return <MixinDialog open onClose={onClose} maxWidth={400}><AlertDialogLayout title="Delete Circle" content={<>Delete the circle “{circle.name}”?</>} actions={<><MixinButton transparent disabled={deleting} onClick={onClose}>Cancel</MixinButton><MixinButton className={classes("is-danger")} disabled={deleting} onClick={() => void confirm()}>Delete</MixinButton></>} /></MixinDialog>;
}

function circleColor(value: string): string {
  const colors = ['#8E7BFF', '#657CFB', '#A739C2', '#BD6DDA', '#FD89F1', '#FA7B95', '#E94156', '#FA9652', '#F1D22B', '#BAE361', '#5EDD5E', '#4BE6FF', '#45B7FE', '#00ECD0', '#FFCCC0', '#CEA06B'];
  const parts = value.trim().split('-');
  if (parts.length === 5 && parts.every((part) => /^[0-9a-f]+$/i.test(part))) {
    try {
      const mostSignificantBits = (BigInt(`0x${parts[0]}`) << 32n)
        | (BigInt(`0x${parts[1]}`) << 16n)
        | BigInt(`0x${parts[2]}`);
      const leastSignificantBits = (BigInt(`0x${parts[3]}`) << 48n)
        | BigInt(`0x${parts[4]}`);
      const hilo = mostSignificantBits ^ leastSignificantBits;
      const hash = Number((hilo >> 32n) ^ BigInt.asIntN(32, hilo));
      return colors[Math.abs(hash) % colors.length];
    } catch {
      // Use the stable string fallback for malformed UUID-like values.
    }
  }
  let hash = 0;
  for (const character of value) hash = (hash * 31 + character.charCodeAt(0)) | 0;
  return colors[Math.abs(hash) % colors.length];
}
