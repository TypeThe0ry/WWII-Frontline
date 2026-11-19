import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { TargetedKeyboardEvent, TargetedPointerEvent } from 'preact';
import { clampView, fitZoom, MAP_SIZE_M, screenToWorld, zoomAt } from './coords';
import { availableLayers, MapRenderer, mapPoint, measureLine, pickSpawn, positionGrid } from './map-renderer';
import type { LayerFlags, MapData, MapFrame, MapMeasure, MapMode, MapPosition, MapSpawn } from './map-renderer';

export interface MapCanvasProps {
  data: MapData;
  mode: MapMode;
  layers?: LayerFlags;
  selectedSpawnId?: number;
  recommendedSpawnId?: number;
  onPickSpawn?: (point: MapSpawn) => void;
  center?: MapPosition;
  focusOnMount?: boolean;
  width?: number;
  height?: number;
}

type Pixel = { x: number; y: number };
type Gesture = { id: number; start: Pixel; last: Pixel; dragged: boolean; measureFrom?: MapPosition };

function eventPixel(event: MouseEvent, canvas: HTMLCanvasElement): Pixel {
  const rect = canvas.getBoundingClientRect();
  return { x: (event.clientX - rect.left) * canvas.width / (rect.width || 1),
    y: (event.clientY - rect.top) * canvas.height / (rect.height || 1) };
}

export function MapCanvas({ data, mode, layers, selectedSpawnId, recommendedSpawnId, onPickSpawn, center,
  focusOnMount, width = 600, height = 460 }: MapCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const fullView = () => ({ zoom: fitZoom(width, height), cx: MAP_SIZE_M / 2, cz: MAP_SIZE_M / 2 });
  const view = useRef(fullView());
  const gesture = useRef<Gesture | undefined>(undefined);
  const measure = useRef<MapMeasure | undefined>(undefined);
  const cursor = useRef<Pixel | undefined>(undefined);
  const [readout, setReadout] = useState('');
  const frame = useRef<Omit<MapFrame, 'view' | 'measure'>>({ data, mode, layers: layers ?? availableLayers(mode) });
  const invalidate = useRef(() => {});

  useLayoutEffect(() => {
    frame.current = { data, mode, layers: layers ?? availableLayers(mode), selectedSpawnId, recommendedSpawnId };
    invalidate.current();
  }, [data, mode, layers, selectedSpawnId, recommendedSpawnId]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const painter = new MapRenderer(canvas);
    const interval = 1000 / (mode === 'deploy' ? 15 : 30);
    view.current = { zoom: fitZoom(width, height), cx: MAP_SIZE_M / 2, cz: MAP_SIZE_M / 2 };
    gesture.current = measure.current = cursor.current = undefined;
    let raf: number | undefined;
    let lastDraw = -Infinity;
    const schedule = () => {
      if (raf === undefined && !document.hidden) raf = requestAnimationFrame(draw);
    };
    const draw = (now: number) => {
      raf = undefined;
      if (document.hidden) return;
      if (now - lastDraw < interval - 1e-6) { schedule(); return; }
      lastDraw = now;
      painter.draw({ ...frame.current, view: view.current, measure: measure.current });
      const line = measure.current;
      const p = cursor.current;
      if (line) {
        const { metres, bearing } = measureLine(line);
        setReadout(`${Math.round(metres)} m · ${Math.round(bearing) % 360}° · ${positionGrid(line.from)} → ${positionGrid(line.to)}`);
      } else if (p) {
        const world = screenToWorld(view.current, width, height, p.x, p.y);
        setReadout(world.x < 0 || world.x > MAP_SIZE_M || world.z < 0 || world.z > MAP_SIZE_M ? '' : positionGrid(world));
      } else setReadout('');
    };
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      if (e.deltaY === 0) return;
      const p = eventPixel(e, canvas);
      cursor.current = p;
      view.current = zoomAt(view.current, width, height, p.x, p.y, e.deltaY < 0 ? 1.25 : 1 / 1.25);
      schedule();
    };
    invalidate.current = schedule;
    canvas.addEventListener('wheel', wheel, { passive: false });
    document.addEventListener('visibilitychange', schedule);
    schedule();
    return () => {
      if (raf !== undefined) cancelAnimationFrame(raf);
      canvas.removeEventListener('wheel', wheel);
      document.removeEventListener('visibilitychange', schedule);
      invalidate.current = () => {};
    };
  }, [width, height, mode]);

  useEffect(() => {
    if (focusOnMount) canvasRef.current?.focus({ preventScroll: true });
  }, [focusOnMount]);

  function pan(dx: number, dy: number) {
    const v = view.current;
    view.current = clampView({ zoom: v.zoom, cx: v.cx - dx / v.zoom, cz: v.cz + dy / v.zoom }, width, height);
    invalidate.current();
  }
  function zoom(factor: number, x = width / 2, y = height / 2) {
    view.current = zoomAt(view.current, width, height, x, y, factor);
    invalidate.current();
  }
  function reset() {
    view.current = fullView();
    measure.current = undefined;
    invalidate.current();
  }
  function recenter() {
    if (!center) return;
    view.current = clampView({ zoom: Math.max(view.current.zoom, fitZoom(width, height) * 2), cx: center.x, cz: center.z }, width, height);
    invalidate.current();
  }
  function world(p: Pixel) {
    return mapPoint(screenToWorld(view.current, width, height, p.x, p.y));
  }
  function start(e: TargetedPointerEvent<HTMLCanvasElement>) {
    if (e.button !== 0 || gesture.current) return;
    const p = eventPixel(e, e.currentTarget);
    gesture.current = { id: e.pointerId, start: p, last: p, dragged: false,
      measureFrom: mode === 'full' && e.shiftKey ? world(p) : undefined };
    if (gesture.current.measureFrom) measure.current = undefined;
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function move(e: TargetedPointerEvent<HTMLCanvasElement>) {
    const p = eventPixel(e, e.currentTarget);
    cursor.current = p;
    const g = gesture.current;
    if (g?.id === e.pointerId && (g.dragged || Math.hypot(g.start.x - p.x, g.start.y - p.y) >= 4)) {
      g.dragged = true;
      if (g.measureFrom) measure.current = { from: g.measureFrom, to: world(p) };
      else pan(p.x - g.last.x, p.y - g.last.y);
      g.last = p;
    }
    invalidate.current();
  }
  function finish(e: TargetedPointerEvent<HTMLCanvasElement>, cancelled = false) {
    const g = gesture.current;
    if (g?.id !== e.pointerId) return;
    if (!cancelled) move(e); // honor a final position even if no pointermove was dispatched there
    gesture.current = undefined;
    if (cancelled && g.measureFrom) measure.current = undefined;
    if (!cancelled && !g.dragged && !g.measureFrom && (layers?.spawnPoints ?? true) && onPickSpawn) {
      const p = eventPixel(e, e.currentTarget);
      const picked = pickSpawn(data.spawnPoints ?? [], view.current, width, height, p.x, p.y);
      if (picked) onPickSpawn(picked);
    }
    invalidate.current();
  }
  function key(e: TargetedKeyboardEvent<HTMLCanvasElement>) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    switch (e.key.toLowerCase()) {
      case 'a': case 'arrowleft': pan(40, 0); break;
      case 'd': case 'arrowright': pan(-40, 0); break;
      case 'w': case 'arrowup': pan(0, 40); break;
      case 's': case 'arrowdown': pan(0, -40); break;
      case '+': case '=': zoom(1.25); break;
      case '-': zoom(1 / 1.25); break;
      case 'home': reset(); break;
      case ' ': recenter(); break;
      default: return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  return <div>
    <canvas ref={canvasRef} width={width} height={height} tabIndex={0}
      aria-label={mode === 'deploy' ? 'Deployment map' : 'Tactical map canvas'}
      onKeyDown={key} onPointerDown={start} onPointerMove={move} onPointerUp={(e) => finish(e)}
      onPointerCancel={(e) => finish(e, true)} onLostPointerCapture={(e) => finish(e, true)}
      onPointerLeave={() => { cursor.current = undefined; invalidate.current(); }}
      onDblClick={(e) => { const p = eventPixel(e, e.currentTarget); zoom(1.5, p.x, p.y); }}>
      This map requires Canvas 2D.
    </canvas>
    <div role="group" aria-label="Map navigation">
      <button type="button" onClick={() => zoom(1.25)} aria-label="Zoom in">+</button>
      <button type="button" onClick={() => zoom(1 / 1.25)} aria-label="Zoom out">−</button>
      <button type="button" onClick={reset}>Full map</button>
      <button type="button" disabled={!center} onClick={recenter}>{mode === 'deploy' ? 'Center on spawn' : 'Center on self'}</button>
    </div>
    <div>{readout}</div>
  </div>;
}
