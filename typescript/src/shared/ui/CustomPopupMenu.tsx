import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { type ReactNode, useState } from 'react';
import { classes } from '@/shared/styles/classes';

export interface CustomPopupMenuItem<T> {
  title: string;
  value: T;
  destructive?: boolean;
  icon?: ReactNode;
}

/** Flutter CustomPopupMenuButton: tap-up anchored menu with Escape/outside dismissal. */
export function CustomPopupMenuButton<T>({ trigger, items, onSelected, onVisibilityChanged, align = 'start', side = 'bottom' }: { trigger: ReactNode; items: CustomPopupMenuItem<T>[]; onSelected: (value: T) => void; onVisibilityChanged?: (visible: boolean) => void; align?: 'start' | 'center' | 'end'; side?: 'top' | 'right' | 'bottom' | 'left' }) {
  const [open, setOpen] = useState(false);
  function change(next: boolean) { setOpen(next); onVisibilityChanged?.(next); }
  return <DropdownMenu.Root open={open} onOpenChange={change}>
    <DropdownMenu.Trigger asChild>{trigger}</DropdownMenu.Trigger>
    <DropdownMenu.Portal><DropdownMenu.Content className={classes("mixin-popup-menu")} side={side} align={align} sideOffset={1}>
      {items.map((item) => <DropdownMenu.Item key={item.title} className={classes(`mixin-popup-menu-item${item.destructive ? ' is-danger' : ''}`)} onSelect={() => onSelected(item.value)}>
        {item.icon && <span>{item.icon}</span>}<span>{item.title}</span>
      </DropdownMenu.Item>)}
    </DropdownMenu.Content></DropdownMenu.Portal>
  </DropdownMenu.Root>;
}
