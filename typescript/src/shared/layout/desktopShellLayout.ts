export const SLIDE_PAGE_MIN_WIDTH = 64;
export const SLIDE_PAGE_MAX_WIDTH = 176;
export const RESPONSIVE_NAVIGATION_MIN_WIDTH = 320;
export const CONVERSATION_LIST_WIDTH = 300;
export const MAIN_ROUTE_SWITCH_WIDTH =
  RESPONSIVE_NAVIGATION_MIN_WIDTH + CONVERSATION_LIST_WIDTH;

export type DesktopShellLayoutMode = 'drawer' | 'compactRail' | 'fullRail';

export interface DesktopShellLayout {
  mode: DesktopShellLayoutMode;
  slideWidth: number;
  availableSlideWidth: number;
  autoCollapse: boolean;
  userCollapse: boolean;
  routeMode: boolean;
  sideRouteMode: boolean;
}

/**
 * Keep the responsive shell thresholds in one place. These values mirror
 * Flutter's DesktopShellLayout.resolve so CSS and React state use the same
 * layout decision.
 */
export function resolveDesktopShellLayout(
  maxWidth: number,
  userCollapse: boolean,
): DesktopShellLayout {
  const width = Math.max(0, maxWidth);
  const availableSlideWidth = Math.min(
    Math.max(width - RESPONSIVE_NAVIGATION_MIN_WIDTH, SLIDE_PAGE_MIN_WIDTH),
    SLIDE_PAGE_MAX_WIDTH,
  );
  const autoCollapse = availableSlideWidth < SLIDE_PAGE_MAX_WIDTH;
  const collapse = userCollapse || autoCollapse;
  const hasDrawer = availableSlideWidth <= SLIDE_PAGE_MIN_WIDTH;
  const mode: DesktopShellLayoutMode = hasDrawer
    ? 'drawer'
    : collapse
      ? 'compactRail'
      : 'fullRail';
  const slideWidth = mode === 'drawer'
    ? 0
    : mode === 'compactRail'
      ? SLIDE_PAGE_MIN_WIDTH
      : SLIDE_PAGE_MAX_WIDTH;
  const mainWidth = Math.max(0, width - slideWidth);
  const routeMode = mainWidth < MAIN_ROUTE_SWITCH_WIDTH;
  const chatSideWidth = Math.max(0, mainWidth - CONVERSATION_LIST_WIDTH);

  return {
    mode,
    slideWidth,
    availableSlideWidth,
    autoCollapse,
    userCollapse,
    routeMode,
    sideRouteMode: !routeMode && chatSideWidth < MAIN_ROUTE_SWITCH_WIDTH,
  };
}
