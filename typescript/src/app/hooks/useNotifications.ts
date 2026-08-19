import { useEffect, useRef } from 'react';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { events } from '@/generated/bindings';
import { e } from '@/shared/lib/logger';
import type { NotificationEvent } from '@/shared/model/types';
import { useDesktopUi } from '@/state/desktopUi';

export function useNotifications(signedIn: boolean) {
  const selectedConversationId = useDesktopUi((state) => state.selectedConversationId);
  const messagePreview = useDesktopUi((state) => state.uiSettings.messagePreview);
  const selectedConversationIdRef = useRef(selectedConversationId);
  const messagePreviewRef = useRef(messagePreview);
  const nextNotificationId = useRef(0);
  selectedConversationIdRef.current = selectedConversationId;
  messagePreviewRef.current = messagePreview;

  useEffect(() => {
    if (!signedIn) return;
    let active = true;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        let granted = await isPermissionGranted();
        if (!granted) granted = (await requestPermission()) === 'granted';
        if (!active || !granted) return;
        const dispose = await events.accountNotification.listen(({ payload }) => {
          if (payload.dismissMessageId) return;
          if (payload.conversationId === selectedConversationIdRef.current) return;
          if (Date.now() - payload.createdAtMicros / 1000 > 120_000) return;
          const text = messagePreviewRef.current ? notificationPreview(payload) : 'A message';
          nextNotificationId.current = (nextNotificationId.current + 1) & 0x7fffffff;
          sendNotification({
            id: nextNotificationId.current,
            title: payload.conversationName,
            body: text,
            group: payload.conversationId,
            autoCancel: true,
          });
        });
        if (active) unlisten = dispose;
        else dispose();
      } catch (error) {
        if (active) e('Unable to initialize desktop notifications', error);
      }
    })();
    return () => { active = false; unlisten?.(); };
  }, [signedIn]);
}

function notificationPreview(event: NotificationEvent) {
  if (event.category.endsWith('_TEXT') || event.category.endsWith('_POST')) return event.content;
  if (event.category.includes('IMAGE')) return 'Photo';
  if (event.category.includes('VIDEO')) return 'Video';
  if (event.category.includes('AUDIO')) return 'Audio';
  if (event.category.includes('CONTACT')) return 'Contact';
  if (event.category.includes('STICKER')) return 'Sticker';
  return 'A message';
}
