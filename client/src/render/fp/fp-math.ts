/** First-person weapon maths (M1-046, GAMEDESIGN 6.10); the pass and animator call these. */

export const WALL_MARGIN_M = 0.05;

/**
 * How far to pull the weapon back when the wall is `wallDistM` ahead of the camera and the weapon
 * reaches `gunLenM` forward. The muzzle ends up `WALL_MARGIN_M` short of the wall; the pushback never
 * exceeds the gun length (at a wall closer than the margin the margin cannot be kept).
 */
export function wallPushback(wallDistM: number, gunLenM: number, marginM = WALL_MARGIN_M): number {
  return Math.min(Math.max(gunLenM + marginM - wallDistM, 0), gunLenM);
}

/** Muzzle distance in front of the camera after pushback. */
export const muzzleReach = (wallDistM: number, gunLenM: number): number => gunLenM - wallPushback(wallDistM, gunLenM);

/** Vertical FOV (deg) while aiming: the FP FOV is divided by the ADS magnification (GD 6.10: 1.2x). */
export function adsFovDeg(baseFovDeg: number, mult: number, adsT: number): number {
  const t = Math.min(Math.max(adsT, 0), 1);
  const base = Math.tan((baseFovDeg * Math.PI) / 360);
  const aimed = base / mult;
  return (2 * Math.atan(base + (aimed - base) * t) * 180) / Math.PI;
}

/** Moves the ADS blend toward 1 (aiming) or 0 so the full travel takes `adsMs`. */
export function stepAds(adsT: number, dtS: number, adsMs: number, aiming: boolean): number {
  const d = adsMs <= 0 ? 1 : dtS / (adsMs / 1000);
  return Math.min(1, Math.max(0, adsT + (aiming ? d : -d)));
}

/** Screen height fraction a point `distM` ahead with height `hM` covers at vertical FOV `fovDeg`. */
export const screenFraction = (hM: number, distM: number, fovDeg: number): number => hM / (2 * distM * Math.tan((fovDeg * Math.PI) / 360));

/** Reload visual length in rendered frames; within one frame of `reload_ms`. */
export const reloadFrames = (reloadMs: number, fps: number): number => Math.round((reloadMs * fps) / 1000);

/** Critically damped spring for the visual recoil kick (position along the weapon axis, metres). */
export class RecoilSpring {
  pos = 0;
  vel = 0;
  constructor(readonly omega = 28) {}

  kick(impulseMps: number): void {
    this.vel += impulseMps;
  }

  update(dtS: number): number {
    const w = this.omega;
    // Semi-implicit critically damped spring toward 0.
    const acc = -2 * w * this.vel - w * w * this.pos;
    this.vel += acc * dtS;
    this.pos += this.vel * dtS;
    return this.pos;
  }
}

/** Walk bob offset (x sideways, y up) from stride phase and speed fraction 0..1. */
export function walkBob(phaseRad: number, speed01: number, ampM = 0.012): [number, number] {
  const a = ampM * Math.min(Math.max(speed01, 0), 1);
  return [Math.sin(phaseRad) * a, Math.abs(Math.sin(phaseRad * 2)) * a * 0.6];
}
