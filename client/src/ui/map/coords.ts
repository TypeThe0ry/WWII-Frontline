/** Tactical-map grid (UI_SPEC §4.7.4). World frame: x east, z north, origin at the SW corner of the playable square. */
export const MAP_SIZE_M = 2048;
export const GRID_N = 10;
export const CELL_M = MAP_SIZE_M / GRID_N; // 204.8

/** Keypad layout: 7 8 9 / 4 5 6 / 1 2 3. */
const KEYPAD = [
  [7, 8, 9],
  [4, 5, 6],
  [1, 2, 3],
] as const;

export interface GridRef {
  col: number; // 0..9 (A..J), west → east
  row: number; // 0..9 (1..10), north → south
  sub: number[]; // keypad digits per subdivision level
}

const clampIdx = (v: number): number => Math.min(GRID_N - 1, Math.max(0, v));

export function worldToGrid(x: number, z: number, levels = 0, size = MAP_SIZE_M): GridRef {
  const cell = size / GRID_N;
  const fx = Math.min(size - 1e-9, Math.max(0, x)) / cell;
  const fy = (size - Math.min(size, Math.max(1e-9, z))) / cell; // row grows southwards
  const col = clampIdx(Math.floor(fx));
  const row = clampIdx(Math.floor(fy));
  let u = fx - col;
  let v = fy - row;
  const sub: number[] = [];
  for (let l = 0; l < levels; l++) {
    const cx = Math.min(2, Math.floor(u * 3));
    const cy = Math.min(2, Math.floor(v * 3));
    sub.push(KEYPAD[cy]![cx]!);
    u = u * 3 - cx;
    v = v * 3 - cy;
  }
  return { col, row, sub };
}

/** Centre of the referenced cell (world metres). */
export function gridToWorld(g: GridRef, size = MAP_SIZE_M): { x: number; z: number } {
  const cell = size / GRID_N;
  let x0 = g.col * cell;
  let y0 = g.row * cell; // measured from the north edge
  let w = cell;
  for (const d of g.sub) {
    w /= 3;
    let cx = 0;
    let cy = 0;
    KEYPAD.forEach((r, j) => r.forEach((k, i) => { if (k === d) { cx = i; cy = j; } }));
    x0 += cx * w;
    y0 += cy * w;
  }
  return { x: x0 + w / 2, z: size - (y0 + w / 2) };
}

export function formatGrid(g: GridRef): string {
  return `${String.fromCharCode(65 + g.col)}${g.row + 1}${g.sub.map((d) => `-${d}`).join('')}`;
}

export function parseGrid(s: string): GridRef | null {
  const m = /^([A-J])(10|[1-9])((?:-[1-9])*)$/.exec(s.trim().toUpperCase());
  if (!m) return null;
  return { col: m[1]!.charCodeAt(0) - 65, row: Number(m[2]) - 1, sub: m[3] ? m[3].slice(1).split('-').map(Number) : [] };
}

/* ---- view transform: zoom about the cursor, pan clamp (pure) ---- */
export interface View {
  zoom: number; // screen px per world metre
  cx: number; // world x at the viewport centre
  cz: number;
}
export const ZOOM_MIN_FACTOR = 1;
export const ZOOM_MAX_FACTOR = 8;

export function fitZoom(viewW: number, viewH: number, size = MAP_SIZE_M): number {
  return Math.min(viewW, viewH) / size;
}
export function worldToScreen(v: View, vw: number, vh: number, x: number, z: number): { sx: number; sy: number } {
  return { sx: vw / 2 + (x - v.cx) * v.zoom, sy: vh / 2 - (z - v.cz) * v.zoom };
}
export function screenToWorld(v: View, vw: number, vh: number, sx: number, sy: number): { x: number; z: number } {
  return { x: v.cx + (sx - vw / 2) / v.zoom, z: v.cz - (sy - vh / 2) / v.zoom };
}
export function clampView(v: View, vw: number, vh: number, size = MAP_SIZE_M): View {
  const base = fitZoom(vw, vh, size);
  const zoom = Math.min(base * ZOOM_MAX_FACTOR, Math.max(base * ZOOM_MIN_FACTOR, v.zoom));
  const halfW = vw / 2 / zoom;
  const halfH = vh / 2 / zoom;
  const clampAxis = (c: number, half: number) => (half * 2 >= size ? size / 2 : Math.min(size - half, Math.max(half, c)));
  return { zoom, cx: clampAxis(v.cx, halfW), cz: clampAxis(v.cz, halfH) };
}
/** Wheel zoom keeping the world point under the cursor fixed. */
export function zoomAt(v: View, vw: number, vh: number, sx: number, sy: number, factor: number, size = MAP_SIZE_M): View {
  const before = screenToWorld(v, vw, vh, sx, sy);
  const zoom = v.zoom * factor;
  const next: View = { zoom, cx: before.x - (sx - vw / 2) / zoom, cz: before.z + (sy - vh / 2) / zoom };
  return clampView(next, vw, vh, size);
}
/** Subdivision grid lines shown from this zoom: level 1 when a cell is ≥ 240 px, level 2 when a level-1 cell is ≥ 120 px. */
export function gridDetail(zoom: number): 0 | 1 | 2 {
  const cellPx = CELL_M * zoom;
  return cellPx >= 720 ? 2 : cellPx >= 240 ? 1 : 0;
}
