import type { WindowStats } from './frame-stats';

export const PERF_SCHEMA_VERSION = 2;

export interface VegetationPerf {
  drawCalls: number;
  instances: number;
  tris: number;
}

/** Fixed-field JSON returned by `window.__perf.dump()` (consumed by M0-030). */
export interface PerfDump {
  schema: number;
  timestamp_ms: number;
  renderer: 'webgpu' | 'webgl2';
  fallback_reason: string | null;
  frames: number;
  fps: number;
  cpu_ms: WindowStats;
  interval_ms: WindowStats;
  gpu_ms: { value: number | null; source: 'timestamp-query' | 'disjoint-timer-query' | 'interval-estimate' | 'n/a'; reason: string | null };
  draw_calls: number;
  active_meshes: number;
  triangles: number;
  render_scale: number;
  resolution: { width: number; height: number };
  sim: { steps_total: number; stepper_clamped: number };
  /** CPU ms per frame stage (M0-030): each `onFrame` callback by label, `scene_render`, and Babylon's split of it. */
  stages: Record<string, WindowStats>;
  /** Vegetation meshes that passed culling in the last frame (M0-024); zeros when vegetation is not attached. */
  vegetation: VegetationPerf;
  slots: { rtt: string; transport: string; corrections: string };
}

export const PERF_DUMP_KEYS: readonly (keyof PerfDump)[] = [
  'schema',
  'timestamp_ms',
  'renderer',
  'fallback_reason',
  'frames',
  'fps',
  'cpu_ms',
  'interval_ms',
  'gpu_ms',
  'draw_calls',
  'active_meshes',
  'triangles',
  'render_scale',
  'resolution',
  'sim',
  'stages',
  'vegetation',
  'slots',
];

export const WINDOW_STATS_KEYS: readonly (keyof WindowStats)[] = ['count', 'mean', 'p50', 'p99', 'max'];

export interface PerfApi {
  dump(): PerfDump;
  /** Frame counter, handy for e2e waits. */
  frames(): number;
  /** Clears the statistics windows (M0-030: capture resets after the warm-up). */
  reset?(): void;
  /** M0-025: engine frame id at which each `fire.*` event was emitted / its muzzle flash became visible (same order, last 512). */
  fireEventFrame?: number[];
  muzzleFlashVisibleFrame?: number[];
}

/** Installs `window.__perf`. `getDump` is supplied by the composition root. */
export function installPerfApi(target: { __perf?: PerfApi }, getDump: () => PerfDump, reset?: () => void): void {
  target.__perf = { dump: getDump, frames: () => getDump().frames, ...(reset ? { reset } : {}) };
}

/** Render-side numbers gathered by the render module; sim numbers are added by the composition root. */
export interface RenderSnapshot {
  renderer: 'webgpu' | 'webgl2';
  fallbackReason: string | null;
  frames: number;
  fps: number;
  cpu: WindowStats;
  interval: WindowStats;
  gpu: PerfDump['gpu_ms'];
  drawCalls: number;
  activeMeshes: number;
  triangles: number;
  renderScale: number;
  width: number;
  height: number;
  /** See `PerfDump.stages`; optional so that test fixtures stay small. */
  stages?: Record<string, WindowStats>;
}

export function buildPerfDump(
  r: RenderSnapshot,
  sim: PerfDump['sim'],
  now: number = Date.now(),
  slots: PerfDump['slots'] = { rtt: 'n/a', transport: 'n/a', corrections: 'n/a' },
  vegetation: VegetationPerf = { drawCalls: 0, instances: 0, tris: 0 },
): PerfDump {
  return {
    schema: PERF_SCHEMA_VERSION,
    timestamp_ms: now,
    renderer: r.renderer,
    fallback_reason: r.fallbackReason,
    frames: r.frames,
    fps: r.fps,
    cpu_ms: r.cpu,
    interval_ms: r.interval,
    gpu_ms: r.gpu,
    draw_calls: r.drawCalls,
    active_meshes: r.activeMeshes,
    triangles: r.triangles,
    render_scale: r.renderScale,
    resolution: { width: r.width, height: r.height },
    sim,
    stages: r.stages ?? {},
    vegetation,
    slots,
  };
}
