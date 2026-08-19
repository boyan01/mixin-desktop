import { useCallback, useEffect, useEffectEvent, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { events } from '@/generated/bindings';
import { desktopApi } from '@/shared/api/desktopApi';
import { openExternal } from '@/shared/lib/openExternal';
import type { CodeResult, Conversation, SnapshotDetail } from '@/shared/model/types';
import { openBotWebview } from '@/shared/platform/botWebview';
import { useDesktopUi } from '@/state/desktopUi';
import { showToastFailed } from '@/shared/ui/toast';
import { decodeProtocolSendPayload, sendProtocolPayload, type ProtocolSendPayload } from '@/features/protocol/lib/protocolPayload';

export function useProtocolUrls({ signedIn, conversations }: { signedIn: boolean; conversations: Conversation[] }) {
  const queryClient = useQueryClient();
  const selectedConversationId = useDesktopUi((state) => state.selectedConversationId);
  const selectConversation = useDesktopUi((state) => state.selectConversation);
  const [groupCode, setGroupCode] = useState<{ code: string; result: CodeResult }>();
  const [paymentCode, setPaymentCode] = useState<{ source: string; result: CodeResult }>();
  const [protocolSnapshot, setProtocolSnapshot] = useState<SnapshotDetail>();
  const [unknownMixinUri, setUnknownMixinUri] = useState<string>();
  const [protocolSend, setProtocolSend] = useState<{ payload: ProtocolSendPayload; conversationId?: string }>();

  const refreshConversations = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: ['bootstrap'] });
  }, [queryClient]);

  const openProtocolUrl = useCallback(async (source: string) => {
    const uri = new URL(source);
    const isMixin = uri.protocol === 'mixin:' || uri.hostname === 'mixin.one' || uri.hostname === 'www.mixin.one';
    if (!isMixin) { openExternal(source); return; }
    const parts = uri.protocol === 'mixin:' ? [uri.hostname, ...uri.pathname.split('/').filter(Boolean)] : uri.pathname.split('/').filter(Boolean);
    const [kind, value] = parts;
    if (kind === 'users' && value) {
      const conversationId = await desktopApi.openUserConversation(value);
      await refreshConversations();
      selectConversation(conversationId);
      return;
    }
    if (kind === 'conversations' && value) {
      const conversation = conversations.find((item) => item.conversationId === value);
      if (!conversation) return;
      selectConversation(value);
      const start = uri.searchParams.get('start')?.trim();
      if (start) await desktopApi.sendText(value, start);
      return;
    }
    if (kind === 'send') {
      const payload = decodeProtocolSendPayload(uri.searchParams.get('category')?.toLowerCase() ?? null, uri.searchParams.get('data'));
      const targetUserId = uri.searchParams.get('user');
      let targetConversationId = uri.searchParams.get('conversation');
      if (!payload) { setUnknownMixinUri(source); return; }
      if (targetUserId) targetConversationId = await desktopApi.openUserConversation(targetUserId);
      if (targetUserId && targetConversationId) {
        await sendProtocolPayload(targetConversationId, payload);
        await refreshConversations();
        selectConversation(targetConversationId);
        return;
      }
      if (!targetConversationId && payload.category === 'text' && selectedConversationId) {
        await sendProtocolPayload(selectedConversationId, payload);
        await refreshConversations();
        return;
      }
      setProtocolSend({ payload, conversationId: targetConversationId ?? undefined });
      return;
    }
    if (kind === 'apps' && value) {
      if (uri.searchParams.get('action') !== 'open') {
        const conversationId = await desktopApi.openUserConversation(value);
        await refreshConversations();
        selectConversation(conversationId);
        return;
      }
      const [home, profile] = await Promise.all([desktopApi.botHomeUri(value), desktopApi.userProfile(value)]);
      if (!home) throw new Error('Bot not found');
      const homeUri = new URL(home);
      for (const [key, entry] of uri.searchParams) if (key !== 'action') homeUri.searchParams.set(key, entry);
      await openBotWebview({ url: homeUri.toString(), title: profile?.fullName ?? '', conversationId: selectedConversationId ?? '' });
      return;
    }
    if (kind === 'snapshots' && uri.protocol === 'mixin:') {
      const traceId = uri.searchParams.get('trace')?.trim();
      if (!traceId) { setUnknownMixinUri(source); return; }
      setProtocolSnapshot(await desktopApi.snapshotByTrace(traceId));
      return;
    }
    if (kind !== 'codes' || !value) { setUnknownMixinUri(source); return; }
    const result = await desktopApi.resolveCode(value);
    if (result.kind === 'user' && result.userId) {
      const conversationId = await desktopApi.openUserConversation(result.userId);
      await refreshConversations();
      selectConversation(conversationId);
    } else if (result.kind === 'conversation' && result.conversationId) {
      if (result.alreadyMember) selectConversation(result.conversationId);
      else setGroupCode({ code: value, result });
    } else if (result.kind === 'payment' || result.kind === 'multisig_request') {
      setPaymentCode({ source, result });
    }
  }, [conversations, refreshConversations, selectConversation, selectedConversationId]);

  const handleProtocolUrl = useEffectEvent((url: string) => {
    void openProtocolUrl(url).catch(showToastFailed);
  });

  useEffect(() => {
    if (!signedIn) return;
    let unlisten: (() => void) | undefined;
    const onMessageAction = (event: Event) => {
      const url = (event as CustomEvent<string>).detail;
      if (typeof url === 'string') handleProtocolUrl(url);
    };
    void desktopApi.takePendingProtocolUrls().then((urls) => urls.forEach(handleProtocolUrl)).catch(showToastFailed);
    void events.appProtocolUrl.listen((event) => {
      void desktopApi.takePendingProtocolUrls()
        .then((urls) => new Set([...urls, event.payload]).forEach(handleProtocolUrl))
        .catch(showToastFailed);
    }).then((dispose) => { unlisten = dispose; });
    window.addEventListener('mixin://open', onMessageAction);
    return () => {
      unlisten?.();
      window.removeEventListener('mixin://open', onMessageAction);
    };
  }, [signedIn]);

  const joinGroup = useCallback(async (conversationId: string) => {
    await refreshConversations();
    selectConversation(conversationId);
    setGroupCode(undefined);
  }, [refreshConversations, selectConversation]);

  const finishProtocolSend = useCallback(async (conversationId: string) => {
    await refreshConversations();
    selectConversation(conversationId);
    setProtocolSend(undefined);
  }, [refreshConversations, selectConversation]);

  return {
    openProtocolUrl,
    groupCode,
    paymentCode,
    protocolSnapshot,
    protocolSend,
    unknownMixinUri,
    closeGroupCode: () => setGroupCode(undefined),
    closePaymentCode: () => setPaymentCode(undefined),
    closeProtocolSnapshot: () => setProtocolSnapshot(undefined),
    closeProtocolSend: () => setProtocolSend(undefined),
    closeUnknownMixinUri: () => setUnknownMixinUri(undefined),
    joinGroup,
    finishProtocolSend,
  };
}
