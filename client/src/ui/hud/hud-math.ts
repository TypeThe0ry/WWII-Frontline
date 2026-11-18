/** Pure HUD logic (M1-057a): ammo tier, compass, kill-feed queue, damage wedge, capture ring, FX controller, bands. */

/* ---- ammo (GAMEDESIGN §6.10): share of the initial loadout ammo, magazine + reserve ---- */
export type AmmoTier = 'full' | 'high' | 'mid' | 'low' | 'empty';
export function ammoTier(current: number, initial: number): AmmoTier {
  const r = initial <= 0 ? 0 : current / initial;
  return r >= 1 ? 'full' : r >= 0.75 ? 'high' : r >= 0.5 ? 'mid' : r >= 0.25 ? 'low' : 'empty';
}
/** Exact numbers only next to a supply crate or inside the loadout view. */
export const showExactAmmo = (nearCrate: boolean, inLoadout: boolean): boolean => nearCrate || inLoadout;
export const ammoWarn = (t: AmmoTier): boolean => t === 'low' || t === 'empty';
/** Empty magazine (separate from the tier): triggers the outline flash. */
export const magEmpty = (inMag: number): boolean => inMag <= 0;

/* ---- compass ---- */
export const COMPASS_LABELS_EN = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'] as const;
export const COMPASS_LABELS_ZH = ['北', '东北', '东', '东南', '南', '西南', '西', '西北'] as const;
export function compassLabel(deg: number, zh: boolean): string {
  const i = Math.round((((deg % 360) + 360) % 360) / 45) % 8;
  return (zh ? COMPASS_LABELS_ZH : COMPASS_LABELS_EN)[i]!;
}
/** Heading (yaw rad; 0 = +z north, +x east → clockwise-positive compass) to degrees 0..360. */
export function headingDeg(yawRad: number): number {
  return ((((yawRad * 180) / Math.PI) % 360) + 360) % 360;
}
/** Single translateX for the tape: px offset so `heading` is centred; the tape repeats every 360°. */
export function compassOffsetPx(headingDegrees: number, pxPerDeg: number): number {
  return -(((headingDegrees % 360) + 360) % 360) * pxPerDeg;
}
/** Marker offset in px relative to the tape centre for a bearing, wrapped to ±180°. */
export function markerOffsetPx(bearingDeg: number, headingDegrees: number, pxPerDeg: number): number {
  let d = (bearingDeg - headingDegrees) % 360;
  if (d > 180) d -= 360;
  else if (d < -180) d += 360;
  return d * pxPerDeg;
}

/* ---- kill feed: ≤ 3 visible, 5 s each, the rest wait ---- */
export const KILLFEED_MAX = 3;
export const KILLFEED_MS = 5000;
export interface FeedItem<T> {
  item: T;
  shownAt: number;
}
export class KillFeedQueue<T> {
  readonly visible: FeedItem<T>[] = [];
  private readonly waiting: T[] = [];
  push(item: T, now: number): void {
    if (this.visible.length < KILLFEED_MAX) this.visible.push({ item, shownAt: now });
    else this.waiting.push(item);
  }
  /** Expires visible entries older than 5 s and promotes waiting ones; returns true when the view changed. */
  update(now: number): boolean {
    let changed = false;
    for (let i = this.visible.length - 1; i >= 0; i--) {
      if (now - this.visible[i]!.shownAt >= KILLFEED_MS) {
        this.visible.splice(i, 1);
        changed = true;
      }
    }
    while (this.visible.length < KILLFEED_MAX && this.waiting.length) {
      this.visible.push({ item: this.waiting.shift()!, shownAt: now });
      changed = true;
    }
    return changed;
  }
  get pending(): number {
    return this.waiting.length;
  }
}

/* ---- damage wedge: angle relative to the camera, 600 ms fade ---- */
export const WEDGE_MS = 600;
/** Angle (rad, 0 = ahead, clockwise) of a source at (sx,sz) as seen from (px,pz) facing `yaw`. */
export function wedgeAngle(px: number, pz: number, sx: number, sz: number, yaw: number): number {
  const bearing = Math.atan2(sx - px, sz - pz); // 0 = +z, clockwise towards +x
  const a = bearing - yaw;
  return ((a % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
}
export const wedgeStrength = (amount: number): number => Math.min(1, Math.max(0.25, amount / 50));
export const wedgeOpacity = (ageMs: number, strength: number): number => (ageMs >= WEDGE_MS ? 0 : strength * (1 - ageMs / WEDGE_MS));

/* ---- capture ring: Hidden / Visible / Contested / Captured ---- */
export type CaptureState = 'hidden' | 'visible' | 'contested' | 'captured';
export function captureState(inZone: boolean, friendly: number, enemy: number, owner: 'friend' | 'enemy' | 'neutral', progress: number): CaptureState {
  if (!inZone) return 'hidden';
  if (friendly > 0 && enemy > 0) return 'contested';
  if (owner === 'friend' && progress >= 1) return 'captured';
  return 'visible';
}
/** Three ring segments (friend/neutral/enemy) as fractions summing to 1. `progress` −1 (enemy) … +1 (friend). */
export function ringSegments(progress: number): { friend: number; neutral: number; enemy: number } {
  const p = Math.max(-1, Math.min(1, progress));
  return { friend: Math.max(0, p), neutral: 1 - Math.abs(p), enemy: Math.max(0, -p) };
}

/* ---- bands ---- */
export const staminaWarn = (s: number): boolean => s <= 0.2;
/** 0 healthy, 1 hurt, 2 badly hurt, 3 critical (no numbers shown). */
export function healthBand(hpFrac: number): 0 | 1 | 2 | 3 {
  return hpFrac > 0.75 ? 0 : hpFrac > 0.5 ? 1 : hpFrac > 0.25 ? 2 : 3;
}

/* ---- FX controller (UI_SPEC §4.6.3): clear / suppressed / bleeding / downed, priority downed > bleeding > suppressed ---- */
export type FxState = 'clear' | 'suppressed' | 'bleeding' | 'downed';
export const SUPPRESS_ON = 0.15;
export const SUPPRESS_OFF = 0.05; // hysteresis
export function fxState(prev: FxState, o: { downed: boolean; hpFrac: number; suppression: number }): FxState {
  if (o.downed) return 'downed';
  if (o.hpFrac <= 0.25) return 'bleeding';
  const th = prev === 'suppressed' ? SUPPRESS_OFF : SUPPRESS_ON;
  return o.suppression > th ? 'suppressed' : 'clear';
}
/** Vignette strength 0..1 for the state (static alternative when reduce-motion is on). */
export function fxVignette(s: FxState, suppression: number, hpFrac: number, reduceMotion: boolean): number {
  const v = s === 'downed' ? 1 : s === 'bleeding' ? 0.6 + 0.4 * (1 - hpFrac / 0.25) : s === 'suppressed' ? Math.min(1, suppression) * 0.8 : 0;
  return reduceMotion ? Math.round(v * 4) / 4 : v;
}

/* ---- squad list ---- */
export const SQUAD_ROWS = 8;
export const TEAM_STALE_MS = 5000;
export const showExactDistance = (lastTeamUpdateMs: number, nowMs: number): boolean => nowMs - lastTeamUpdateMs <= TEAM_STALE_MS;
