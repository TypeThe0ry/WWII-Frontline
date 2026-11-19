import { CELL_M, GRID_N, MAP_SIZE_M, formatGrid, gridDetail, worldToGrid, worldToScreen } from './coords';
import type { View } from './coords';
import { unitStaleness } from './map-redraw';
import { markerRemaining } from '../../game/markers';
import type { Marker } from '../../game/markers';
import type { SpawnPoint } from '../../game/deploy-validate';

/** x east, z north, metres measured from the map's south-west corner. */
export interface MapPosition { x: number; z: number }
export interface MapSpawn extends SpawnPoint, MapPosition { label: string }
export interface MapObjective extends MapPosition { id: number; label: string; owner: 'friendly' | 'enemy' | 'neutral' }
export interface MapUnit extends MapPosition {
  id: number;
  /** Already filtered to allies by the caller. */
  relation: 'self' | 'squad' | 'friendly';
  label?: string;
  downed?: boolean;
  /** Heading in radians, clockwise from north. */
  yaw?: number;
  /** Optional TEAM receipt time, in the same clock as nowS. */
  updatedAtS?: number;
}
export interface MapData {
  /** Optional preloaded, north-up image covering the playable square. The fallback is a plain grid. */
  base?: CanvasImageSource;
  objectives?: readonly MapObjective[];
  spawnPoints?: readonly MapSpawn[];
  units?: readonly MapUnit[];
  /** Authoritative/display-only data; the live M1 server does not supply marker placement. */
  markers?: readonly Marker[];
  nowS?: number;
}
export const MAP_LAYERS = ['base', 'grid', 'objectives', 'spawnPoints', 'units', 'markers'] as const;
export type MapLayer = typeof MAP_LAYERS[number];
export type LayerFlags = Record<MapLayer, boolean>;
export type MapMode = 'full' | 'deploy';
export interface MapMeasure { from: MapPosition; to: MapPosition }

export function measureLine({ from, to }: MapMeasure): { metres: number; bearing: number } {
  const dx = to.x - from.x;
  const dz = to.z - from.z;
  return { metres: Math.hypot(dx, dz), bearing: (Math.atan2(dx, dz) * 180 / Math.PI + 360) % 360 };
}

export function mapPoint(p: MapPosition): MapPosition {
  return { x: Math.max(0, Math.min(MAP_SIZE_M, p.x)), z: Math.max(0, Math.min(MAP_SIZE_M, p.z)) };
}

export function availableLayers(mode: MapMode): LayerFlags {
  return { base: true, grid: true, objectives: true, spawnPoints: true, units: true, markers: mode === 'full' };
}

export function pickSpawn(points: readonly MapSpawn[], view: View, w: number, h: number, x: number, y: number): MapSpawn | undefined {
  let best: MapSpawn | undefined;
  let bestDistance = 13;
  for (const point of points) {
    const screen = worldToScreen(view, w, h, point.x, point.z);
    const distance = Math.hypot(screen.sx - x, screen.sy - y);
    if (distance < bestDistance) { best = point; bestDistance = distance; }
  }
  return best;
}

export interface MapFrame {
  data: MapData;
  view: View;
  mode: MapMode;
  layers: LayerFlags;
  selectedSpawnId?: number;
  recommendedSpawnId?: number;
  measure?: MapMeasure;
}

const SPAWN_GLYPH: Record<MapSpawn['status'], string> = {
  available: '+', blocked: '/', cooldown: '◷', destroyed: '×', full: '■', locked: '!',
};

function startMap(ctx: CanvasRenderingContext2D, w: number, h: number, view: View): void {
  const top = worldToScreen(view, w, h, 0, MAP_SIZE_M);
  const size = MAP_SIZE_M * view.zoom;
  ctx.save();
  ctx.beginPath();
  ctx.rect(top.sx, top.sy, size, size);
  ctx.clip();
  ctx.font = '12px sans-serif';
  ctx.textBaseline = 'middle';
}

function drawBase(ctx: CanvasRenderingContext2D, w: number, h: number, frame: MapFrame): void {
  const { data, view, layers } = frame;
  const top = worldToScreen(view, w, h, 0, MAP_SIZE_M);
  const size = MAP_SIZE_M * view.zoom;
  startMap(ctx, w, h, view);
  if (layers.base) {
    ctx.fillStyle = '#d9ddce';
    ctx.fillRect(top.sx, top.sy, size, size);
    if (data.base) ctx.drawImage(data.base, top.sx, top.sy, size, size);
  }
  if (layers.grid) {
    const detail = gridDetail(view.zoom);
    const subdiv = 3 ** detail;
    ctx.beginPath();
    for (let i = 0; i <= GRID_N * subdiv; i++) {
      const offset = i * CELL_M * view.zoom / subdiv;
      ctx.moveTo(top.sx + offset, top.sy);
      ctx.lineTo(top.sx + offset, top.sy + size);
      ctx.moveTo(top.sx, top.sy + offset);
      ctx.lineTo(top.sx + size, top.sy + offset);
    }
    ctx.strokeStyle = '#566052';
    ctx.stroke();
    ctx.fillStyle = '#222';
    for (let row = 0; row < GRID_N; row++) for (let col = 0; col < GRID_N; col++) {
      ctx.fillText(formatGrid({ row, col, sub: [] }), top.sx + col * size / GRID_N + 4, top.sy + row * size / GRID_N + 10);
    }
  }
  ctx.restore();
}

function drawOverlays(ctx: CanvasRenderingContext2D, w: number, h: number, frame: MapFrame): void {
  const { data, view, mode, layers, selectedSpawnId, recommendedSpawnId, measure } = frame;
  const point = (p: MapPosition) => worldToScreen(view, w, h, p.x, p.z);
  startMap(ctx, w, h, view);
  if (layers.objectives) for (const objective of data.objectives ?? []) {
    const { sx, sy } = point(objective);
    ctx.fillStyle = objective.owner === 'friendly' ? '#1974ae' : objective.owner === 'enemy' ? '#a83930' : '#666';
    ctx.fillRect(sx - 7, sy - 7, 14, 14);
    ctx.fillStyle = '#222';
    ctx.fillText(`${objective.label} (${objective.owner})`, sx + 10, sy);
  }
  if (layers.units) for (const unit of data.units ?? []) {
    if (mode === 'deploy' && unit.relation === 'friendly') continue;
    const { sx, sy } = point(unit);
    ctx.save();
    ctx.translate(sx, sy);
    ctx.fillStyle = ctx.strokeStyle = unit.relation === 'squad' ? '#713999' : '#165483';
    if (unit.updatedAtS !== undefined && data.nowS !== undefined && unitStaleness(unit.updatedAtS, data.nowS).stale) ctx.setLineDash([2, 2]);
    if (unit.relation === 'self') {
      ctx.rotate(unit.yaw ?? 0);
      ctx.beginPath(); ctx.moveTo(0, -8); ctx.lineTo(5, 5); ctx.lineTo(-5, 5); ctx.closePath(); ctx.fill();
      ctx.beginPath(); ctx.arc(0, 0, 10, 0, 2 * Math.PI); ctx.stroke();
    } else {
      ctx.strokeRect(-5, -4, 10, 8);
      if (unit.relation === 'squad') { ctx.beginPath(); ctx.arc(0, 0, 8, 0, 2 * Math.PI); ctx.stroke(); }
      if (unit.downed) { ctx.fillStyle = '#a04700'; ctx.fillRect(-5, 6, 10, 2); }
      if (unit.label) ctx.fillText(unit.label, 10, 0);
    }
    ctx.restore();
  }
  if (layers.spawnPoints) for (const spawn of data.spawnPoints ?? []) {
    const { sx, sy } = point(spawn);
    ctx.fillStyle = spawn.status === 'available' ? '#285d36' : '#595959';
    ctx.fillRect(sx - 8, sy - 8, 16, 16);
    if (spawn.id === selectedSpawnId || spawn.id === recommendedSpawnId) {
      ctx.strokeStyle = '#222';
      ctx.setLineDash(spawn.id === selectedSpawnId ? [] : [3, 2]);
      ctx.strokeRect(sx - 10, sy - 10, 20, 20);
      ctx.setLineDash([]);
    }
    ctx.fillStyle = '#fff';
    ctx.fillText(SPAWN_GLYPH[spawn.status], sx - 4, sy);
    ctx.fillStyle = '#222';
    ctx.fillText(spawn.label, sx + 12, sy);
  }
  if (mode === 'full' && layers.markers) for (const marker of data.markers ?? []) {
    const remaining = data.nowS === undefined ? 1 : Math.min(1, markerRemaining(marker, data.nowS));
    if (remaining <= 0) continue;
    const { sx, sy } = point(marker);
    ctx.strokeStyle = '#863824';
    ctx.beginPath(); ctx.moveTo(sx, sy - 6); ctx.lineTo(sx + 6, sy); ctx.lineTo(sx, sy + 6); ctx.lineTo(sx - 6, sy); ctx.closePath(); ctx.stroke();
    ctx.beginPath(); ctx.arc(sx, sy, 9, -Math.PI / 2, -Math.PI / 2 + 2 * Math.PI * remaining); ctx.stroke();
  }
  if (measure) {
    const from = point(measure.from);
    const to = point(measure.to);
    ctx.strokeStyle = '#222';
    ctx.beginPath(); ctx.moveTo(from.sx, from.sy); ctx.lineTo(to.sx, to.sy); ctx.stroke();
  }
  ctx.restore();
}

/** Shared Canvas2D painter. Keep the static viewport cache out of the DOM and reuse it across overlay updates. */
export class MapRenderer {
  private readonly output: CanvasRenderingContext2D | null;
  private readonly base: CanvasRenderingContext2D | null;
  private baseKey?: { view: View; base: boolean; grid: boolean; image?: CanvasImageSource };

  constructor(canvas: HTMLCanvasElement) {
    this.output = canvas.getContext('2d');
    const cache = canvas.ownerDocument.createElement('canvas');
    cache.width = canvas.width;
    cache.height = canvas.height;
    this.base = cache.getContext('2d');
  }

  draw(frame: MapFrame): void {
    if (!this.output) return;
    const { width: w, height: h } = this.output.canvas;
    const { view, layers, data } = frame;
    this.output.clearRect(0, 0, w, h);
    if (this.base) {
      const prev = this.baseKey;
      if (!prev || prev.view.cx !== view.cx || prev.view.cz !== view.cz || prev.view.zoom !== view.zoom ||
          prev.base !== layers.base || prev.grid !== layers.grid || prev.image !== data.base) {
        this.base.clearRect(0, 0, w, h);
        drawBase(this.base, w, h, frame);
        this.baseKey = { view, base: layers.base, grid: layers.grid, image: data.base };
      }
      this.output.drawImage(this.base.canvas, 0, 0);
    } else drawBase(this.output, w, h, frame);
    drawOverlays(this.output, w, h, frame);
  }
}

export const positionGrid = ({ x, z }: MapPosition): string => formatGrid(worldToGrid(x, z));
