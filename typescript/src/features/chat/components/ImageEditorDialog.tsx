import { useEffect, useMemo, useRef, useState } from 'react';
import { Check, Crop, Eraser, FlipHorizontal, Pencil, Redo2, RotateCw, Undo2, X } from 'lucide-react';
import { IconButton } from '@/shared/ui/IconButton';
import { CustomPopupMenuButton } from '@/shared/ui/CustomPopupMenu';
import { resolveAssetUrl } from '@/shared/api/desktopApi';
import { classes } from '@/shared/styles/classes';

type DrawMode = 'none' | 'brush' | 'eraser';
type Point = { x: number; y: number };
type Stroke = { points: Point[]; color: string };

interface ImageEditorDialogProps {
  path: string;
  onClose: () => void;
  /** Receives a PNG data URL. Persist it through desktopApi.writeEditedImage before sending. */
  onDone: (dataUrl: string) => Promise<void> | void;
}

const colors = ['#ff4c4f', '#ffae23', '#39c5bb', '#3d75e3', '#8e5ce3', '#000000', '#ffffff'];
const ratios: Array<[string, number | undefined]> = [['Original', undefined], ['1:1', 1], ['2:3', 2 / 3], ['3:2', 3 / 2], ['3:4', 3 / 4], ['4:3', 4 / 3], ['9:16', 9 / 16], ['16:9', 16 / 9]];

function distance(point: Point, other: Point) {
  return Math.hypot(point.x - other.x, point.y - other.y);
}

function transformedSize(width: number, height: number, quarterTurns: number) {
  return quarterTurns % 2 ? { width: height, height: width } : { width, height };
}

export function ImageEditorDialog({ path, onClose, onDone }: ImageEditorDialogProps) {
  const source = resolveAssetUrl(path);
  const image = useRef<HTMLImageElement | null>(null);
  const preview = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);
  const [quarterTurns, setQuarterTurns] = useState(0);
  const [flipped, setFlipped] = useState(false);
  const [cropRatio, setCropRatio] = useState<number>();
  const [drawMode, setDrawMode] = useState<DrawMode>('none');
  const [color, setColor] = useState(colors[0]);
  const [strokes, setStrokes] = useState<Stroke[]>([]);
  const [redo, setRedo] = useState<Stroke[]>([]);
  const [backup, setBackup] = useState<Stroke[]>([]);
  const [current, setCurrent] = useState<Stroke>();
  const [saving, setSaving] = useState(false);
  const canReset = quarterTurns !== 0 || flipped || cropRatio !== undefined || strokes.length > 0;
  const currentRatio = useMemo(() => {
    if (!ready || !image.current) return cropRatio;
    const size = transformedSize(image.current.naturalWidth, image.current.naturalHeight, quarterTurns);
    return cropRatio ?? size.width / size.height;
  }, [cropRatio, quarterTurns, ready]);

  useEffect(() => {
    function keyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose();
    }
    window.addEventListener('keydown', keyDown);
    return () => window.removeEventListener('keydown', keyDown);
  }, [onClose]);

  function pointFor(event: React.PointerEvent<HTMLDivElement>): Point | undefined {
    const bounds = preview.current?.getBoundingClientRect();
    if (!bounds || !bounds.width || !bounds.height) return undefined;
    return { x: (event.clientX - bounds.left) / bounds.width, y: (event.clientY - bounds.top) / bounds.height };
  }

  function begin(event: React.PointerEvent<HTMLDivElement>) {
    const point = pointFor(event);
    if (!point) return;
    if (drawMode === 'eraser') {
      const index = strokes.map((stroke) => stroke.points.some((item) => distance(item, point) <= .055)).lastIndexOf(true);
      if (index >= 0) setStrokes((items) => {
        const next = [...items];
        const [removed] = next.splice(index, 1);
        setRedo((redoItems) => [...redoItems, removed]);
        return next;
      });
      return;
    }
    if (drawMode !== 'brush') return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const stroke = { color, points: [point] };
    setRedo([]);
    setCurrent(stroke);
    setStrokes((items) => [...items, stroke]);
  }

  function move(event: React.PointerEvent<HTMLDivElement>) {
    if (drawMode !== 'brush' || !current || !event.currentTarget.hasPointerCapture(event.pointerId)) return;
    const point = pointFor(event);
    if (!point) return;
    setCurrent((stroke) => stroke && { ...stroke, points: [...stroke.points, point] });
    setStrokes((items) => [...items.slice(0, -1), { ...current, points: [...current.points, point] }]);
  }

  function finish(event: React.PointerEvent<HTMLDivElement>) {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    setCurrent(undefined);
  }

  async function done() {
    const sourceImage = image.current;
    if (!sourceImage || saving) return;
    setSaving(true);
    try {
      const rotated = transformedSize(sourceImage.naturalWidth, sourceImage.naturalHeight, quarterTurns);
      const ratio = cropRatio ?? rotated.width / rotated.height;
      let cropWidth = rotated.width;
      let cropHeight = rotated.height;
      if (cropWidth / cropHeight > ratio) cropWidth = cropHeight * ratio;
      else cropHeight = cropWidth / ratio;
      const scale = Math.min(1, 2048 / Math.max(cropWidth, cropHeight));
      const canvas = document.createElement('canvas');
      canvas.width = Math.max(1, Math.round(cropWidth * scale));
      canvas.height = Math.max(1, Math.round(cropHeight * scale));
      const context = canvas.getContext('2d');
      if (!context) return;
      context.translate(canvas.width / 2, canvas.height / 2);
      context.rotate(quarterTurns * Math.PI / 2);
      context.scale(flipped ? -1 : 1, 1);
      const sourceScale = Math.max(cropWidth / rotated.width, cropHeight / rotated.height);
      context.drawImage(sourceImage, -sourceImage.naturalWidth * sourceScale * scale / 2, -sourceImage.naturalHeight * sourceScale * scale / 2, sourceImage.naturalWidth * sourceScale * scale, sourceImage.naturalHeight * sourceScale * scale);
      context.setTransform(1, 0, 0, 1, 0, 0);
      for (const stroke of strokes) {
        if (stroke.points.length < 2) continue;
        context.beginPath();
        context.moveTo(stroke.points[0].x * canvas.width, stroke.points[0].y * canvas.height);
        for (const point of stroke.points.slice(1)) context.lineTo(point.x * canvas.width, point.y * canvas.height);
        context.strokeStyle = stroke.color;
        context.lineWidth = 11 * scale;
        context.lineCap = 'round';
        context.lineJoin = 'round';
        context.stroke();
      }
      await onDone(canvas.toDataURL('image/png'));
    } finally {
      setSaving(false);
    }
  }

  return <div className={classes("image-editor")} role="dialog" aria-modal="true" aria-label="Edit image">
    <div className={classes("image-editor-preview-wrap")}>
      {!ready && <span className={classes("image-editor-loading")} />}
      <div ref={preview} className={classes("image-editor-preview")} style={{ aspectRatio: currentRatio, visibility: ready ? 'visible' : 'hidden' }} onPointerDown={begin} onPointerMove={move} onPointerUp={finish} onPointerCancel={finish}>
        {source && <img ref={image} src={source} alt="" draggable={false} onLoad={() => setReady(true)} style={{ transform: `rotate(${quarterTurns * 90}deg) scaleX(${flipped ? -1 : 1})` }} />}
        <svg viewBox="0 0 1 1" preserveAspectRatio="none" aria-hidden="true">{strokes.map((stroke, index) => <polyline key={index} points={stroke.points.map((point) => `${point.x},${point.y}`).join(' ')} fill="none" stroke={stroke.color} strokeWidth=".025" strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke" />)}</svg>
      </div>
    </div>
    <div className={classes("image-editor-tools")}>
      {drawMode === 'none' ? <button type="button" disabled={!canReset} onClick={() => { setQuarterTurns(0); setFlipped(false); setCropRatio(undefined); setStrokes([]); setRedo([]); }}>Reset</button> : <div className={classes("image-editor-colors")}>{colors.map((item) => <button type="button" key={item} className={classes(item === color ? 'is-selected' : '')} style={{ background: item }} onClick={() => setColor(item)} />)}</div>}
      {drawMode === 'none' ? <nav><button type="button" onClick={onClose}>Cancel</button><IconButton label="Rotate" onClick={() => setQuarterTurns((value) => (value + 1) % 4)}><RotateCw size={20} /></IconButton><IconButton label="Flip" onClick={() => setFlipped((value) => !value)}><FlipHorizontal size={20} /></IconButton><CustomPopupMenuButton
        align="center"
        trigger={<IconButton label="Crop ratio" className={classes("image-editor-crop")}><Crop size={20} /></IconButton>}
        onSelected={setCropRatio}
        items={ratios.map(([title, value]) => ({ title, value }))}
      /><IconButton label="Draw" onClick={() => { setBackup(strokes); setDrawMode('brush'); }}><Pencil size={20} /></IconButton><button type="button" disabled={saving} onClick={() => void done()}>{saving ? '…' : 'Done'}</button></nav> : <nav><button type="button" onClick={() => { setStrokes(backup); setRedo([]); setDrawMode('none'); }}>Cancel</button><IconButton label="Undo" disabled={strokes.length === 0} onClick={() => setStrokes((items) => { const next = [...items]; const removed = next.pop(); if (removed) setRedo((redoItems) => [...redoItems, removed]); return next; })}><Undo2 size={20} /></IconButton><IconButton label="Redo" disabled={redo.length === 0} onClick={() => setRedo((items) => { const next = [...items]; const restored = next.pop(); if (restored) setStrokes((strokesItems) => [...strokesItems, restored]); return next; })}><Redo2 size={20} /></IconButton><IconButton label="Brush" onClick={() => setDrawMode('brush')}><Pencil size={20} /></IconButton><IconButton label="Eraser" onClick={() => setDrawMode('eraser')}><Eraser size={20} /></IconButton><button type="button" onClick={() => { setBackup([]); setDrawMode('none'); }}><Check size={18} />Done</button></nav>}
    </div>
    <IconButton className={classes("image-editor-close")} label="Close image editor" onClick={onClose}><X size={20} /></IconButton>
  </div>;
}
