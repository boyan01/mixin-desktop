import { type CSSProperties, type ReactNode, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { classes } from '@/shared/styles/classes';

export function hoverOverlayCloseDelay(closeWaitDuration = 0, closeDuration = 0): number {
  return Math.max(0, closeWaitDuration) + Math.max(0, closeDuration);
}

interface HoverOverlayProps {
  child: ReactNode;
  portal: ReactNode | ((progress: number) => ReactNode);
  delayDuration?: number;
  duration?: number;
  closeDuration: number;
  closeWaitDuration?: number;
  className?: string;
}

/**
 * Portal hover primitive with Flutter-compatible delayed open and delayed close.
 * The portal stays interactive, so moving the pointer from anchor to overlay does
 * not dismiss it.
 */
export function HoverOverlay({ child, portal, delayDuration, duration = 0, closeDuration, closeWaitDuration = 0, className = '' }: HoverOverlayProps) {
  const anchor = useRef<HTMLSpanElement>(null);
  const openTimer = useRef<number | undefined>(undefined);
  const closeTimer = useRef<number | undefined>(undefined);
  const [visible, setVisible] = useState(false);
  const [progress, setProgress] = useState(0);

  function clearTimers() {
    if (openTimer.current !== undefined) window.clearTimeout(openTimer.current);
    if (closeTimer.current !== undefined) window.clearTimeout(closeTimer.current);
    openTimer.current = undefined;
    closeTimer.current = undefined;
  }
  function open() {
    if (closeTimer.current !== undefined) {
      window.clearTimeout(closeTimer.current);
      closeTimer.current = undefined;
    }
    if (visible || openTimer.current !== undefined) return;
    const show = () => {
      openTimer.current = undefined;
      setVisible(true);
      requestAnimationFrame(() => setProgress(1));
    };
    openTimer.current = delayDuration ? window.setTimeout(show, delayDuration) : undefined;
    if (!delayDuration) show();
  }
  function close() {
    if (openTimer.current !== undefined) { window.clearTimeout(openTimer.current); openTimer.current = undefined; }
    if (!visible || closeTimer.current !== undefined) return;
    setProgress(0);
    closeTimer.current = window.setTimeout(() => { closeTimer.current = undefined; setVisible(false); }, hoverOverlayCloseDelay(closeWaitDuration, closeDuration));
  }
  useEffect(() => () => clearTimers(), []);

  const rect = anchor.current?.getBoundingClientRect();
  const style: CSSProperties | undefined = rect ? { top: rect.bottom, left: rect.left, '--hover-overlay-duration': `${visible ? duration : closeDuration}ms` } as CSSProperties : undefined;
  return <>
    <span ref={anchor} className={classes("hover-overlay-anchor")} onMouseEnter={open} onMouseLeave={close}>{child}</span>
    {visible && rect && createPortal(<div className={classes(`hover-overlay ${className}`)} style={style} onMouseEnter={open} onMouseLeave={close} onClick={close}>
      <div className={classes("hover-overlay-content")} style={{ opacity: progress, transitionDuration: `${visible ? duration : closeDuration}ms` }}>{typeof portal === 'function' ? portal(progress) : portal}</div>
    </div>, document.body)}
  </>;
}
