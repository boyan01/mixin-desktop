type ScrollbarAxis = 'horizontal' | 'vertical';

declare global {
  interface Window {
    __MIXIN_SCROLLBAR_INTERACTION__?: boolean;
  }
}

interface ScrollbarState {
  element: HTMLElement;
  axis: ScrollbarAxis;
  track: HTMLDivElement;
  thumb: HTMLDivElement;
  hideTimer?: number;
  dragging?: {
    pointerId: number;
    startPointer: number;
    startScroll: number;
  };
}

const TRACK_SIZE = 14;
const MIN_THUMB_SIZE = 36;
const HIDE_DELAY_MS = 3000;
const statesByElement = new Map<HTMLElement, Map<ScrollbarAxis, ScrollbarState>>();

let resizeObserver: ResizeObserver | undefined;
let scanFrame: number | undefined;

function hasOverflow(element: HTMLElement, axis: ScrollbarAxis) {
  const style = getComputedStyle(element);
  const overflow = axis === 'vertical' ? style.overflowY : style.overflowX;
  if (overflow !== 'auto' && overflow !== 'scroll' && overflow !== 'overlay') return false;
  return axis === 'vertical'
    ? element.scrollHeight > element.clientHeight
    : element.scrollWidth > element.clientWidth;
}

function pointerPosition(event: PointerEvent, axis: ScrollbarAxis) {
  return axis === 'vertical' ? event.clientY : event.clientX;
}

function scrollPosition(element: HTMLElement, axis: ScrollbarAxis) {
  return axis === 'vertical' ? element.scrollTop : element.scrollLeft;
}

function setScrollPosition(element: HTMLElement, axis: ScrollbarAxis, value: number) {
  if (axis === 'vertical') element.scrollTop = value;
  else element.scrollLeft = value;
}

function clearHideTimer(state: ScrollbarState) {
  if (state.hideTimer === undefined) return;
  window.clearTimeout(state.hideTimer);
  state.hideTimer = undefined;
}

function showScrollbar(state: ScrollbarState) {
  clearHideTimer(state);
  state.track.dataset.visible = '';
}

function scheduleHideScrollbar(state: ScrollbarState) {
  clearHideTimer(state);
  state.hideTimer = window.setTimeout(() => {
    state.hideTimer = undefined;
    if (!state.dragging) delete state.track.dataset.visible;
  }, HIDE_DELAY_MS);
}

function updateScrollbar(state: ScrollbarState) {
  const { element, axis, track, thumb } = state;
  if (!hasOverflow(element, axis)) {
    track.style.display = 'none';
    delete track.dataset.visible;
    return;
  }

  const rect = element.getBoundingClientRect();
  const start = axis === 'vertical' ? Math.max(0, rect.top) : Math.max(0, rect.left);
  const end = axis === 'vertical'
    ? Math.min(window.innerHeight, rect.bottom)
    : Math.min(window.innerWidth, rect.right);
  const length = end - start;
  if (length <= TRACK_SIZE) {
    track.style.display = 'none';
    delete track.dataset.visible;
    return;
  }

  track.style.display = 'block';
  if (axis === 'vertical') {
    track.style.left = `${Math.min(window.innerWidth - TRACK_SIZE, rect.right - TRACK_SIZE)}px`;
    track.style.top = `${start}px`;
    track.style.width = `${TRACK_SIZE}px`;
    track.style.height = `${length}px`;
  } else {
    track.style.left = `${Math.max(0, rect.left)}px`;
    track.style.top = `${Math.min(window.innerHeight - TRACK_SIZE, rect.bottom - TRACK_SIZE)}px`;
    track.style.width = `${Math.min(window.innerWidth, rect.right) - Math.max(0, rect.left)}px`;
    track.style.height = `${TRACK_SIZE}px`;
  }

  const viewport = axis === 'vertical' ? element.clientHeight : element.clientWidth;
  const content = axis === 'vertical' ? element.scrollHeight : element.scrollWidth;
  const position = scrollPosition(element, axis);
  const maxScroll = Math.max(0, content - viewport);
  const thumbLength = Math.min(length, Math.max(MIN_THUMB_SIZE, length * viewport / content));
  const maxThumbOffset = Math.max(0, length - thumbLength);
  const thumbOffset = maxScroll === 0 ? 0 : (position / maxScroll) * maxThumbOffset;

  if (axis === 'vertical') {
    thumb.style.top = `${thumbOffset}px`;
    thumb.style.height = `${thumbLength}px`;
  } else {
    thumb.style.left = `${thumbOffset}px`;
    thumb.style.width = `${thumbLength}px`;
  }
}

function startThumbDrag(state: ScrollbarState, event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  showScrollbar(state);
  state.dragging = {
    pointerId: event.pointerId,
    startPointer: pointerPosition(event, state.axis),
    startScroll: scrollPosition(state.element, state.axis),
  };
  state.thumb.setPointerCapture(event.pointerId);
  state.track.dataset.dragging = '';
}

function moveThumbDrag(state: ScrollbarState, event: PointerEvent) {
  const dragging = state.dragging;
  if (!dragging || dragging.pointerId !== event.pointerId) return;
  const trackLength = state.axis === 'vertical' ? state.track.clientHeight : state.track.clientWidth;
  const thumbLength = state.axis === 'vertical' ? state.thumb.offsetHeight : state.thumb.offsetWidth;
  const maxThumbOffset = Math.max(1, trackLength - thumbLength);
  const viewport = state.axis === 'vertical' ? state.element.clientHeight : state.element.clientWidth;
  const content = state.axis === 'vertical' ? state.element.scrollHeight : state.element.scrollWidth;
  const maxScroll = Math.max(0, content - viewport);
  const delta = pointerPosition(event, state.axis) - dragging.startPointer;
  const nextPosition = dragging.startScroll + (delta / maxThumbOffset) * maxScroll;
  setScrollPosition(state.element, state.axis, Math.max(0, Math.min(maxScroll, nextPosition)));
}

function finishThumbDrag(state: ScrollbarState, event: PointerEvent) {
  if (!state.dragging || state.dragging.pointerId !== event.pointerId) return;
  if (state.thumb.hasPointerCapture(event.pointerId)) state.thumb.releasePointerCapture(event.pointerId);
  state.dragging = undefined;
  delete state.track.dataset.dragging;
  scheduleHideScrollbar(state);
}

function pageScrollFromTrack(state: ScrollbarState, event: PointerEvent) {
  if (event.button !== 0 || event.target !== state.track) return;
  event.preventDefault();
  showScrollbar(state);
  const thumbRect = state.thumb.getBoundingClientRect();
  const pointer = pointerPosition(event, state.axis);
  const thumbCenter = state.axis === 'vertical'
    ? thumbRect.top + thumbRect.height / 2
    : thumbRect.left + thumbRect.width / 2;
  const viewport = state.axis === 'vertical' ? state.element.clientHeight : state.element.clientWidth;
  const direction = pointer < thumbCenter ? -1 : 1;
  if (state.axis === 'vertical') state.element.scrollBy({ top: direction * viewport, behavior: 'smooth' });
  else state.element.scrollBy({ left: direction * viewport, behavior: 'smooth' });
}

function createScrollbarState(element: HTMLElement, axis: ScrollbarAxis): ScrollbarState {
  const track = document.createElement('div');
  track.dataset.mixinScrollbarTrack = '';
  track.dataset.axis = axis;
  const thumb = document.createElement('div');
  thumb.dataset.mixinScrollbarThumb = '';
  track.appendChild(thumb);
  document.body.appendChild(track);

  const state = { element, axis, track, thumb };
  track.addEventListener('pointerenter', () => showScrollbar(state));
  track.addEventListener('pointerleave', () => scheduleHideScrollbar(state));
  track.addEventListener('pointerdown', (event) => pageScrollFromTrack(state, event));
  thumb.addEventListener('pointerdown', (event) => startThumbDrag(state, event));
  thumb.addEventListener('pointermove', (event) => moveThumbDrag(state, event));
  thumb.addEventListener('pointerup', (event) => finishThumbDrag(state, event));
  thumb.addEventListener('pointercancel', (event) => finishThumbDrag(state, event));
  resizeObserver?.observe(element);
  return state;
}

function removeScrollbarState(state: ScrollbarState) {
  clearHideTimer(state);
  resizeObserver?.unobserve(state.element);
  state.track.remove();
}

function queueScan() {
  if (scanFrame !== undefined) return;
  scanFrame = window.requestAnimationFrame(() => {
    scanFrame = undefined;
    scanScrollContainers();
  });
}

function scanScrollContainers() {
  if (!document.body) return;
  const elements = [document.documentElement, ...document.body.querySelectorAll<HTMLElement>('*')];
  const seen = new Set<HTMLElement>();
  for (const element of elements) {
    if (element.hasAttribute('data-mixin-scrollbar-track')) continue;
    const activeAxes = new Set<ScrollbarAxis>();
    for (const axis of ['vertical', 'horizontal'] as const) {
      if (!hasOverflow(element, axis)) continue;
      activeAxes.add(axis);
      let states = statesByElement.get(element);
      if (!states) {
        states = new Map();
        statesByElement.set(element, states);
      }
      const state = states.get(axis) ?? createScrollbarState(element, axis);
      states.set(axis, state);
      updateScrollbar(state);
    }
    const states = statesByElement.get(element);
    if (!states) continue;
    seen.add(element);
    for (const [axis, state] of states) {
      if (activeAxes.has(axis)) continue;
      removeScrollbarState(state);
      states.delete(axis);
    }
    if (states.size === 0) statesByElement.delete(element);
  }

  for (const [element, states] of statesByElement) {
    if (seen.has(element) && element.isConnected) continue;
    for (const state of states.values()) removeScrollbarState(state);
    statesByElement.delete(element);
  }
}

function refreshScrollbars() {
  for (const states of statesByElement.values()) {
    for (const state of states.values()) updateScrollbar(state);
  }
}

function handleScroll(event: Event) {
  refreshScrollbars();
  if (!(event.target instanceof HTMLElement)) return;
  const states = statesByElement.get(event.target);
  if (!states) return;
  for (const state of states.values()) {
    showScrollbar(state);
    scheduleHideScrollbar(state);
  }
}

function handleMutations(records: MutationRecord[]) {
  if (records.some((record) => {
    const target = record.target;
    return !(target instanceof HTMLElement && target.closest('[data-mixin-scrollbar-track]'));
  })) queueScan();
}

export function installScrollbarInteraction() {
  if (!document.body || window.__MIXIN_SCROLLBAR_INTERACTION__) return;
  window.__MIXIN_SCROLLBAR_INTERACTION__ = true;
  resizeObserver = new ResizeObserver(refreshScrollbars);
  document.addEventListener('scroll', handleScroll, { capture: true, passive: true });
  window.addEventListener('resize', refreshScrollbars, { passive: true });
  new MutationObserver(handleMutations).observe(document.body, {
    attributes: true,
    attributeFilter: ['class', 'style'],
    childList: true,
    subtree: true,
  });
  queueScan();
}
