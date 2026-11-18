/**
 * Remote vehicle sampling helpers (M1-041, ARCHITECTURE 6.5.2): render 100 ms behind, and when the
 * buffer runs dry extrapolate with the 4 B velocity the snapshot carries, for at most 150 ms.
 */
import { MAX_EXTRAPOLATION_MS } from './interpolation';

/** Velocity-aided extrapolation of one axis; `ageMs` = time past the last sample. */
export function extrapolate(pos: number, velMps: number, ageMs: number): number {
  const t = Math.min(Math.max(ageMs, 0), MAX_EXTRAPOLATION_MS) / 1000;
  return pos + velMps * t;
}

/** RMS of the second difference of a sampled path (per-axis), a jitter measure (metres). */
export function secondDiffRms(samples: readonly number[]): number {
  if (samples.length < 3) return 0;
  let acc = 0;
  for (let i = 2; i < samples.length; i++) {
    const d = (samples[i] as number) - 2 * (samples[i - 1] as number) + (samples[i - 2] as number);
    acc += d * d;
  }
  return Math.sqrt(acc / (samples.length - 2));
}
