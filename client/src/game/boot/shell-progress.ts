import { signal } from '@preact/signals';

/** Throttles a progress stream (fetch bytes) to at most one update per `intervalMs` (10 Hz = 100 ms), always passing the final value. */
export function createThrottle(emit: (v: number) => void, intervalMs: number, now: () => number): { push(v: number, final?: boolean): void } {
  let last = -Infinity;
  return {
    push(v, final = false) {
      const t = now();
      if (final || t - last >= intervalMs) {
        last = t;
        emit(v);
      }
    },
  };
}

export const shellProgress = signal(0); // 0..1
