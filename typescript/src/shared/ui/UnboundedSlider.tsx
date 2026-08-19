import { useRef, useState } from 'react';
import { classes } from '@/shared/styles/classes';

export function unboundedSliderValue(clientX: number, left: number, width: number, min: number, max: number, step: number) {
  if (width <= 0 || max <= min || step <= 0) return min;
  const ratio = Math.max(0, Math.min(1, (clientX - left) / width));
  const raw = min + ratio * (max - min);
  return Math.max(min, Math.min(max, Math.round((raw - min) / step) * step + min));
}

/** DOM counterpart to Flutter's unbounded rounded slider track: its hit/paint range reaches both edges. */
export function UnboundedSlider({ value, min, max, step, onChange, ariaLabel }: {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  ariaLabel: string;
}) {
  const track = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  const percent = (Math.max(min, Math.min(max, value)) - min) / (max - min || 1) * 100;
  const update = (clientX: number) => {
    const rect = track.current?.getBoundingClientRect();
    if (rect) onChange(unboundedSliderValue(clientX, rect.left, rect.width, min, max, step));
  };
  return <div ref={track} className={classes(`unbounded-slider ${dragging ? 'is-dragging' : ''}`)} role="slider" aria-label={ariaLabel} aria-valuemin={min} aria-valuemax={max} aria-valuenow={value} tabIndex={0}
    onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); setDragging(true); update(event.clientX); }}
    onPointerMove={(event) => { if (event.currentTarget.hasPointerCapture(event.pointerId)) update(event.clientX); }}
    onPointerUp={(event) => { if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId); setDragging(false); update(event.clientX); }}
    onKeyDown={(event) => { if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') { event.preventDefault(); onChange(Math.max(min, value - step)); } if (event.key === 'ArrowRight' || event.key === 'ArrowUp') { event.preventDefault(); onChange(Math.min(max, value + step)); } }}>
    <i className={classes("unbounded-slider-inactive")} /><i className={classes("unbounded-slider-active")} style={{ width: `${percent}%` }} /><b style={{ left: `${percent}%` }} />
  </div>;
}
