/**
 * Wind (M0-024): `sway = amp x sin(t x w + phase(instance)) x heightFactor`, evaluated in the vertex shader
 * (grass-material.*); this module owns the uniform block and a CPU twin of the formula for tests.
 * `uWind` = (dirX, dirZ, strength, time): xz displacement direction (unit), amplitude in metres at
 * heightFactor 1 and instance scale 1, and the wind clock in seconds. Trunks have heightFactor 0 (vertex-colour
 * alpha of the placeholder meshes), crowns/blades > 0.
 */

export interface WindState {
  /** Unit direction in the xz plane. */
  dirX: number;
  dirZ: number;
  /** Peak displacement in metres at heightFactor = 1 and scale = 1. */
  strength: number;
  /** Seconds. */
  time: number;
}

/** Sway angular frequency (rad/s): ~0.5 Hz, a slow breeze. */
export const SWAY_OMEGA = 3.1;
export const DEFAULT_WIND_STRENGTH = 0.18;

export function defaultWind(): WindState {
  const a = Math.PI * 0.3;
  return { dirX: Math.cos(a), dirZ: Math.sin(a), strength: DEFAULT_WIND_STRENGTH, time: 0 };
}

/** Per-instance phase in [0, 2 pi): same hash as the shader (fract(sin(dot(xz, (12.9898, 78.233))) * 43758.5453)). */
export function instancePhase(x: number, z: number): number {
  const s = Math.sin(x * 12.9898 + z * 78.233) * 43758.5453;
  return (s - Math.floor(s)) * Math.PI * 2;
}

/** Horizontal displacement (m, along the wind direction) of a vertex with sway weight `heightFactor` in [0,1]. */
export function windSway(time: number, phase: number, heightFactor: number, strength: number, scale = 1): number {
  const w = Math.min(1, Math.max(0, heightFactor));
  return strength * w * scale * Math.sin(time * SWAY_OMEGA + phase);
}

export function windUniform(w: WindState, out: Float32Array = new Float32Array(4)): Float32Array {
  out[0] = w.dirX;
  out[1] = w.dirZ;
  out[2] = w.strength;
  out[3] = w.time;
  return out;
}
