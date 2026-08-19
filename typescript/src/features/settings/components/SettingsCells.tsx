import { type CSSProperties, type ReactNode } from 'react';
import arrowRightAsset from '@flutter-assets/images/ic_arrow_right.svg';
import { classes } from '@/shared/styles/classes';

interface SettingsCellGroupProps {
  children: ReactNode;
  className?: string;
}

/** Flutter CellGroup: a 600px-wide rounded list with 10px side/bottom insets. */
export function SettingsCellGroup({ children, className = '' }: SettingsCellGroupProps) {
  return <section className={classes(`settings-cell-group ${className}`)}>{children}</section>;
}

interface SettingsCellItemProps {
  title: ReactNode;
  leading?: ReactNode;
  description?: ReactNode;
  trailing?: ReactNode | null;
  selected?: boolean;
  color?: string;
  destructive?: boolean;
  disabled?: boolean;
  onClick?: () => void;
}

/** Flutter CellItem's 17px vertical/16px leading geometry and selected fill. */
export function SettingsCellItem({ title, leading, description, trailing = <span className={classes("settings-cell-arrow")} aria-hidden="true" style={{ '--settings-cell-arrow-image': `url("${arrowRightAsset}")` } as CSSProperties} />, selected = false, color, destructive = false, disabled = false, onClick }: SettingsCellItemProps) {
  return <button type="button" className={classes(`settings-cell-item${selected ? ' is-selected' : ''}${destructive ? ' is-danger' : ''}`)} style={color ? { color } : undefined} disabled={disabled} onClick={onClick}>
    {leading && <span className={classes("settings-cell-leading")}>{leading}</span>}
    <span className={classes("settings-cell-title")}>{title}</span>
    {description && <span className={classes("settings-cell-description")}>{description}</span>}
    {trailing && <span className={classes("settings-cell-trailing")}>{trailing}</span>}
  </button>;
}
