/** Tank rendering maths (M1-048, GAMEDESIGN 14.1/14.3): track speeds, turret smoothing, socket list. */

/**
 * Sockets of the tank. The vehicle data file (`gameplay.sockets`) is the source of truth for the names (PI-112, resolved):
 * `socket_gun_muzzle`, `socket_turret`, `socket_hatch` come from /data; the rest have no data counterpart yet and keep the task-card names.
 */
export const TANK_SOCKETS = [
  'socket_gun_muzzle',
  'socket_turret',
  'socket_hatch',
  'socket_seat_driver',
  'socket_seat_gunner',
  'socket_seat_commander',
  'socket_exhaust',
  'socket_track_l',
  'socket_track_r',
  'socket_mg_coax',
] as const;

/** Task-card names that map onto a data name; kept as read-only aliases until every caller has moved over. */
export const TANK_SOCKET_ALIASES: Readonly<Record<string, (typeof TANK_SOCKETS)[number]>> = { socket_muzzle: 'socket_gun_muzzle' };

export interface TrackSpeeds {
  left: number;
  right: number;
}

/**
 * Linear speed of each track (m/s) from the hull's forward speed and yaw rate (rad/s, +ve = turning
 * right). `gaugeM` = distance between the track centrelines (hull width minus the track width).
 */
export function trackSpeeds(forwardMps: number, yawRateRadS: number, gaugeM: number): TrackSpeeds {
  const d = (yawRateRadS * gaugeM) / 2;
  return { left: forwardMps + d, right: forwardMps - d };
}

/** UV scroll rate (texture repeats per second) of a track with period `pitchM` metres of belt. */
export const trackUvRate = (trackMps: number, repeatM: number): number => trackMps / repeatM;

/** Drive-wheel rotation (rad/s) for a track speed. */
export const wheelOmega = (trackMps: number, wheelRadiusM: number): number => trackMps / wheelRadiusM;

const TAU = Math.PI * 2;
/** Signed shortest angular difference `to - from`, in (-PI, PI]. */
export function angleDiff(from: number, to: number): number {
  const d = (((to - from + Math.PI) % TAU) + TAU) % TAU;
  return d === 0 && to !== from ? Math.PI : d - Math.PI;
}

/**
 * Moves a displayed turret angle toward the authoritative one: rotates along the shortest arc, at
 * most `maxRateRadS` (the hull's traverse rate) -- the displayed turret therefore never spins the
 * long way and never outruns the real mechanism.
 */
export function followAngle(shown: number, target: number, dtS: number, maxRateRadS: number): number {
  const d = angleDiff(shown, target);
  const step = maxRateRadS * dtS;
  return Math.abs(d) <= step ? target : shown + Math.sign(d) * step;
}

/** Gun pitch clamp to the data range [min, max] in degrees. */
export const clampGunPitch = (deg: number, range: readonly [number, number]): number => Math.min(Math.max(deg, range[0]), range[1]);

export interface PlaceholderDims {
  length: number;
  width: number;
  height: number;
}

/** Placeholder hull box from `historical.dimensions_m` (the task's 1 % size check compares these). */
export function hullBox(dims: PlaceholderDims): { size: [number, number, number]; halfHeight: number } {
  return { size: [dims.width, dims.height, dims.length], halfHeight: dims.height / 2 };
}
