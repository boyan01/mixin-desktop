import { createPortal } from 'react-dom';
import { type MouseEvent, type RefObject, useEffect, useState } from 'react';
import { e } from '@/shared/lib/logger';
import { classes } from '@/shared/styles/classes';

export interface MenuPoint { x: number; y: number; }
export interface MenuSize { width: number; height: number; }

/** Flutter _PositionedLayoutDelegate: place after the anchor, or before it on overflow. */
export function positionedMenu(anchor: MenuPoint, menu: MenuSize, viewport: MenuSize): MenuPoint {
  return {
    x: viewport.width - anchor.x < menu.width ? anchor.x - menu.width : anchor.x + 1,
    y: viewport.height - anchor.y < menu.height ? anchor.y - menu.height : anchor.y + 1,
  };
}

export interface SelectionToolbarActions {
  copy?: () => void;
  cut?: () => void;
  selectAll?: () => void;
  paste?: () => void;
}

export function selectionToolbarLabels(actions: SelectionToolbarActions) {
  return [
    actions.copy && 'Copy',
    actions.cut && 'Cut',
    actions.selectAll && 'Select All',
    actions.paste && 'Paste',
  ].filter((value): value is string => Boolean(value));
}

/** Desktop counterpart of MixinAdaptiveSelectionToolbar and SelectionArea toolbar. */
export function AdaptiveSelectionToolbar({ anchor, actions, onDismiss }: { anchor: MenuPoint; actions: SelectionToolbarActions; onDismiss: () => void }) {
  const [menu, setMenu] = useState<MenuSize>({ width: 160, height: 40 });
  useEffect(() => {
    const close = (event: KeyboardEvent) => { if (event.key === 'Escape') onDismiss(); };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [onDismiss]);
  const position = positionedMenu(anchor, menu, { width: window.innerWidth, height: window.innerHeight });
  const items: Array<[string, (() => void) | undefined]> = [['Copy', actions.copy], ['Cut', actions.cut], ['Select All', actions.selectAll], ['Paste', actions.paste]];
  return createPortal(<div className={classes("mixin-selection-toolbar-scrim")} role="presentation" onMouseDown={onDismiss}>
    <div ref={(element) => { if (element && (element.offsetWidth !== menu.width || element.offsetHeight !== menu.height)) setMenu({ width: element.offsetWidth, height: element.offsetHeight }); }} className={classes("mixin-selection-toolbar")} role="menu" style={{ left: position.x, top: position.y }} onMouseDown={(event) => event.stopPropagation()}>
      {items.map(([label, action]) => action && <button key={label} type="button" role="menuitem" onClick={() => { action(); onDismiss(); }}>{label}</button>)}
    </div>
  </div>, document.body);
}

type Editable = HTMLInputElement | HTMLTextAreaElement;
type SelectionEditable = Editable | HTMLElement;

function hasEditableSelection(element: HTMLElement) {
  const selection = window.getSelection();
  return Boolean(selection && !selection.isCollapsed && selection.rangeCount > 0 && element.contains(selection.getRangeAt(0).commonAncestorContainer));
}

function editableActions(element: SelectionEditable): SelectionToolbarActions {
  if (!(element instanceof HTMLInputElement) && !(element instanceof HTMLTextAreaElement)) {
    return {
      copy: hasEditableSelection(element) ? () => { document.execCommand('copy'); } : undefined,
      cut: hasEditableSelection(element) ? () => { document.execCommand('cut'); } : undefined,
      selectAll: () => {
        const range = document.createRange();
        range.selectNodeContents(element);
        const selection = window.getSelection();
        selection?.removeAllRanges();
        selection?.addRange(range);
      },
      paste: () => { void navigator.clipboard.readText().then((text) => {
        document.execCommand('insertText', false, text);
        element.dispatchEvent(new Event('input', { bubbles: true }));
      }).catch((reason) => e('Unable to paste into editable content', reason)); },
    };
  }
  const selected = element.selectionStart !== element.selectionEnd;
  return {
    copy: selected ? () => { document.execCommand('copy'); } : undefined,
    cut: selected && !element.readOnly && !element.disabled ? () => { document.execCommand('cut'); } : undefined,
    selectAll: () => { element.select(); },
    paste: !element.readOnly && !element.disabled ? () => { void navigator.clipboard.readText().then((text) => {
      const start = element.selectionStart ?? 0;
      const end = element.selectionEnd ?? start;
      element.setRangeText(text, start, end, 'end');
      element.dispatchEvent(new Event('input', { bubbles: true }));
    }).catch((reason) => e('Unable to paste into editable content', reason)); } : undefined,
  };
}

function editableTarget(target: EventTarget | null): SelectionEditable | undefined {
  if (!(target instanceof Element)) return undefined;
  const element = target.closest('input, textarea, [contenteditable="true"]');
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) return element;
  if (element instanceof HTMLElement && element.isContentEditable) return element;
  return undefined;
}

/**
 * Flutter installs MixinAdaptiveSelectionToolbar on every TextField.  A
 * capture listener gives browser-native inputs the same common toolbar,
 * including fields rendered inside independently owned dialogs.
 */
export function DocumentSelectionToolbar() {
  const [state, setState] = useState<{ anchor: MenuPoint; element: SelectionEditable }>();
  useEffect(() => {
    function contextMenu(event: globalThis.MouseEvent) {
      const element = editableTarget(event.target);
      if (!element) return;
      if (element instanceof HTMLInputElement && ['button', 'checkbox', 'color', 'date', 'file', 'hidden', 'image', 'radio', 'range', 'reset', 'submit', 'time'].includes(element.type)) return;
      event.preventDefault();
      setState({ anchor: { x: event.clientX, y: event.clientY }, element });
    }
    document.addEventListener('contextmenu', contextMenu, true);
    return () => document.removeEventListener('contextmenu', contextMenu, true);
  }, []);
  return state && <AdaptiveSelectionToolbar anchor={state.anchor} actions={editableActions(state.element)} onDismiss={() => setState(undefined)} />;
}

/**
 * Applies Flutter's explicit desktop text-action menu to a native editable
 * field. Browser clipboard permissions are allowed to reject paste, matching
 * Flutter's disabled action when a platform clipboard cannot provide text.
 */
export function useEditableSelectionToolbar<T extends Editable>(ref: RefObject<T | null>) {
  const [anchor, setAnchor] = useState<MenuPoint>();
  const element = ref.current;
  const actions = element ? editableActions(element) : {};
  return {
    onContextMenu(event: MouseEvent<Editable>) {
      event.preventDefault();
      setAnchor({ x: event.clientX, y: event.clientY });
    },
    toolbar: anchor && <AdaptiveSelectionToolbar anchor={anchor} actions={actions} onDismiss={() => setAnchor(undefined)} />,
  };
}
