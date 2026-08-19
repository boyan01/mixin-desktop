export const PAGE_SIZE = 60;
export const CHAT_TAIL_FOLLOW_THRESHOLD = 50;
export const CHAT_JUMP_TO_LATEST_THRESHOLD = 96;
export const CHAT_MESSAGE_FOCUS_ANCHOR = 0.2;
/** `ChatHistoryViewport` retains two viewport lengths on either side of its center sliver. */
export const CHAT_VIEWPORT_CACHE_EXTENT = 2;
/** `ChatScrollCoordinator` starts loading a page before that retained cache is exhausted. */
export const CHAT_PRELOAD_VIEWPORT_COUNT = 3;
export const CHAT_ESTIMATED_ROW_HEIGHT = 76;

export function chatTailState(distanceToBottom: number) {
  return {
    followTail: distanceToBottom <= CHAT_TAIL_FOLLOW_THRESHOLD,
    showJumpToLatest: distanceToBottom > CHAT_JUMP_TO_LATEST_THRESHOLD,
  };
}

export function preservedPrependScrollTop(scrollTop: number, beforeHeight: number, afterHeight: number) {
  return scrollTop + Math.max(0, afterHeight - beforeHeight);
}

export type CenteredViewportBounds = {
  minScrollOffset: number;
  maxScrollOffset: number;
};

/**
 * The custom Flutter viewport has a real zero at its center sliver.  Its
 * reverse slivers therefore produce negative offsets.  DOM scrollers only
 * expose the equivalent translated interval (`0..scrollHeight-clientHeight`),
 * but keeping this calculation explicit prevents us from treating a centered
 * window as a one-sided list when reasoning about its boundaries.
 */
export function centeredViewportBounds(
  beforeExtent: number,
  afterExtent: number,
  viewportHeight: number,
  anchor = CHAT_MESSAGE_FOCUS_ANCHOR,
): CenteredViewportBounds {
  const before = Math.max(0, beforeExtent);
  const after = Math.max(0, afterExtent);
  const viewport = Math.max(0, viewportHeight);
  // ClampingViewport intentionally permits anchors outside the framework's
  // usual 0..1 range, so retain the supplied finite value here too.
  const viewportAnchor = Number.isFinite(anchor) ? anchor : CHAT_MESSAGE_FOCUS_ANCHOR;
  const top = -before + viewport * viewportAnchor;
  const bottom = after - viewport * (1 - viewportAnchor);
  const maxScrollOffset = Math.max(Math.min(0, top), bottom);
  return {
    minScrollOffset: Math.min(top, maxScrollOffset),
    maxScrollOffset,
  };
}

export type ChatWheelBoundaryPolicy = {
  scrollTop: number;
  preventDefault: boolean;
};

/** Clamp WebKit's transient elastic offsets and consume wheel input at either edge, matching ClampingScrollPhysics. */
export function chatWheelBoundaryPolicy(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  deltaY: number,
): ChatWheelBoundaryPolicy {
  const maxScrollTop = Math.max(0, scrollHeight - clientHeight);
  const normalizedScrollTop = Math.min(maxScrollTop, Math.max(0, Number.isFinite(scrollTop) ? scrollTop : 0));
  return {
    scrollTop: normalizedScrollTop,
    preventDefault: !Number.isFinite(deltaY)
      || (deltaY < 0 && normalizedScrollTop <= 0)
      || (deltaY > 0 && normalizedScrollTop >= maxScrollTop),
  };
}

/**
 * Maps Flutter's `top / center / bottom` sliver model to the single DOM
 * sequence used by react-virtual. The unread separator consumes the center
 * entry, just as `ChatTimelineWindow` moves it into the reverse sliver.
 */
export function centeredTimelineWindow<T extends { messageId: string }>(
  messages: readonly T[],
  centerMessageId?: string | null,
  unreadMessageId?: string | null,
) {
  const centerIndex = centerMessageId == null
    ? -1
    : messages.findIndex((message) => message.messageId === centerMessageId);
  if (centerIndex < 0) {
    return {
      top: messages,
      center: undefined as T | undefined,
      bottom: [] as readonly T[],
      anchorUnreadSeparator: false,
      centerIndex: undefined as number | undefined,
    };
  }
  const anchorUnreadSeparator = centerMessageId === unreadMessageId && centerIndex + 1 < messages.length;
  return anchorUnreadSeparator
    ? {
      top: messages.slice(0, centerIndex + 1),
      center: undefined as T | undefined,
      bottom: messages.slice(centerIndex + 1),
      anchorUnreadSeparator: true,
      centerIndex: centerIndex + 1,
    }
    : {
      top: messages.slice(0, centerIndex),
      center: messages[centerIndex],
      bottom: messages.slice(centerIndex + 1),
      anchorUnreadSeparator: false,
      centerIndex,
    };
}

/** React Virtual's overscan is row-based; convert Flutter's viewport cache extent at the estimate boundary. */
export function viewportCacheOverscanRows(
  viewportHeight: number,
  estimatedRowHeight = CHAT_ESTIMATED_ROW_HEIGHT,
) {
  if (viewportHeight <= 0 || estimatedRowHeight <= 0) return 1;
  return Math.max(1, Math.ceil(viewportHeight * CHAT_VIEWPORT_CACHE_EXTENT / estimatedRowHeight));
}

export function shouldPreloadTimelinePage(distanceToEdge: number, viewportHeight: number) {
  return distanceToEdge <= Math.max(0, viewportHeight) * CHAT_PRELOAD_VIEWPORT_COUNT;
}

export type TimelineVirtualItem = { index: number; start: number; end: number };

/** The first request chooses a stable window anchor; subsequent requests expand that window. */
export type TimelineAnchor =
  | { kind: 'latest' }
  | { kind: 'unread'; messageId: string }
  | { kind: 'message'; messageId: string };

export type TimelinePageParam =
  | { kind: 'latest' }
  | { kind: 'center'; messageId: string }
  | { kind: 'older'; beforeCreatedAtMicros: number; beforeMessageId: string }
  | { kind: 'newer'; messageId: string };

export type TimelinePage<T> = {
  messages: T[];
  hasOlder: boolean;
  hasNewer: boolean;
  /** The DAO returns a stable cursor order; keep the actual boundary instead of recomputing it from display order. */
  olderCursor?: { createdAtMicros: number; messageId: string };
  newerAnchorMessageId?: string;
};

export function timelineAnchorKey(anchor: TimelineAnchor) {
  return anchor.kind === 'latest' ? 'latest' : `${anchor.kind}:${anchor.messageId}`;
}

export function timelineMessages<T extends { createdAtMicros: number; messageId: string }>(pages: readonly TimelinePage<T>[]) {
  const byId = new Map<string, T>();
  for (const page of pages) {
    for (const message of page.messages) byId.set(message.messageId, message);
  }
  return [...byId.values()].sort((left, right) =>
    left.createdAtMicros - right.createdAtMicros || left.messageId.localeCompare(right.messageId),
  );
}

export function timelineOlderParam<T extends { createdAtMicros: number; messageId: string }>(page: TimelinePage<T>) {
  if (!page.hasOlder || !page.olderCursor) return undefined;
  return {
    kind: 'older' as const,
    beforeCreatedAtMicros: page.olderCursor.createdAtMicros,
    beforeMessageId: page.olderCursor.messageId,
  };
}

export function timelineNewerParam<T extends { createdAtMicros: number; messageId: string }>(page: TimelinePage<T>) {
  if (!page.hasNewer || !page.newerAnchorMessageId) return undefined;
  return { kind: 'newer' as const, messageId: page.newerAnchorMessageId };
}

/** Flutter's `messageFocusAnchor`: keep the located row one fifth below the viewport top. */
export function chatFocusScrollOffset(itemStart: number, viewportHeight: number, maxScrollTop = Number.POSITIVE_INFINITY) {
  return Math.max(0, Math.min(maxScrollTop, itemStart - viewportHeight * CHAT_MESSAGE_FOCUS_ANCHOR));
}

/** The virtualizer includes overscan rows; the sticky day must come from the first actually visible row. */
export function firstVisibleTimelineIndex(items: readonly TimelineVirtualItem[], scrollTop: number) {
  return items.find((item) => item.end > scrollTop)?.index;
}

/** Mirrors Flutter's rendered-key lifecycle by dropping identities that left the message window. */
export function renderedTimelineMessageIds(
  messageIds: readonly string[],
  items: readonly TimelineVirtualItem[],
) {
  // A DOM virtual row unregisters as soon as it is recycled, exactly like
  // `ChatRenderedMessage.dispose` removes Flutter's GlobalKey identity.
  const next = new Set<string>();
  for (const item of items) {
    const messageId = messageIds[item.index];
    if (messageId) next.add(messageId);
  }
  return next;
}
