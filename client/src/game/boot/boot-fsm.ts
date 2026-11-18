/** Boot state machine (UI_SPEC §4.1.3): probing → shell_loading → ready | unsupported | failed, and map phases. */
export type BootPhase = 'probing' | 'shell_loading' | 'ready' | 'unsupported' | 'failed';
export type MapPhase = 'idle' | 'check_cache' | 'downloading' | 'compiling' | 'warmup' | 'done';
export type BootEvent =
  | { t: 'probed'; backend: 'webgpu' | 'webgl2' | 'none' }
  | { t: 'shell_ok' }
  | { t: 'shell_fail' }
  | { t: 'retry' };

export function bootNext(p: BootPhase, e: BootEvent): BootPhase {
  switch (p) {
    case 'probing':
      return e.t === 'probed' ? (e.backend === 'none' ? 'unsupported' : 'shell_loading') : p;
    case 'shell_loading':
      return e.t === 'shell_ok' ? 'ready' : e.t === 'shell_fail' ? 'failed' : p;
    case 'failed':
      return e.t === 'retry' ? 'shell_loading' : p;
    default:
      return p; // ready / unsupported are terminal (no "force continue")
  }
}

const MAP_ORDER: readonly MapPhase[] = ['idle', 'check_cache', 'downloading', 'compiling', 'warmup', 'done'];

/** Map phases only move forward, one step at a time; cache hit may skip `downloading`. */
export function mapNext(p: MapPhase, cacheHit = false): MapPhase {
  if (p === 'check_cache' && cacheHit) return 'compiling';
  const i = MAP_ORDER.indexOf(p);
  return MAP_ORDER[Math.min(MAP_ORDER.length - 1, i + 1)]!;
}

/** Deploy is only allowed once shader warm-up has finished (behaviour 3). */
export const canEnterDeploy = (m: MapPhase): boolean => m === 'done';

/** Backoff 1/2/4/8 s, then stays at 8 s. */
export function backoffMs(attempt: number): number {
  return 1000 * 2 ** Math.min(3, Math.max(0, attempt));
}

/** Retry helper; `sleep` injectable for tests. Returns after success or when `maxAttempts` is exhausted. */
export async function withRetry<T>(
  fn: () => Promise<T>,
  opts: { maxAttempts: number; sleep: (ms: number) => Promise<void>; onFail?: (attempt: number, err: unknown) => void },
): Promise<T> {
  let last: unknown;
  for (let i = 0; i < opts.maxAttempts; i++) {
    try {
      return await fn();
    } catch (err) {
      last = err;
      opts.onFail?.(i, err);
      if (i < opts.maxAttempts - 1) await opts.sleep(backoffMs(i));
    }
  }
  throw last;
}
