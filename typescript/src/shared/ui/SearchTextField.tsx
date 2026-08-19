import { X } from 'lucide-react';
import { type ChangeEventHandler, type InputHTMLAttributes, type ReactNode, useRef } from 'react';
import { InteractiveDecoratedBox } from '@/shared/ui/InteractiveDecoratedBox';
import { useEditableSelectionToolbar } from '@/shared/ui/AdaptiveSelectionToolbar';
import searchAsset from '@flutter-assets/images/ic_search_small.svg';
import { classes } from '@/shared/styles/classes';

interface SearchTextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'size'> {
  value: string;
  onChange: ChangeEventHandler<HTMLInputElement>;
  onClear: () => void;
  showClear?: boolean;
  leading?: ReactNode;
  fontSize?: number;
}

/** Matches Flutter SearchTextField: 36px stadium, 200-character input and a reserved trailing target. */
export function SearchTextField({
  value,
  onChange,
  onClear,
  showClear = false,
  leading,
  fontSize = 14,
  className = '',
  maxLength = 200,
  onContextMenu,
  ...props
}: SearchTextFieldProps) {
  const input = useRef<HTMLInputElement>(null);
  const selectionToolbar = useEditableSelectionToolbar(input);
  const canClear = showClear || value.length > 0;
  return <InteractiveDecoratedBox
    className={classes(`mixin-search-field ${className}`)}
    data-tauri-drag-region="false"
    role="presentation"
    onMouseDown={(event) => {
      if (!(event.target instanceof Element) || !event.target.closest('button')) input.current?.focus();
    }}
  >
    <span className={classes("mixin-search-icon")} aria-hidden="true" style={{ '--mixin-search-icon-image': `url("${searchAsset}")` } as React.CSSProperties} />
    {leading}
    <input
      {...props}
      ref={input}
      value={value}
      maxLength={maxLength}
      onChange={onChange}
      onContextMenu={(event) => { onContextMenu?.(event); if (!event.defaultPrevented) selectionToolbar.onContextMenu(event); }}
      style={{ fontSize, ...props.style }}
    />
    {canClear
      ? <button className={classes("mixin-search-clear")} type="button" aria-label="Clear search" onClick={onClear}><X size={16} /></button>
      : <span className={classes("mixin-search-trailing-space")} aria-hidden="true" />}
    {selectionToolbar.toolbar}
  </InteractiveDecoratedBox>;
}
