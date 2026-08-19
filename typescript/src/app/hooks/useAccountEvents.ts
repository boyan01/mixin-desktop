import { useEffect } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { events } from '@/generated/bindings';
import { desktopApi } from '@/shared/api/desktopApi';
import { useDesktopUi } from '@/state/desktopUi';
import type { BootstrapData } from '@/shared/model/types';

export function useAccountEvents(): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    const listeners: Promise<UnlistenFn>[] = [];
    listeners.push(
      events.accountConversationChange.listen(async ({ payload }) => {
        if (payload.reloadAll) {
          await queryClient.invalidateQueries({ queryKey: ['bootstrap'] });
          return;
        }
        if (payload.conversationIds.length === 0) return;
        const changed = await desktopApi.conversationsByIds(payload.conversationIds);
        queryClient.setQueryData<BootstrapData>(['bootstrap'], (current) => {
          if (!current) return current;
          const byId = new Map(current.conversations.map((item) => [item.conversationId, item]));
          for (const conversation of changed) byId.set(conversation.conversationId, conversation);
          for (const id of payload.conversationIds) {
            if (!changed.some((conversation) => conversation.conversationId === id)) byId.delete(id);
          }
          return { ...current, conversations: [...byId.values()] };
        });
      }),
    );
    listeners.push(
      events.accountCircleChange.listen(({ payload }) => {
        queryClient.setQueryData(['circles'], payload);
      }),
    );
    listeners.push(
      events.accountMessageChange.listen(async () => {
        const selected = useDesktopUi.getState().selectedConversationId;
        if (selected) {
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['messages', selected] }),
            queryClient.invalidateQueries({ queryKey: ['pinned-message-ids', selected] }),
            queryClient.invalidateQueries({ queryKey: ['pin-message-preview', selected] }),
            queryClient.invalidateQueries({ queryKey: ['unread-mention-message-ids', selected] }),
          ]);
        }
      }),
    );
    listeners.push(
      events.accountProfileChange.listen(({ payload }) => {
        queryClient.setQueryData<BootstrapData>(['bootstrap'], (current) =>
          current ? { ...current, profile: payload } : current,
        );
      }),
    );
    listeners.push(
      events.accountHealthChange.listen(({ payload }) => {
        queryClient.setQueryData<BootstrapData>(['bootstrap'], (current) =>
          current ? { ...current, accountHealth: payload } : current,
        );
      }),
    );
    listeners.push(
      events.accountConnectionChange.listen(({ payload }) => {
        queryClient.setQueryData<BootstrapData>(['bootstrap'], (current) =>
          current ? { ...current, connectionStatus: payload } : current,
        );
      }),
    );

    return () => {
      for (const listener of listeners) void listener.then((unlisten) => unlisten());
    };
  }, [queryClient]);
}
