/** Gunshot layer crossfade and speed-of-sound delay (ARCHITECTURE §12.2). All thresholds come from `/data/audio/params.json`. */
import params from '../../../data/audio/params.json';

export interface AudioParams {
  layer_near_fade_start_m: number;
  layer_near_fade_end_m: number;
  layer_mid_in_start_m: number;
  layer_mid_in_end_m: number;
  layer_mid_out_start_m: number;
  layer_mid_out_end_m: number;
  layer_far_in_start_m: number;
  layer_far_peak_m: number;
  speed_of_sound_mps: number;
  shot_delay_min_distance_m: number;
  shot_delay_max_s: number;
}
export const AUDIO_PARAMS: AudioParams = params;

const clamp01 = (v: number): number => Math.min(1, Math.max(0, v));
/** Linear ramp from 0 at `a` to 1 at `b`. */
const ramp = (d: number, a: number, b: number): number => clamp01((d - a) / (b - a));

export interface LayerGains {
  near: number;
  mid: number;
  far: number;
}

export function layerGains(d: number, p: AudioParams = AUDIO_PARAMS): LayerGains {
  const dist = Math.max(0, d);
  // near = clamp(1 − (d−30)/60)
  const near = clamp01(1 - (dist - p.layer_near_fade_start_m) / (p.layer_near_fade_end_m - p.layer_near_fade_start_m));
  // mid: in 30–60, out 200–300
  const mid = ramp(dist, p.layer_mid_in_start_m, p.layer_mid_in_end_m) * (1 - ramp(dist, p.layer_mid_out_start_m, p.layer_mid_out_end_m));
  // far: in 200→600 (peak), then natural attenuation ∝ 1/d
  const far = dist <= p.layer_far_peak_m ? ramp(dist, p.layer_far_in_start_m, p.layer_far_peak_m) : p.layer_far_peak_m / dist;
  return { near, mid, far };
}

/** `d ≤ 150` → 0 (no extra delay for close shots), otherwise `min(d/343, 3)`. */
export function shotDelaySeconds(d: number, p: AudioParams = AUDIO_PARAMS): number {
  if (d <= p.shot_delay_min_distance_m) return 0;
  return Math.min(d / p.speed_of_sound_mps, p.shot_delay_max_s);
}

/** Variant picker that never repeats the previous index (needs ≥ 2 variants to avoid repeats). */
export function pickVariant(count: number, last: number | undefined, rnd: () => number): number {
  if (count <= 1) return 0;
  let i = Math.min(count - 1, Math.floor(rnd() * count));
  if (i === last) i = (i + 1 + Math.floor(rnd() * (count - 1))) % count;
  return i;
}

/** A missing layer is silent: gains for absent layers are zero and nothing substitutes for them. */
export function applyAvailability(g: LayerGains, has: { near: boolean; mid: boolean; far: boolean }): LayerGains {
  return { near: has.near ? g.near : 0, mid: has.mid ? g.mid : 0, far: has.far ? g.far : 0 };
}

/* ---- suppression-lite (M1 version): low-pass + duck, recover within 400 ms ---- */
export interface SuppressFilter {
  cutoffHz: number;
  gainDb: number;
}
export const SUPPRESS_BASE_HZ = 20000;
export const SUPPRESS_MIN_HZ = 1200;
export const SUPPRESS_MAX_DB = -9;
/** `intensity` 0..255 as sent in the `suppress` event; `t` ms since the event; `recoverMs` from params. */
export function suppressFilter(intensity: number, t: number, recoverMs = 400): SuppressFilter {
  const k = Math.min(1, Math.max(0, intensity / 255));
  const r = Math.min(1, Math.max(0, t / recoverMs)); // 0 = just hit, 1 = recovered
  const f = k * (1 - r);
  // geometric interpolation of cutoff so the sweep sounds even
  const cutoffHz = SUPPRESS_BASE_HZ * Math.pow(SUPPRESS_MIN_HZ / SUPPRESS_BASE_HZ, f);
  return { cutoffHz, gainDb: SUPPRESS_MAX_DB * f };
}

/* ---- footsteps ---- */
export type Surface = 'mud' | 'grass' | 'stone' | 'gravel' | 'wood' | 'sand';
export type Gait = 'walk' | 'run' | 'crouch' | 'land';
export function footstepAudibleM(g: Gait, p: { footstep_audible_walk_m: number; footstep_audible_run_m: number; footstep_audible_crouch_m: number }): number {
  return g === 'run' || g === 'land' ? p.footstep_audible_run_m : g === 'crouch' ? p.footstep_audible_crouch_m : p.footstep_audible_walk_m;
}
/** Voice-limited footsteps: keep the nearest `maxSlots`, farther ones are dropped (merged into the ambient bed). */
export function limitFootsteps<T extends { distM: number }>(steps: readonly T[], maxSlots: number): T[] {
  return [...steps].sort((a, b) => a.distM - b.distM).slice(0, maxSlots);
}
