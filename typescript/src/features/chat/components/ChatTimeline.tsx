import {
  useCallback,
  useEffect,
  useEffectEvent,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type WheelEvent,
} from 'react';
import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
  type InfiniteData,
} from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { desktopApi } from '@/shared/api/desktopApi';
import { useDesktopUi } from '@/state/desktopUi';
import type { Conversation, Message, UserProfile } from '@/shared/model/types';
import { UserProfileDialog } from '@/features/profile/components/UserProfileDialog';
import { MessageRow } from '@/features/chat/components/MessageRow';
import {
  PAGE_SIZE,
  centeredTimelineWindow,
  chatFocusScrollOffset,
  chatTailState,
  chatWheelBoundaryPolicy,
  firstVisibleTimelineIndex,
  renderedTimelineMessageIds,
  shouldPreloadTimelinePage,
  timelineAnchorKey,
  timelineMessages,
  timelineNewerParam,
  timelineOlderParam,
  viewportCacheOverscanRows,
  type TimelineAnchor,
  type TimelinePage,
  type TimelinePageParam,
} from '@/features/chat/model/timeline';
import { formatDayTime, sameLocalDay } from '@/features/chat/model/message';
import { classes } from '@/shared/styles/classes';

const EMPTY_MENTION_NAMES: Record<string, string> = {};

function initialTimelineAnchor(conversation: Conversation): TimelineAnchor {
  return conversation.unseenCount > 0 && conversation.lastReadMessageId
    ? { kind: 'unread', messageId: conversation.lastReadMessageId }
    : { kind: 'latest' };
}

function sortMessages(messages: Message[]) {
  return [...messages].sort((left, right) =>
    left.createdAtMicros - right.createdAtMicros || left.messageId.localeCompare(right.messageId),
  );
}

function messageCursor(message: Message | undefined) {
  return message == null
    ? undefined
    : { createdAtMicros: message.createdAtMicros, messageId: message.messageId };
}

function centeredPage(messages: Message[], targetMessageId: string, before: number, after: number): TimelinePage<Message> {
  const sorted = sortMessages(messages);
  const targetIndex = messages.findIndex((message) => message.messageId === targetMessageId);
  if (targetIndex < 0) return { messages: [], hasOlder: false, hasNewer: false };
  return {
    messages: sorted,
    hasOlder: targetIndex >= before,
    hasNewer: messages.length - targetIndex - 1 >= after,
    olderCursor: targetIndex >= before ? messageCursor(messages[0]) : undefined,
    newerAnchorMessageId: messages.length - targetIndex - 1 >= after
      ? messages[messages.length - 1]?.messageId
      : undefined,
  };
}

async function loadTimelinePage(conversationId: string, pageParam: TimelinePageParam): Promise<TimelinePage<Message>> {
  if (pageParam.kind === 'latest') {
    const messages = await desktopApi.messages(conversationId, undefined, undefined, PAGE_SIZE);
    return {
      messages: sortMessages(messages),
      hasOlder: messages.length >= PAGE_SIZE,
      hasNewer: false,
      olderCursor: messageCursor(messages[messages.length - 1]),
    };
  }
  if (pageParam.kind === 'older') {
    const messages = await desktopApi.messages(
      conversationId,
      pageParam.beforeCreatedAtMicros,
      pageParam.beforeMessageId,
      PAGE_SIZE,
    );
    return {
      messages: sortMessages(messages),
      hasOlder: messages.length >= PAGE_SIZE,
      hasNewer: false,
      olderCursor: messageCursor(messages[messages.length - 1]),
    };
  }
  if (pageParam.kind === 'newer') {
    const messages = await desktopApi.messagesAround(conversationId, pageParam.messageId, 0, PAGE_SIZE);
    const page = centeredPage(messages, pageParam.messageId, 0, PAGE_SIZE);
    return {
      ...page,
      hasOlder: false,
      olderCursor: undefined,
      messages: page.messages.filter((message) => message.messageId !== pageParam.messageId),
    };
  }
  const halfPageSize = Math.floor(PAGE_SIZE / 2);
  return centeredPage(
    await desktopApi.messagesAround(conversationId, pageParam.messageId, halfPageSize, halfPageSize),
    pageParam.messageId,
    halfPageSize,
    halfPageSize,
  );
}

export function ChatTimeline({ conversation, currentUserId, renderOverlays }: {
  conversation: Conversation;
  currentUserId: string;
  renderOverlays: (controls: { showJumpToLatest: boolean; jumpToLatest: () => void }) => ReactNode;
}) {
  const conversationId = conversation.conversationId;
  const scrollRef = useRef<HTMLDivElement>(null);
  const initializedConversation = useRef<string | null>(null);
  const previousMessageIds = useRef<string[]>([]);
  const followTailRef = useRef(true);
  const windowResetRef = useRef(false);
  const loadingOlderRef = useRef(false);
  const loadingNewerRef = useRef(false);
  const renderedMessageIdsRef = useRef<Set<string>>(new Set());
  const focusRequestRef = useRef(0);
  const focusClearTimerRef = useRef<number | undefined>(undefined);
  const scrollUiFrameRef = useRef<number | undefined>(undefined);
  const [showJumpToLatest, setShowJumpToLatest] = useState(false);
  const [stickyDayMicros, setStickyDayMicros] = useState<number>();
  const [profileDialog, setProfileDialog] = useState<UserProfile>();
  const queryClient = useQueryClient();
  const focusMessageId = useDesktopUi((state) => state.focusMessageId);
  const clearFocusMessage = useDesktopUi((state) => state.clearFocusMessage);
  const [storedWindow, setStoredWindow] = useState<{ conversationId: string; anchor: TimelineAnchor }>(() => ({
    conversationId,
    anchor: initialTimelineAnchor(conversation),
  }));
  const conversationAnchor = useMemo(() => initialTimelineAnchor(conversation), [
    conversation,
  ]);
  const storedAnchor = storedWindow.conversationId === conversationId
    ? storedWindow.anchor
    : conversationAnchor;
  const activeAnchor = useMemo<TimelineAnchor>(() => focusMessageId
    ? { kind: 'message', messageId: focusMessageId }
    : storedAnchor, [focusMessageId, storedAnchor]);
  const activeAnchorKey = timelineAnchorKey(activeAnchor);
  const query = useInfiniteQuery<TimelinePage<Message>, Error, InfiniteData<TimelinePage<Message>, TimelinePageParam>, readonly ['messages', string, string], TimelinePageParam>({
    queryKey: ['messages', conversationId, activeAnchorKey] as const,
    initialPageParam: activeAnchor.kind === 'latest'
      ? { kind: 'latest' }
      : { kind: 'center', messageId: activeAnchor.messageId },
    queryFn: ({ pageParam }) => loadTimelinePage(conversationId, pageParam),
    getPreviousPageParam: (firstPage) => timelineOlderParam(firstPage),
    getNextPageParam: (lastPage) => timelineNewerParam(lastPage),
  });
  const participantQuery = useQuery({
    queryKey: ['conversation-participants', conversationId],
    queryFn: () => desktopApi.conversationParticipants(conversationId),
    enabled: conversation.category === 'GROUP',
  });
  const currentUserRole = participantQuery.data?.find((participant) => participant.userId === currentUserId)?.role;
  const messages = useMemo(() => timelineMessages(query.data?.pages ?? []), [query.data]);
  const timelineWindow = useMemo(
    () => centeredTimelineWindow(
      messages,
      activeAnchor.kind === 'latest' ? undefined : activeAnchor.messageId,
      activeAnchor.kind === 'unread' ? activeAnchor.messageId : undefined,
    ),
    [activeAnchor, messages],
  );
  const mentionContents = useMemo(() => messages
    .flatMap((message) => [message.content, message.caption].filter((value): value is string => Boolean(value)))
    .filter((content) => /@\d{4,}/.test(content)), [messages]);
  const mentionNames = useQuery({
    queryKey: ['mention-names', conversationId, mentionContents],
    queryFn: () => desktopApi.mentionNames(mentionContents),
    enabled: mentionContents.length > 0,
  });
  const mentionNameMap = mentionNames.data ?? EMPTY_MENTION_NAMES;
  const avatarsEnabled = useDesktopUi((state) => state.uiSettings.showAvatar);
  const [viewportHeight, setViewportHeight] = useState(0);
  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => scrollRef.current,
    getItemKey: useCallback((index: number) => messages[index]?.messageId ?? index, [messages]),
    estimateSize: useCallback(() => 76, []),
    // Flutter's `ScrollCacheExtent.viewport(2)` is measured from each side
    // of the centered sliver. TanStack accepts rows, so convert at the same
    // 76px estimate it uses before measurement.
    overscan: viewportCacheOverscanRows(viewportHeight),
    // Preserve the visible item when either edge of the window changes. This
    // is the single prepend/append anchor owner; do not also compensate with
    // scrollHeight deltas in the component.
    anchorTo: 'end',
    followOnAppend: false,
    useFlushSync: false,
  });
  const virtualItems = virtualizer.getVirtualItems();
  renderedMessageIdsRef.current = renderedTimelineMessageIds(
    messages.map((message) => message.messageId),
    virtualItems,
  );

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const update = () => setViewportHeight(element.clientHeight);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [conversationId]);

  function updateScrollUi() {
    const element = scrollRef.current;
    if (!element) return;
    const distanceToBottom = element.scrollHeight - element.clientHeight - element.scrollTop;
    const nextTailState = chatTailState(distanceToBottom);
    setShowJumpToLatest((current) => current === nextTailState.showJumpToLatest ? current : nextTailState.showJumpToLatest);
    const index = firstVisibleTimelineIndex(virtualizer.getVirtualItems(), element.scrollTop);
    const nextDayMicros = index === undefined ? undefined : messages[index]?.createdAtMicros;
    setStickyDayMicros((current) => current === nextDayMicros ? current : nextDayMicros);
  }

  function scheduleScrollUiUpdate() {
    if (scrollUiFrameRef.current !== undefined) return;
    scrollUiFrameRef.current = requestAnimationFrame(() => {
      scrollUiFrameRef.current = undefined;
      updateScrollUi();
    });
  }

  const scheduleScrollUiUpdateAfterRender = useEffectEvent(scheduleScrollUiUpdate);

  function loadOlder() {
    if (!query.hasPreviousPage || query.isFetchingPreviousPage || loadingOlderRef.current) return;
    loadingOlderRef.current = true;
    void query.fetchPreviousPage().finally(() => { loadingOlderRef.current = false; });
  }

  function loadNewer() {
    if (!query.hasNextPage || query.isFetchingNextPage || loadingNewerRef.current) return;
    loadingNewerRef.current = true;
    void query.fetchNextPage().finally(() => { loadingNewerRef.current = false; });
  }

  useEffect(() => {
    if (storedWindow.conversationId === conversationId) return;
    setStoredWindow({ conversationId, anchor: conversationAnchor });
    windowResetRef.current = true;
    loadingOlderRef.current = false;
    loadingNewerRef.current = false;
  }, [conversationAnchor, conversationId, storedWindow.conversationId]);

  useEffect(() => {
    windowResetRef.current = true;
  }, [activeAnchorKey]);

  useEffect(() => {
    if (messages.length === 0 || initializedConversation.current === conversationId) return;
    initializedConversation.current = conversationId;
    requestAnimationFrame(() => {
      const anchorIndex = timelineWindow.centerIndex;
      if (anchorIndex === undefined) {
        virtualizer.scrollToEnd({ behavior: 'instant' });
        return;
      }
      // The first pass makes the row measurable. The second applies the same
      // one-fifth viewport anchor as the centered Flutter viewport.
      virtualizer.scrollToIndex(anchorIndex, { align: 'start', behavior: 'instant' });
      requestAnimationFrame(() => {
        const element = scrollRef.current;
        const item = virtualizer.getVirtualItems().find((candidate) => candidate.index === anchorIndex);
        if (!element || !item || initializedConversation.current !== conversationId) return;
        virtualizer.scrollToOffset(chatFocusScrollOffset(
          item.start,
          element.clientHeight,
          Math.max(0, element.scrollHeight - element.clientHeight),
        ), { behavior: 'instant' });
      });
    });
  }, [conversationId, messages.length, timelineWindow.centerIndex, virtualizer]);

  useEffect(() => {
    initializedConversation.current = null;
    focusRequestRef.current += 1;
    if (focusClearTimerRef.current !== undefined) window.clearTimeout(focusClearTimerRef.current);
    focusClearTimerRef.current = undefined;
    renderedMessageIdsRef.current.clear();
    setShowJumpToLatest(false);
    setStickyDayMicros(undefined);
  }, [conversationId]);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    if (windowResetRef.current) {
      windowResetRef.current = false;
    } else if (previousMessageIds.current.length > 0 && followTailRef.current) {
      virtualizer.scrollToEnd({ behavior: 'instant' });
    }
    previousMessageIds.current = messages.map((message) => message.messageId);
  }, [messages, virtualizer]);

  useEffect(() => {
    scheduleScrollUiUpdateAfterRender();
  }, [messages, virtualizer]);

  useEffect(() => () => {
    if (scrollUiFrameRef.current !== undefined) cancelAnimationFrame(scrollUiFrameRef.current);
  }, []);

  function jumpToLatest() {
    virtualizer.scrollToEnd({ behavior: 'smooth' });
    setShowJumpToLatest(false);
  }

  const openUserProfile = useCallback((userId: string) => {
    void desktopApi.userProfile(userId).then((profile) => setProfileDialog(profile ?? undefined));
  }, []);

  async function openProfileChat() {
    if (!profileDialog) return;
    const conversationId = await desktopApi.openUserConversation(profileDialog.userId);
    await queryClient.invalidateQueries({ queryKey: ['bootstrap'] });
    useDesktopUi.getState().selectConversation(conversationId);
    setProfileDialog(undefined);
  }

  async function openProfileInfo() {
    await openProfileChat();
    useDesktopUi.getState().toggleInfo();
  }

  useEffect(() => {
    if (!focusMessageId) return;
    const requestId = ++focusRequestRef.current;
    const index = messages.findIndex((message) => message.messageId === focusMessageId);
    if (index < 0) return;
    let cancelled = false;
    setStoredWindow((current) => current.conversationId === conversationId
      && current.anchor.kind === 'message'
      && current.anchor.messageId === focusMessageId
      ? current
      : { conversationId, anchor: { kind: 'message', messageId: focusMessageId } });
    requestAnimationFrame(() => {
      if (cancelled || requestId !== focusRequestRef.current || useDesktopUi.getState().focusMessageId !== focusMessageId) return;
      const element = scrollRef.current;
      if (!element) return;
      // The first instant pass makes a dynamically measured row available; the
      // second pass applies Flutter's 0.2 viewport focus anchor.
      virtualizer.scrollToIndex(index, { align: 'start', behavior: 'instant' });
      requestAnimationFrame(() => {
        if (cancelled || requestId !== focusRequestRef.current) return;
        const item = virtualizer.getVirtualItems().find((candidate) => candidate.index === index);
        if (!item) return;
        virtualizer.scrollToOffset(chatFocusScrollOffset(
          item.start,
          element.clientHeight,
          Math.max(0, element.scrollHeight - element.clientHeight),
        ), { behavior: 'smooth' });
      });
      if (focusClearTimerRef.current !== undefined) window.clearTimeout(focusClearTimerRef.current);
      focusClearTimerRef.current = window.setTimeout(() => {
        if (!cancelled && requestId === focusRequestRef.current) clearFocusMessage(focusMessageId);
      }, 1200);
    });
    return () => {
      cancelled = true;
      if (focusClearTimerRef.current !== undefined) window.clearTimeout(focusClearTimerRef.current);
      focusClearTimerRef.current = undefined;
    };
  }, [clearFocusMessage, conversationId, focusMessageId, messages, virtualizer]);

  if (query.isPending) return <div className={classes("chat-timeline")}><div className={classes("chat-state")}>Loading messages…</div>{renderOverlays({ showJumpToLatest: false, jumpToLatest })}</div>;
  if (query.isError) return <div className={classes("chat-timeline")}><div className={classes("chat-state is-error")}>Unable to load messages</div>{renderOverlays({ showJumpToLatest: false, jumpToLatest })}</div>;

  return (
    <div className={classes("chat-timeline")}>
      {(query.isFetchingPreviousPage || query.isFetchingNextPage) && <div className={classes("history-loader")}>Loading history…</div>}
      <div
        className={classes("chat-scroll")}
        ref={scrollRef}
        onWheel={(event: WheelEvent<HTMLDivElement>) => {
          const element = event.currentTarget;
          const policy = chatWheelBoundaryPolicy(
            element.scrollTop,
            element.scrollHeight,
            element.clientHeight,
            event.deltaY,
          );
          if (element.scrollTop !== policy.scrollTop) element.scrollTop = policy.scrollTop;
          if (policy.preventDefault) event.preventDefault();
        }}
        onScroll={(event) => {
          const element = event.currentTarget;
          const policy = chatWheelBoundaryPolicy(
            element.scrollTop,
            element.scrollHeight,
            element.clientHeight,
            0,
          );
          // Safari can report elastic negative/overflow values during a fling.
          // Correct before calculating tail and preload distances.
          if (element.scrollTop !== policy.scrollTop) element.scrollTop = policy.scrollTop;
          const distanceToBottom = element.scrollHeight - element.clientHeight - element.scrollTop;
          const tailState = chatTailState(distanceToBottom);
          followTailRef.current = tailState.followTail;
          scheduleScrollUiUpdate();
          if (shouldPreloadTimelinePage(element.scrollTop, element.clientHeight)) loadOlder();
          if (shouldPreloadTimelinePage(distanceToBottom, element.clientHeight)) loadNewer();
        }}
      >
        <div className={classes("virtual-content")} style={{ height: virtualizer.getTotalSize() }}>
          {virtualItems.map((item) => {
            const message = messages[item.index];
            const previous = messages[item.index - 1];
            const next = messages[item.index + 1];
            const showAvatar = avatarsEnabled;
            const showDayTime = !previous || !sameLocalDay(previous.createdAtMicros, message.createdAtMicros);
            const showUnread = timelineWindow.anchorUnreadSeparator
              && item.index === timelineWindow.centerIndex;
            return (
              <div
                key={item.key}
                data-index={item.index}
                ref={virtualizer.measureElement}
                className={classes("virtual-row")}
                data-message-id={message.messageId}
                style={{ transform: `translateY(${item.start}px)` }}
              >
                {showUnread && <div className={classes("unread-message-bar")}>Unread Messages</div>}
                {showDayTime && <div className={classes("message-day-time")}>{formatDayTime(message.createdAtMicros)}</div>}
                <MessageRow message={message} previous={previous} next={next} currentUserId={currentUserId} currentUserRole={currentUserRole} showAvatar={showAvatar} conversation={conversation} mentionNames={mentionNameMap} audioPlaylist={messages} imagePlaylist={messages} onOpenUser={openUserProfile} />
              </div>
            );
          })}
        </div>
        {messages.length === 0 && <div className={classes("chat-state")}>No messages yet</div>}
      </div>
      {stickyDayMicros != null && <div className={classes("message-day-time message-day-time--sticky")}>{formatDayTime(stickyDayMicros)}</div>}
      {renderOverlays({ showJumpToLatest, jumpToLatest })}
      {profileDialog && <UserProfileDialog profile={profileDialog} currentUserId={currentUserId} onClose={() => setProfileDialog(undefined)} onChanged={async () => { await queryClient.invalidateQueries({ queryKey: ['bootstrap'] }); setProfileDialog(await desktopApi.userProfile(profileDialog.userId) ?? undefined); }} onChat={openProfileChat} onInfo={openProfileInfo} />}
    </div>
  );
}
