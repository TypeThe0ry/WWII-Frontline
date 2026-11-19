/**
 * First-person view (M0-025): puts the scene camera at the locally predicted pose and carries the grey-box weapon.
 *  - position: `local.pos` (feet) + eye height of the current stance (eased over the role's `stance_transition_ms`),
 *  - orientation: the player's yaw / pitch (Babylon: rotation.x = -pitch, rotation.y = yaw),
 *  - weapon: hip / ADS offsets and the ADS FOV multiplier from data/placeholders/weapon_models.json; kicked back and up
 *    by the predicted recoil (`recoilPitch/Yaw`, degrees) and swayed slightly while moving; ADS takes the weapon's
 *    `ads_ms` (data) to blend. Hip FOV is data too (`view.hip_fov_deg`).
 */
import type { TargetCamera } from '@babylonjs/core/Cameras/targetCamera';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { ActorData, WeaponActorDef } from '../actors/actor-data';
import type { WeaponNode } from '../actors/placeholder-weapon';
import { freshWorldMatrix } from '../actors/world';
import { RecoilSpring, stepAds, wallPushback } from '../fp/fp-math';

export interface FpLocal {
  pos: ArrayLike<number>;
  vel: ArrayLike<number>;
  yaw: number;
  pitch: number;
  stance: number;
  recoilPitch: number;
  recoilYaw: number;
}

const DEG = Math.PI / 180;
/** Visual gain of the predicted recoil on the weapon model. */
const RECOIL_GAIN = 1.6;
/** Metres the weapon slides back per degree of recoil pitch. */
const RECOIL_BACK_M_PER_DEG = 0.012;
const BOB_AMPLITUDE_M = 0.006;
const BOB_SPEED_RAD_PER_M = 2.4;

/** Distance (m) to the nearest blocking surface along a ray, or null: set by the host scene (picks buildings in the world scene). */
export type WallProbe = (ox: number, oy: number, oz: number, dx: number, dy: number, dz: number, maxDist: number) => number | null;
export type AdsState = 'hip' | 'entering' | 'ads' | 'leaving';

export class FirstPersonRig {
  /** Wall pushback (M1-046b): when set, the weapon is pulled back so the muzzle stays `WALL_MARGIN_M` short of the wall. */
  wallProbe: WallProbe | null = null;
  /** Sample the equipped weapon's predicted reload fraction (0 to 1), or undefined when idle. */
  localReloadProgress: (() => number | undefined) | undefined;
  /** Called when the ADS state changes (hip -> entering -> ads -> leaving -> hip). */
  onAds: ((state: AdsState, t: number) => void) | null = null;
  adsState: AdsState = 'hip';
  /** Last wall distance probed (m), Infinity when nothing is in reach. */
  wallDist = Infinity;

  /** Current eye height above the feet. */
  eye: number;
  /** 0 = hip, 1 = fully aimed. */
  ads = 0;
  weapon: WeaponNode | null = null;
  private def: WeaponActorDef | null = null;
  private bobPhase = 0;
  private hipFov: number;
  private readonly recoil = new RecoilSpring();
  private readonly muzzleWorld = new Vector3();
  private pendingWeapon = -1;
  private pendingShots = 0;

  constructor(
    readonly camera: TargetCamera,
    private readonly data: ActorData,
    private readonly aiming: () => boolean,
  ) {
    this.eye = data.capsule.eyeStand;
    this.hipFov = data.hipFovDeg * DEG;
    camera.detachControl();
    camera.fov = this.hipFov;
    camera.rotationQuaternion = null;
  }

  /** The host settings store clamps the hip FOV in degrees; ADS still applies its data multiplier. */
  setHipFovDegrees(deg: number): void {
    this.hipFov = deg * DEG;
    if (!this.def) this.camera.fov = this.hipFov;
  }

  setWeapon(w: WeaponNode | null): void {
    this.weapon?.dispose();
    this.weapon = w;
    this.def = w?.def ?? null;
    this.recoil.pos = this.recoil.vel = 0;
    if (!w) this.camera.fov = this.hipFov;
  }

  queueShot(weapon: number): void {
    if (weapon !== this.pendingWeapon) this.pendingShots = 0;
    this.pendingWeapon = weapon;
    this.pendingShots++;
  }

  update(l: FpLocal, dtMs: number): void {
    const { capsule: c, stanceTransitionMs } = this.data;
    const target = l.stance === 0 ? c.eyeStand : c.eyeCrouch;
    const step = ((c.eyeStand - c.eyeCrouch) * dtMs) / Math.max(1, stanceTransitionMs);
    if (this.eye < target) this.eye = Math.min(target, this.eye + step);
    else if (this.eye > target) this.eye = Math.max(target, this.eye - step);

    const cam = this.camera;
    const px = l.pos[0] as number;
    const py = (l.pos[1] as number) + this.eye;
    const pz = l.pos[2] as number;
    cam.position.set(px, py, pz);
    cam.rotation.set(-l.pitch, l.yaw, 0);

    const def = this.def;
    const w = this.weapon;
    const shots = def?.index === this.pendingWeapon ? this.pendingShots : 0;
    this.pendingShots = 0;
    if (!def || !w) return;
    const aiming = this.aiming();
    this.ads = stepAds(this.ads, dtMs / 1000, def.adsMs, aiming);
    const k = this.ads * this.ads * (3 - 2 * this.ads); // smoothstep
    const st: AdsState = this.ads >= 1 ? 'ads' : this.ads <= 0 ? 'hip' : aiming ? 'entering' : 'leaving';
    if (st !== this.adsState) {
      this.adsState = st;
      this.onAds?.(st, this.ads);
    }
    cam.fov = this.hipFov * (1 + (def.view.adsFovMult - 1) * k);

    const speed = Math.hypot(l.vel[0] as number, l.vel[2] as number);
    this.bobPhase += speed * (dtMs / 1000) * BOB_SPEED_RAD_PER_M;
    const bob = (1 - 0.85 * k) * Math.min(1, speed / 4);
    const rp = l.recoilPitch;
    this.recoil.update(dtMs / 1000);
    if (shots) {
      // The shot has a visible displacement even if this frame's elapsed time is zero.
      this.recoil.pos = Math.min(0.035, this.recoil.pos + shots * 0.012);
    }
    const reload = this.localReloadProgress?.() ?? 0;
    const lowered = Math.sin(Math.PI * Math.min(1, Math.max(0, reload))) ** 2;
    const ox = lerp(def.view.hipOffset[0], def.view.adsOffset[0], k) + Math.sin(this.bobPhase) * BOB_AMPLITUDE_M * bob;
    const oy = lerp(def.view.hipOffset[1], def.view.adsOffset[1], k) + Math.abs(Math.sin(this.bobPhase)) * BOB_AMPLITUDE_M * bob - lowered * 0.12;
    const oz = lerp(def.view.hipOffset[2], def.view.adsOffset[2], k) - rp * RECOIL_BACK_M_PER_DEG - this.recoil.pos;

    // camera basis (left-handed, y up): forward f, right r, up u
    const sy = Math.sin(l.yaw);
    const cy = Math.cos(l.yaw);
    const sp = Math.sin(l.pitch);
    const cp = Math.cos(l.pitch);
    const fx = sy * cp;
    const fy = sp;
    const fz = cy * cp;
    const rx = cy;
    const rz = -sy;
    const ux = -sy * sp;
    const uy = cp;
    const uz = -cy * sp;
    w.root.position.set(px + rx * ox + ux * oy + fx * oz, py + uy * oy + fy * oz, pz + rz * ox + uz * oy + fz * oz);
    w.root.rotation.set(-(l.pitch + rp * DEG * RECOIL_GAIN) + lowered * 0.3, l.yaw + l.recoilYaw * DEG * RECOIL_GAIN, 0);
    this.wallDist = Infinity;
    if (this.wallProbe) {
      // Project the animated muzzle onto the view ray before applying the final retraction.
      Vector3.TransformCoordinatesToRef(Vector3.ZeroReadOnly, freshWorldMatrix(w.muzzle), this.muzzleWorld);
      const muzzle = this.muzzleWorld;
      const reach = Math.max(0, (muzzle.x - px) * fx + (muzzle.y - py) * fy + (muzzle.z - pz) * fz);
      const dist = this.wallProbe(px, py, pz, fx, fy, fz, reach + 0.1);
      this.wallDist = dist ?? Infinity;
      if (dist !== null) {
        const back = Math.min(def.dims.length, wallPushback(dist, reach));
        w.root.position.set(w.root.position.x - fx * back, w.root.position.y - fy * back, w.root.position.z - fz * back);
      }
    }
  }

  dispose(): void {
    this.setWeapon(null);
  }
}

const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
