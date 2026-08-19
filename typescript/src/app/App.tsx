import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { RotateCw } from 'lucide-react';
import { events } from '@/generated/bindings';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { openExternal } from '@/shared/lib/openExternal';
import { macosMenuState } from '@/shared/platform/macosMenu';
import { useDesktopUi } from '@/state/desktopUi';
import type { BootstrapData, Profile } from '@/shared/model/types';
import { ToastHost, showToastFailed } from '@/shared/ui/toast';
import { DesktopShell } from '@/app/components/DesktopShell';
import { useAccountEvents } from '@/app/hooks/useAccountEvents';
import { useNotifications } from '@/app/hooks/useNotifications';
import { AccountHealthOverlay, ProfileSetupOverlay } from '@/features/account/components/AccountOverlays';
import { AuthGuard } from '@/features/auth/components/AuthGuard';
import { Login } from '@/features/auth/components/Login';
import { useSecurity } from '@/features/auth/hooks/useSecurity';
import { ConversationInfo } from '@/features/conversations/components/ConversationInfo';
import { MessageSearchPanel } from '@/features/conversations/components/MessageSearchPanel';
import { matchesCategory } from '@/features/conversations/lib/conversationPresentation';
import { ForwardDialog } from '@/features/messages/components/ForwardDialog';
import { ProtocolOverlays } from '@/features/protocol/components/ProtocolDialogs';
import { useProtocolUrls } from '@/features/protocol/hooks/useProtocolUrls';
import { CommandPalette } from '@/features/search/components/CommandPalette';
import { DeviceTransferDialog } from '@/features/transfers/components/DeviceTransferDialog';
import { classes } from '@/shared/styles/classes';

export function App() {
  const queryClient = useQueryClient();
  const bootstrap = useQuery({ queryKey: ['bootstrap'], queryFn: desktopApi.bootstrap });
  const selectedId = useDesktopUi((state) => state.selectedConversationId);
  const selectConversation = useDesktopUi((state) => state.selectConversation);
  const infoOpen = useDesktopUi((state) => state.infoOpen);
  const messageSearchOpen = useDesktopUi((state) => state.messageSearchOpen);
  const closeSettings = useDesktopUi((state) => state.closeSettings);
  const uiSettings = useDesktopUi((state) => state.uiSettings);
  const setUiSettings = useDesktopUi((state) => state.setUiSettings);
  const forwardMessageIds = useDesktopUi((state) => state.forwardMessageIds);
  const forwardCombined = useDesktopUi((state) => state.forwardCombined);
  const closeForward = useDesktopUi((state) => state.closeForward);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const recentConversationIds = useRef<string[]>([]);
  useAccountEvents();

  useEffect(() => {
    const keys = ['brightness', 'messageShowAvatar', 'messageShowIdentityNumber', 'messagePreview', 'chatFontSizeDelta'] as const;
    void Promise.all(keys.map(async (key) => [key, await desktopApi.setting(key)] as const)).then((entries) => {
      const values = Object.fromEntries(entries.map(([key, value]) => [key, value === null ? undefined : JSON.parse(value)]));
      setUiSettings({
        theme: values.brightness === 1 ? 'dark' : values.brightness === 2 ? 'light' : 'system',
        showAvatar: values.messageShowAvatar ?? true,
        showIdentityNumber: values.messageShowIdentityNumber ?? false,
        messagePreview: values.messagePreview ?? true,
        chatFontSizeDelta: Number(values.chatFontSizeDelta ?? 0),
      });
    }).catch((reason) => e('Unable to load UI settings', reason));
  }, [setUiSettings]);

  useEffect(() => {
    if (bootstrap.error) e('Failed to bootstrap application', bootstrap.error);
  }, [bootstrap.error]);

  const data = bootstrap.data;
  const security = useSecurity(data?.profile?.userId ?? 'anonymous');
  const selected = data?.conversations.find((item) => item.conversationId === selectedId);
  const protocol = useProtocolUrls({ signedIn: Boolean(data?.signedIn), conversations: data?.conversations ?? [] });
  const { openProtocolUrl, ...protocolOverlays } = protocol;
  useNotifications(Boolean(data?.signedIn));

  useEffect(() => {
    void desktopApi.updateMacosMenu(macosMenuState(
      data?.signedIn ? selected : undefined,
      security.hasPasscode,
    )).catch((reason) => e('Unable to update macOS menu', reason));
  }, [data?.signedIn, security.hasPasscode, selected]);

  useEffect(() => {
    if (!selectedId) return;
    recentConversationIds.current = [selectedId, ...recentConversationIds.current.filter((id) => id !== selectedId)].slice(0, 5);
  }, [selectedId]);

  useEffect(() => {
    if (!data?.signedIn) return;
    const conversations = data.conversations;
    function onKeyDown(event: globalThis.KeyboardEvent) {
      const modifier = event.metaKey || event.ctrlKey;
      if (!modifier) return;
      const state = useDesktopUi.getState();
      const key = event.key.toLowerCase();
      if (key === 'n') {
        event.preventDefault();
        state.openCreate(event.shiftKey ? 'group' : 'conversation');
        return;
      }
      if (key === 'k') {
        event.preventDefault(); setCommandPaletteOpen(true); return;
      }
      if (key === ',' && event.metaKey) {
        event.preventDefault(); state.openSettings(); return;
      }
      if (key === 'l' && event.metaKey && event.shiftKey) {
        event.preventDefault(); security.lockNow(); return;
      }
      if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp' || state.settingsOpen || !state.selectedConversationId) return;
      const keyword = state.query.trim().toLocaleLowerCase();
      const visible = conversations.filter((item) => matchesCategory(item, state.category, state.circleId)).filter((item) => !state.unseenOnly || item.unseenCount > 0).filter((item) => !keyword || item.name.toLocaleLowerCase().includes(keyword) || item.identityNumber.includes(keyword) || item.lastMessage.toLocaleLowerCase().includes(keyword)).sort((left, right) => {
        if (left.isPinned !== right.isPinned) return left.isPinned ? -1 : 1;
        return (right.pinTimeMillis || right.updatedAtMillis) - (left.pinTimeMillis || left.updatedAtMillis);
      });
      const index = visible.findIndex((item) => item.conversationId === state.selectedConversationId);
      const next = visible[index + (event.key === 'ArrowDown' ? 1 : -1)];
      if (next) { event.preventDefault(); state.selectConversation(next.conversationId); }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [data, security]);

  useEffect(() => {
    if (!data?.signedIn) return;
    let unlisten: (() => void) | undefined;
    void events.appMenuAction.listen(({ payload }) => {
      const state = useDesktopUi.getState();
      const conversation = data.conversations.find((item) => item.conversationId === state.selectedConversationId);
      switch (payload) {
        case 'menu-preferences': state.openSettings(); break;
        case 'menu-lock': security.lockNow(); break;
        case 'menu-quick-search': setCommandPaletteOpen(true); break;
        case 'menu-create-conversation': state.openCreate('conversation'); break;
        case 'menu-create-group': state.openCreate('group'); break;
        case 'menu-create-circle': state.openCreate('circle'); break;
        case 'menu-conversation-mute': if (conversation) state.requestMenuConversationAction('mute'); break;
        case 'menu-conversation-search': if (conversation) state.openMessageSearch(); break;
        case 'menu-conversation-delete': if (conversation) state.requestMenuConversationAction('delete'); break;
        case 'menu-conversation-pin':
          if (conversation) void desktopApi.setConversationPinned(conversation.conversationId, !conversation.isPinned)
            .then(() => queryClient.invalidateQueries({ queryKey: ['bootstrap'] }))
            .catch(showToastFailed);
          break;
        case 'menu-conversation-info': if (conversation) state.toggleInfo(); break;
        case 'menu-previous-conversation':
        case 'menu-next-conversation': {
          if (!conversation || state.settingsOpen) break;
          const keyword = state.query.trim().toLocaleLowerCase();
          const visible = data.conversations
            .filter((item) => matchesCategory(item, state.category, state.circleId))
            .filter((item) => !state.unseenOnly || item.unseenCount > 0)
            .filter((item) => !keyword || item.name.toLocaleLowerCase().includes(keyword) || item.identityNumber.includes(keyword) || item.lastMessage.toLocaleLowerCase().includes(keyword))
            .sort((left, right) => left.isPinned !== right.isPinned ? (left.isPinned ? -1 : 1) : (right.pinTimeMillis || right.updatedAtMillis) - (left.pinTimeMillis || left.updatedAtMillis));
          const index = visible.findIndex((item) => item.conversationId === conversation.conversationId);
          const next = visible[index + (payload === 'menu-next-conversation' ? 1 : -1)];
          if (next) state.selectConversation(next.conversationId);
          break;
        }
        case 'menu-help-center': openExternal('https://support.mixin.one/'); break;
        case 'menu-terms': openExternal('https://mixin.one/pages/terms'); break;
        case 'menu-privacy': openExternal('https://mixin.one/pages/privacy'); break;
      }
    }).then((dispose) => { unlisten = dispose; });
    return () => unlisten?.();
  }, [data, queryClient, security]);

  useEffect(() => {
    if (selectedId && data && !selected) selectConversation();
  }, [data, selectConversation, selected, selectedId]);

  let content: ReactNode;
  if (bootstrap.isPending) {
    content = <LaunchState title="Opening Mixin Messenger" detail="Restoring your local account…" />;
  } else if (bootstrap.isError) {
    content = (
      <LaunchState
        title="Mixin Messenger couldn't open"
        detail={bootstrap.error instanceof Error ? bootstrap.error.message : String(bootstrap.error)}
        action={<button type="button" onClick={() => void bootstrap.refetch()}><RotateCw size={17} /> Retry</button>}
      />
    );
  } else if (!data || !data.signedIn || !data.profile) {
    content = <Login onAuthenticated={(next) => queryClient.setQueryData(['bootstrap'], next)} />;
  } else {
    const signedInData = data as BootstrapData & { profile: Profile };
    content = (
      <>
        <DesktopShell
          data={signedInData}
          security={security}
          onOpenUrl={(url) => void openProtocolUrl(url).catch(showToastFailed)}
          info={selected && infoOpen ? <ConversationInfo conversation={selected} currentUserId={signedInData.profile.userId} /> : undefined}
          search={selected && messageSearchOpen ? <MessageSearchPanel conversationId={selected.conversationId} /> : undefined}
          onProfileUpdated={(profile) => queryClient.setQueryData<BootstrapData>(['bootstrap'], (current) => current ? { ...current, profile } : current)}
          onSignedOut={() => {
            closeSettings();
            queryClient.setQueryData(['bootstrap'], { signedIn: false, conversations: [] });
          }}
        />
        <AuthGuard security={security} />
        <DeviceTransferDialog />
        <ToastHost />
        {data.accountHealth === 'time_inaccurate' && <AccountHealthOverlay health="time_inaccurate" />}
        {data.accountHealth === 'update_required' && <AccountHealthOverlay health="update_required" />}
        {forwardMessageIds.length > 0 && <ForwardDialog conversations={data.conversations} messageIds={forwardMessageIds} combined={forwardCombined} onClose={closeForward} />}
        {commandPaletteOpen && <CommandPalette conversations={data.conversations} recentConversationIds={recentConversationIds.current} onClose={() => setCommandPaletteOpen(false)} onSelect={async (conversationId, userId) => { if (conversationId) selectConversation(conversationId); else if (userId) { const opened = await desktopApi.openUserConversation(userId); await queryClient.invalidateQueries({ queryKey: ['bootstrap'] }); selectConversation(opened); } setCommandPaletteOpen(false); }} />}
        <ProtocolOverlays conversations={data.conversations} fiatCurrency={data.profile.fiatCurrency} {...protocolOverlays} />
        {!data.profile.fullName.trim() && <ProfileSetupOverlay biography={data.profile.biography} onSaved={(profile) => queryClient.setQueryData<BootstrapData>(['bootstrap'], (current) => current?.profile ? { ...current, profile } : current)} />}
      </>
    );
  }

  return (
    <div
      className={classes(`app-root theme-${uiSettings.theme}`)}
      style={{ '--chat-font-delta': `${uiSettings.chatFontSizeDelta}px` } as CSSProperties}
    >
      {content}
    </div>
  );
}

function LaunchState({ title, detail, action }: {
  title: string;
  detail: string;
  action?: ReactNode;
}) {
  return (
    <div className={classes("launch-state")}>
      <div data-tauri-drag-region className={classes("launch-titlebar")} />
      <div className={classes("mixin-mark")}><span /></div>
      <h1>{title}</h1>
      <p>{detail}</p>
      {action}
    </div>
  );
}
