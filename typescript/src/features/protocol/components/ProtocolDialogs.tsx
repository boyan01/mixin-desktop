import { useEffect, useState } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { sendProtocolPayload, type ProtocolSendPayload } from '@/features/protocol/lib/protocolPayload';
import type { CodeResult, Conversation, SnapshotDetail, UserProfile } from '@/shared/model/types';
import { SnapshotDetailDialog } from '@/features/wallet/components/SnapshotDetailDialog';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { showToastFailed } from '@/shared/ui/toast';
import { classes } from '@/shared/styles/classes';

export function JoinGroupCodeDialog({ code, result, onClose, onJoined }: { code: string; result: CodeResult; onClose: () => void; onJoined: (conversationId: string) => void }) {
  const [joining, setJoining] = useState(false);
  async function join() {
    if (joining) return;
    setJoining(true);
    try { onJoined(await desktopApi.joinGroup(code)); } catch (error) { showToastFailed(error); } finally { setJoining(false); }
  }
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("join-group-code-dialog")} role="dialog" aria-modal="true" aria-label="Join Group" onMouseDown={(event) => event.stopPropagation()}><button type="button" aria-label="Close" onClick={onClose}>×</button><div className={classes("join-group-avatars")}>{result.participantAvatars.slice(0, 4).map((avatar, index) => <span key={avatar.userId} style={{ zIndex: 4 - index }}><Avatar name={avatar.name} url={avatar.avatarUrl} size={48} /></span>)}</div><strong>{result.conversationName}</strong><small>{result.participantCount} participants</small><button className={classes("primary-button")} type="button" disabled={joining} onClick={() => void join()}>{joining ? 'Joining…' : '+ Join Group'}</button></section></div>;
}

export function ProtocolSendDialog({ conversations, payload, conversationId, onClose, onSent }: { conversations: Conversation[]; payload: ProtocolSendPayload; conversationId?: string; onClose: () => void; onSent: (conversationId: string) => Promise<void> }) {
  const [selectorOpen, setSelectorOpen] = useState(false);
  const [sending, setSending] = useState(false);
  const categoryName: Record<ProtocolSendPayload['category'], string> = { text: 'Text', image: 'Image', sticker: 'Sticker', contact: 'Contact', post: 'Post', app_card: 'Card' };
  async function send(target: string) {
    if (sending) return;
    setSending(true);
    try {
      await sendProtocolPayload(target, payload);
      await onSent(target);
    } catch (error) { showToastFailed(error); } finally { setSending(false); }
  }
  if (selectorOpen) return <ProtocolConversationSelector conversations={conversations} onClose={() => setSelectorOpen(false)} onSelect={(target) => void send(target)} />;
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("protocol-send-dialog")} role="dialog" aria-modal="true" aria-label={`Send ${categoryName[payload.category]}`} onMouseDown={(event) => event.stopPropagation()}><header><span /><strong>Share {categoryName[payload.category]}</strong><button type="button" aria-label="Close" onClick={onClose}>×</button></header><ProtocolPayloadPreview payload={payload} /><button className={classes("primary-button protocol-send-action")} type="button" disabled={sending} onClick={() => conversationId ? void send(conversationId) : setSelectorOpen(true)}>{sending ? 'Sending…' : conversationId ? 'Send' : 'Forward'}</button></section></div>;
}

function ProtocolPayloadPreview({ payload }: { payload: ProtocolSendPayload }) {
  if (payload.category === 'image') return <div className={classes("protocol-send-preview image")}><img src={resolveAssetUrl(payload.data)} alt="" /></div>;
  if (payload.category === 'sticker') return <div className={classes("protocol-send-preview")}><span className={classes("protocol-send-sticker")}>◇</span><small>Sticker</small></div>;
  if (payload.category === 'contact') return <div className={classes("protocol-send-preview")}><span className={classes("protocol-send-contact")}>♙</span><small>{payload.data}</small></div>;
  if (payload.category === 'app_card') {
    try {
      const card = JSON.parse(payload.data) as { title?: string; description?: string; icon_url?: string };
      return <div className={classes("protocol-send-preview protocol-send-card")}>{card.icon_url && <img src={resolveAssetUrl(card.icon_url)} alt="" />}<strong>{card.title || 'App Card'}</strong><small>{card.description || ''}</small></div>;
    } catch { return <div className={classes("protocol-send-preview")}><small>App Card</small></div>; }
  }
  return <div className={classes("protocol-send-preview")}><p>{payload.data}</p></div>;
}

type ProtocolTarget = { id: string; userId?: string; name: string; avatarUrl: string; identityNumber: string; isVerified?: boolean; isBot?: boolean; membership?: string | null; section: 'Recent Chats' | 'Contacts' | 'Bots' };

function ProtocolConversationSelector({ conversations, onClose, onSelect }: { conversations: Conversation[]; onClose: () => void; onSelect: (conversationId: string) => void }) {
  const [query, setQuery] = useState('');
  const [users, setUsers] = useState<UserProfile[]>([]);
  const [opening, setOpening] = useState(false);
  useEffect(() => { void desktopApi.selectableUsers().then(setUsers).catch((reason) => { e('Unable to load protocol recipients', reason); setUsers([]); }); }, []);
  const existingOwners = new Set(conversations.map((item) => item.ownerId));
  const allTargets: ProtocolTarget[] = [
    ...conversations.map((item) => ({ id: item.conversationId, name: item.name, avatarUrl: item.avatarUrl, identityNumber: item.identityNumber, isVerified: item.isVerified, isBot: item.isBot, membership: item.membership, section: 'Recent Chats' as const })),
    ...users.filter((user) => !existingOwners.has(user.userId)).map((user) => ({ id: `user:${user.userId}`, userId: user.userId, name: user.fullName, avatarUrl: user.avatarUrl, identityNumber: user.identityNumber, isVerified: user.isVerified, isBot: user.isBot, membership: user.membership, section: user.isBot ? 'Bots' as const : 'Contacts' as const })),
  ];
  const needle = query.trim().toLowerCase();
  const visible = allTargets.filter((item) => !needle || `${item.name} ${item.identityNumber}`.toLowerCase().includes(needle)).slice(0, 200);
  async function select(target: ProtocolTarget) {
    if (opening) return;
    setOpening(true);
    try { onSelect(target.userId ? await desktopApi.openUserConversation(target.userId) : target.id); } catch (error) { showToastFailed(error); setOpening(false); }
  }
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("protocol-conversation-selector")} role="dialog" aria-modal="true" aria-label="Forward" onMouseDown={(event) => event.stopPropagation()}><header><button type="button" aria-label="Close" onClick={onClose}>×</button><strong>Forward</strong><span /></header><label><span>⌕</span><input autoFocus value={query} maxLength={200} onChange={(event) => setQuery(event.target.value)} placeholder="Search" /></label><div className={classes("protocol-target-list")}>{(['Recent Chats', 'Contacts', 'Bots'] as const).map((section) => { const entries = visible.filter((item) => item.section === section); return entries.length ? <section key={section}><h3>{section}</h3>{entries.map((target) => <button type="button" disabled={opening} key={target.id} onClick={() => void select(target)}><Avatar name={target.name} url={target.avatarUrl} size={50} /><span><strong>{target.name}</strong><small>{target.identityNumber}</small></span><Badges verified={Boolean(target.isVerified)} isBot={Boolean(target.isBot)} membership={target.membership} /></button>)}</section> : null; })}{visible.length === 0 && <p>No results</p>}</div></section></div>;
}

export function UnknownMixinUriDialog({ uri, onClose }: { uri: string; onClose: () => void }) {
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("unknown-mixin-uri-dialog")} role="dialog" aria-modal="true" aria-label="Unsupported Mixin URI" onMouseDown={(event) => event.stopPropagation()}><p>This Mixin URI is not supported on desktop. Scan it with your phone.</p><div><QRCodeSVG value={uri} size={224} level="Q" marginSize={0} /></div><button className={classes("primary-button")} type="button" onClick={onClose}>Confirm</button></section></div>;
}

export function ProtocolPaymentDialog({ source, result, onClose }: { source: string; result: CodeResult; onClose: () => void }) {
  const done = ['signed', 'unlocked', 'paid'].includes(result.state?.toLowerCase() ?? '');
  const senders = result.senders.slice(0, 3);
  const receivers = result.receivers.slice(0, 3);
  const avatars = new Map(result.participantAvatars.map((avatar) => [avatar.userId, avatar]));
  const users = (ids: string[]) => <div className={classes("protocol-payment-users")}>{ids.map((id) => { const user = avatars.get(id); return <Avatar key={id} name={user?.name || ''} url={user?.avatarUrl} size={24} />; })}</div>;
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("protocol-payment-dialog")} role="dialog" aria-modal="true" aria-label="Multisig transaction" onMouseDown={(event) => event.stopPropagation()}><button type="button" aria-label="Close" onClick={onClose}>×</button><strong>{result.kind === 'multisig_request' && result.action === 'unlock' ? 'Revoke Multisig Transaction' : 'Multisig Transaction'}</strong><div className={classes("protocol-payment-participants")}>{users(senders)}<span>→</span>{users(receivers)}</div>{result.assetIconUrl && <img src={resolveAssetUrl(result.assetIconUrl)} alt="" />}<p>{result.amount || ''} {result.assetSymbol || ''}</p>{done ? <div className={classes("protocol-payment-done")}>✓</div> : <div className={classes("protocol-payment-qr")}><QRCodeSVG value={source} size={180} level="Q" marginSize={0} /></div>}</section></div>;
}

export type ProtocolOverlaysProps = {
  conversations: Conversation[];
  fiatCurrency: string;
  groupCode?: { code: string; result: CodeResult };
  paymentCode?: { source: string; result: CodeResult };
  protocolSnapshot?: SnapshotDetail;
  protocolSend?: { payload: ProtocolSendPayload; conversationId?: string };
  unknownMixinUri?: string;
  closeGroupCode: () => void;
  closePaymentCode: () => void;
  closeProtocolSnapshot: () => void;
  closeProtocolSend: () => void;
  closeUnknownMixinUri: () => void;
  joinGroup: (conversationId: string) => Promise<void>;
  finishProtocolSend: (conversationId: string) => Promise<void>;
};

export function ProtocolOverlays({
  conversations,
  fiatCurrency,
  groupCode,
  paymentCode,
  protocolSnapshot,
  protocolSend,
  unknownMixinUri,
  closeGroupCode,
  closePaymentCode,
  closeProtocolSnapshot,
  closeProtocolSend,
  closeUnknownMixinUri,
  joinGroup,
  finishProtocolSend,
}: ProtocolOverlaysProps) {
  return (
    <>
      {groupCode && <JoinGroupCodeDialog code={groupCode.code} result={groupCode.result} onClose={closeGroupCode} onJoined={(conversationId) => void joinGroup(conversationId)} />}
      {paymentCode && <ProtocolPaymentDialog source={paymentCode.source} result={paymentCode.result} onClose={closePaymentCode} />}
      {protocolSnapshot && <SnapshotDetailDialog snapshot={protocolSnapshot} fiatCurrency={fiatCurrency} onClose={closeProtocolSnapshot} />}
      {protocolSend && <ProtocolSendDialog conversations={conversations} payload={protocolSend.payload} conversationId={protocolSend.conversationId} onClose={closeProtocolSend} onSent={finishProtocolSend} />}
      {unknownMixinUri && <UnknownMixinUriDialog uri={unknownMixinUri} onClose={closeUnknownMixinUri} />}
    </>
  );
}
