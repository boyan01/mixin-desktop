import { IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import type { MenuIcon } from '@tauri-apps/api/image';
import { LogicalPosition } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { e } from '@/shared/lib/logger';

export type NativeMenuAction = () => void | Promise<void>;

export type NativeMenuEntry =
  | { type: 'item'; id: string; text: string; enabled?: boolean; accelerator?: string; icon?: MenuIcon | Promise<MenuIcon>; action: NativeMenuAction }
  | { type: 'separator' }
  | { type: 'submenu'; id: string; text: string; enabled?: boolean; items: NativeMenuEntry[] };

type NativeMenuResource = Menu | Submenu;

let activeMenu: Menu | undefined;

async function disposeMenu(menu: NativeMenuResource) {
  if (activeMenu === menu) activeMenu = undefined;
  try {
    await menu.close();
  } catch {
    // The native menu may already be closed by the platform.
  }
}

async function buildEntries(entries: NativeMenuEntry[], onActionFinished: () => void): Promise<Array<IconMenuItem | MenuItem | PredefinedMenuItem | Submenu>> {
  return Promise.all(entries.map(async (entry) => {
    if (entry.type === 'separator') return PredefinedMenuItem.new({ item: 'Separator' });
    if (entry.type === 'submenu') {
      return Submenu.new({
        id: entry.id,
        text: entry.text,
        enabled: entry.enabled,
        items: await buildEntries(entry.items, onActionFinished),
      });
    }
    let icon: MenuIcon | undefined;
    if (entry.icon) {
      try {
        icon = await entry.icon;
      } catch (reason) {
        e(`Unable to load native menu icon (${entry.id})`, reason);
      }
    }
    const options = {
      id: entry.id,
      text: entry.text,
      enabled: entry.enabled,
      accelerator: entry.accelerator,
      ...(icon ? { icon } : {}),
      action: () => {
        void Promise.resolve()
          .then(entry.action)
          .catch((reason) => e(`Native menu action failed (${entry.id})`, reason))
          .finally(onActionFinished);
      },
    };
    return icon ? IconMenuItem.new(options) : MenuItem.new(options);
  }));
}

export async function showNativeContextMenu(event: Pick<MouseEvent, 'clientX' | 'clientY' | 'preventDefault'>, entries: NativeMenuEntry[]) {
  event.preventDefault();
  if (entries.length === 0) return;
  const position = new LogicalPosition(event.clientX, event.clientY);
  if (activeMenu) await disposeMenu(activeMenu);

  let menu: Menu | undefined;
  const closeMenu = () => { if (menu) void disposeMenu(menu); };
  try {
    const items = await buildEntries(entries, closeMenu);
    menu = await Menu.new({ items });
    activeMenu = menu;
    await menu.popup(position, getCurrentWindow());
  } catch (reason) {
    if (menu) await disposeMenu(menu);
    e('Unable to show native context menu', reason);
  }
}
