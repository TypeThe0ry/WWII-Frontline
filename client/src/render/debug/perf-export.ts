import type { WindowStats } from './frame-stats';

export const PERF_SCHEMA_VERSION = 1;

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
  'slots',
];

export const WINDOW_STATS_KEYS: readonly (keyof WindowStats)[] = ['count', 'mean', 'p50', 'p99', 'max'];

export interface PerfApi {
  dump(): PerfDump;
  /** Frame counter, handy for e2e waits. */
  frames(): number;
}

/** Installs `window.__perf`. `getDump` is supplied by the composition root. */
export function installPerfApi(target: { __perf?: PerfApi }, getDump: () => PerfDump): void {
  target.__perf = { dump: getDump, frames: () => getDump().frames };
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
}

export function buildPerfDump(
  r: RenderSnapshot,
  sim: PerfDump['sim'],
  now: number = Date.now(),
  slots: PerfDump['slots'] = { rtt: 'n/a', transport: 'n/a', corrections: 'n/a' },
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
    slots,
  };
}
