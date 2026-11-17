/** Sliding-window sample statistics (default 600 frames, ARCHITECTURE §13.5.1 / M0-020). */
export const STATS_WINDOW = 600;

export interface WindowStats {
  count: number;
  mean: number;
  p50: number;
  p99: number;
  max: number;
}

export class RingStats {
  private readonly buf: Float64Array;
  private n = 0;
  private head = 0;

  constructor(readonly capacity: number = STATS_WINDOW) {
    this.buf = new Float64Array(capacity);
  }

  push(v: number): void {
    if (!Number.isFinite(v)) return;
    this.buf[this.head] = v;
    this.head = (this.head + 1) % this.capacity;
    if (this.n < this.capacity) this.n++;
  }

  get count(): number {
    return this.n;
  }

  /** Most recent sample, or NaN if empty. */
  get last(): number {
    if (this.n === 0) return NaN;
    return this.buf[(this.head - 1 + this.capacity) % this.capacity] ?? NaN;
  }

  clear(): void {
    this.n = 0;
    this.head = 0;
  }

  stats(): WindowStats {
    if (this.n === 0) return { count: 0, mean: 0, p50: 0, p99: 0, max: 0 };
    const a = Array.from(this.buf.subarray(0, this.n)).sort((x, y) => x - y);
    let sum = 0;
    for (const v of a) sum += v;
    return { count: this.n, mean: sum / this.n, p50: percentile(a, 0.5), p99: percentile(a, 0.99), max: a[a.length - 1] ?? 0 };
  }
}

/** Nearest-rank percentile on an ascending-sorted array. */
export function percentile(sorted: readonly number[], q: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.ceil(q * sorted.length) - 1));
  return sorted[idx] ?? 0;
}

/** Per-frame CPU / interval tracker. */
export class FrameStats {
  readonly cpu = new RingStats();
  readonly interval = new RingStats();
  frames = 0;
  private lastStart = NaN;

  /** Call at the start of a frame with performance.now(). */
  begin(now: number): number {
    const dt = Number.isNaN(this.lastStart) ? 0 : now - this.lastStart;
    this.lastStart = now;
    if (dt > 0) this.interval.push(dt);
    return dt;
  }

  /** Call at the end of the frame's CPU work; cpuMs = JS time spent in the frame callback. */
  end(cpuMs: number): void {
    this.cpu.push(cpuMs);
    this.frames++;
  }

  /** Forget everything measured so far (the perf capture resets after its warm-up). */
  reset(): void {
    this.cpu.clear();
    this.interval.clear();
    this.frames = 0;
    this.lastStart = NaN;
  }

  fps(): number {
    const s = this.interval.stats();
    return s.mean > 0 ? 1000 / s.mean : 0;
  }
}
