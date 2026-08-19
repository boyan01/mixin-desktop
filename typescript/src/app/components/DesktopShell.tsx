import { useEffect, useState } from 'react';
import { Chat } from '@/features/chat/components/Chat';
import { ConversationList } from '@/features/conversations/components/ConversationList';
import { Settings } from '@/features/settings/components/Settings';
import { Sidebar } from '@/features/conversations/components/Sidebar';
import { useDesktopUi } from '@/state/desktopUi';
import { resolveDesktopShellLayout } from '@/shared/layout/desktopShellLayout';
import type { ReactNode } from 'react';
import type { BootstrapData, Profile, SecurityState } from '@/shared/model/types';
import { classes } from '@/shared/styles/classes';

interface DesktopShellProps {
  data: BootstrapData & { profile: Profile };
  security: SecurityState;
  onProfileUpdated: (profile: Profile) => void;
  onSignedOut: () => void;
  onOpenUrl?: (url: string) => void;
  info?: ReactNode;
  search?: ReactNode;
}

function viewportWidth() {
  return typeof window === 'undefined' ? 1280 : window.innerWidth;
}

export function DesktopShell({
  data,
  security,
  onProfileUpdated,
  onSignedOut,
  onOpenUrl,
  info,
  search,
}: DesktopShellProps) {
  const [width, setWidth] = useState(viewportWidth);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const selectedId = useDesktopUi((state) => state.selectedConversationId);
  const infoOpen = useDesktopUi((state) => state.infoOpen);
  const messageSearchOpen = useDesktopUi((state) => state.messageSearchOpen);
  const settingsOpen = useDesktopUi((state) => state.settingsOpen);
  const sidebarCollapsed = useDesktopUi((state) => state.sidebarCollapsed);
  const selected = data.conversations.find((item) => item.conversationId === selectedId);
  const layout = resolveDesktopShellLayout(width, sidebarCollapsed);
  const infoOpenForConversation = Boolean(selected && infoOpen && info);
  const sideOpenForConversation = Boolean(selected && (infoOpenForConversation || messageSearchOpen) && (info || search));
  const side = infoOpenForConversation ? info : messageSearchOpen ? search : undefined;
  const wideSide = !settingsOpen && sideOpenForConversation && !layout.routeMode && !layout.sideRouteMode ? side : undefined;

  useEffect(() => {
    const onResize = () => setWidth(viewportWidth());
    window.addEventListener('resize', onResize);
    onResize();
    return () => window.removeEventListener('resize', onResize);
  }, []);

  useEffect(() => {
    if (layout.mode !== 'drawer') setDrawerOpen(false);
  }, [layout.mode]);

  const closeDrawer = () => setDrawerOpen(false);
  const sidebar = (
    <Sidebar
      profile={data.profile}
      conversations={data.conversations}
      collapsed={layout.mode === 'drawer' ? false : layout.mode === 'compactRail' || sidebarCollapsed}
      showCollapseControl={!layout.autoCollapse && layout.mode !== 'drawer'}
      onClose={closeDrawer}
    />
  );
  const openDrawer = () => setDrawerOpen(true);
  const conversationList = (
    <ConversationList
      conversations={data.conversations}
      currentUserId={data.profile.userId}
      connected={data.connectionStatus ?? undefined}
      onOpenUrl={onOpenUrl}
      onOpenSidebar={layout.mode === 'drawer' ? openDrawer : undefined}
    />
  );
  const chat = (
    <Chat
      conversation={selected}
      currentUserId={data.profile.userId}
    />
  );

  return (
    <>
      <div
        className={classes(`desktop-shell shell-${layout.mode} ${layout.routeMode ? 'shell-route' : ''} ${layout.sideRouteMode ? 'shell-side-route' : ''} ${settingsOpen ? 'settings-mode' : ''} ${selected ? 'has-selection' : ''} ${wideSide ? 'has-info' : ''}`)}
      >
        {layout.mode !== 'drawer' && sidebar}
        {settingsOpen ? (
          <Settings
            profile={data.profile}
            onProfileUpdated={onProfileUpdated}
            onSignedOut={onSignedOut}
            security={security}
          />
        ) : layout.routeMode ? (
          selected && sideOpenForConversation ? side : selected ? chat : conversationList
        ) : (
          <>
            {conversationList}
            {layout.sideRouteMode && sideOpenForConversation ? side : chat}
          </>
        )}
        {wideSide}
      </div>
      {layout.mode === 'drawer' && drawerOpen && (
        <div className={classes("mobile-drawer-layer")} role="presentation" onMouseDown={closeDrawer}>
          <aside className={classes("mobile-drawer")} onMouseDown={(event) => event.stopPropagation()}>
            {sidebar}
          </aside>
        </div>
      )}
    </>
  );
}
