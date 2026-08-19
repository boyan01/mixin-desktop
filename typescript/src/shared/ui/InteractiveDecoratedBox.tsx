import { type CSSProperties, type HTMLAttributes, type MouseEvent, type ReactNode, useState } from 'react';
import { classes } from '@/shared/styles/classes';

export type InteractiveStatus = 'interactive' | 'hovering' | 'tapDowning';

export function interactiveStatus(hovering: boolean, tapDowning: boolean): InteractiveStatus {
  if (tapDowning) return 'tapDowning';
  return hovering ? 'hovering' : 'interactive';
}

interface InteractiveDecoratedBoxProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  children: ReactNode | ((status: InteractiveStatus, lastStatus: InteractiveStatus) => ReactNode);
  inDuration?: number;
  outDuration?: number;
  onRightClick?: (event: MouseEvent<HTMLDivElement>) => void;
}

/**
 * Flutter's InteractiveDecoratedBox equivalent. Consumers style the three
 * explicit states while this component keeps hover, press and transition timing
 * consistent across desktop controls.
 */
export function InteractiveDecoratedBox({
  children,
  className = '',
  inDuration = 120,
  outDuration = 60,
  onRightClick,
  onMouseEnter,
  onMouseLeave,
  onMouseDown,
  onMouseUp,
  onContextMenu,
  style,
  ...props
}: InteractiveDecoratedBoxProps) {
  const [hovering, setHovering] = useState(false);
  const [tapDowning, setTapDowning] = useState(false);
  const [lastStatus, setLastStatus] = useState<InteractiveStatus>('interactive');
  const status = interactiveStatus(hovering, tapDowning);
  const duration = lastStatus === 'interactive' && status !== 'interactive' ? inDuration : outDuration;
  const resolvedChildren = typeof children === 'function' ? children(status, lastStatus) : children;

  function update(nextHovering: boolean, nextTapDowning: boolean) {
    setLastStatus(interactiveStatus(hovering, tapDowning));
    setHovering(nextHovering);
    setTapDowning(nextTapDowning);
  }

  return <div
    {...props}
    className={classes(`interactive-decorated-box interactive-${status} ${className}`)}
    style={{ '--interactive-duration': `${duration}ms`, ...style } as CSSProperties}
    onMouseEnter={(event) => { update(true, tapDowning); onMouseEnter?.(event); }}
    onMouseLeave={(event) => { update(false, false); onMouseLeave?.(event); }}
    onMouseDown={(event) => { update(hovering, true); onMouseDown?.(event); }}
    onMouseUp={(event) => { update(hovering, false); onMouseUp?.(event); }}
    onContextMenu={(event) => { onContextMenu?.(event); onRightClick?.(event); }}
  >{resolvedChildren}</div>;
}
