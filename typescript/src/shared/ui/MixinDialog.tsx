import * as Dialog from '@radix-ui/react-dialog';
import { type CSSProperties, type ReactNode } from 'react';
import { classes } from '@/shared/styles/classes';

export interface MixinDialogProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  padding?: number;
  maxWidth?: number;
  backgroundColor?: string;
  barrierDismissible?: boolean;
  className?: string;
}

/** showMixinDialog: 80ms modal route, 32px outer inset and an 11px popup. */
export function MixinDialog({ open, onClose, children, padding = 32, maxWidth = 600, backgroundColor, barrierDismissible = true, className = '' }: MixinDialogProps) {
  return <Dialog.Root open={open} onOpenChange={(visible) => { if (!visible && barrierDismissible) onClose(); }}>
    <Dialog.Portal>
      <Dialog.Overlay className={classes("mixin-dialog-overlay")} />
      <Dialog.Content className={classes(`mixin-dialog-content ${className}`)} style={{ '--mixin-dialog-padding': `${padding}px`, '--mixin-dialog-max-width': `${maxWidth}px`, ...(backgroundColor ? { backgroundColor } : {}) } as CSSProperties} onPointerDownOutside={(event) => { if (!barrierDismissible) event.preventDefault(); }}>
        {children}
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>;
}

export function AlertDialogLayout({ title, content, actions, minWidth = 400, minHeight = 210, maxWidth, titleMarginBottom = 48 }: { title?: ReactNode; content: ReactNode; actions?: ReactNode; minWidth?: number; minHeight?: number; maxWidth?: number; titleMarginBottom?: number }) {
  return <section className={classes("mixin-alert-dialog")} style={{ minWidth, minHeight, maxWidth }}>
    {title && <header style={{ marginBottom: titleMarginBottom }}>{title}</header>}
    <div className={classes("mixin-alert-content")}>{content}</div>
    <footer>{actions}</footer>
  </section>;
}

export function MixinButton({ children, onClick, disabled = false, transparent = false, type = 'button', className = '' }: { children: ReactNode; onClick?: () => void; disabled?: boolean; transparent?: boolean; type?: 'button' | 'submit'; className?: string }) {
  return <button type={type} className={classes(`mixin-dialog-button${transparent ? ' is-transparent' : ''}${className ? ` ${className}` : ''}`)} disabled={disabled} onClick={onClick}>{children}</button>;
}
