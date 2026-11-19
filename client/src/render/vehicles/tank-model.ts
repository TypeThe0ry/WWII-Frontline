/**
 * Placeholder tank (M1-048b, GAMEDESIGN 14.1/14.3): hull, two tracks, turret and gun as boxes sized from the vehicle data,
 * bound to requirement V-VEH-US-003 (`PH_V-VEH-US-003`, nodes `PH_V-VEH-US-003__<part>`). The turret follows the authoritative angle
 * at no more than the data traverse rate, the gun pitch is clamped to the data range, and each track scrolls at its own speed
 * (`tank-math.ts`). Sockets are empty transform nodes under the part they belong to.
 */
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { Quaternion, Vector3, Vector4, type Matrix } from '@babylonjs/core/Maths/math.vector';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { RawTexture } from '@babylonjs/core/Materials/Textures/rawTexture';
import { Texture } from '@babylonjs/core/Materials/Textures/texture';
import { Constants } from '@babylonjs/core/Engines/constants';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { Scene } from '@babylonjs/core/scene';
import { PlaceholderRegistry, phName, placeholderMaterial } from '../actors/names';
import { freshWorldMatrix } from '../actors/world';
import { TANK_SOCKETS, TANK_SOCKET_ALIASES, angleDiff, clampGunPitch, followAngle, hullBox, trackSpeeds, trackUvRate } from './tank-math';

export const TANK_REQ = 'V-VEH-US-003';
/** Belt repeat length (m) for the track UV scroll [tbd, real track pitch comes with the asset]. */
export const TRACK_REPEAT_M = 0.6;
const TRACK_W = 0.38;

export interface TankDef {
  id: string;
  dims: { length: number; width: number; height: number };
  turretTraverseRadS: number;
  gunElevationDeg: [number, number];
  maxSpeedMps: number;
}

interface TankJson {
  id: string;
  historical: { dimensions_m: { length: number; width: number; height: number } };
  gameplay: { max_speed_mps: number; turret_traverse_deg_s: number; gun_elevation_deg: [number, number]; sockets: string[] };
}

export function tankDefFromJson(j: TankJson): TankDef {
  return {
    id: j.id,
    dims: j.historical.dimensions_m,
    turretTraverseRadS: (j.gameplay.turret_traverse_deg_s * Math.PI) / 180,
    gunElevationDeg: j.gameplay.gun_elevation_deg,
    maxSpeedMps: j.gameplay.max_speed_mps,
  };
}

/** Authoritative (or interpolated) state of one tank for a frame. Yaw convention of the sim: 0 = +z, growing toward +x. */
export interface TankState {
  x: number;
  y: number;
  z: number;
  yaw: number;
  /** Sampled hull rotation when available. Overrides yaw and preserves slope pitch and roll. */
  hullQuat?: { readonly x: number; readonly y: number; readonly z: number; readonly w: number };
  /** Hull forward speed (m/s, negative = reverse) and yaw rate (rad/s, positive = turning right). */
  speed: number;
  yawRate: number;
  /** Turret yaw relative to the hull and gun pitch (deg) as the server has them. */
  turretYaw: number;
  gunPitchDeg: number;
}

function trackTexture(scene: Scene): RawTexture {
  const w = 4;
  const h = 8;
  const px = new Uint8Array(w * h * 4);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) px.set(y % 2 === 0 ? [40, 38, 34, 255] : [86, 80, 70, 255], (y * w + x) * 4);
  const t = RawTexture.CreateRGBATexture(px, w, h, scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE);
  t.wrapU = Texture.WRAP_ADDRESSMODE;
  t.wrapV = Texture.WRAP_ADDRESSMODE;
  return t;
}

export class TankModel {
  readonly root: TransformNode;
  readonly turret: TransformNode;
  readonly gun: TransformNode;
  readonly sockets = new Map<string, TransformNode>();
  readonly meshes: Mesh[] = [];
  /** Displayed turret yaw (rad, hull-relative) and the track texture offsets (diagnostics / tests). */
  shownTurretYaw = 0;
  trackOffsetL = 0;
  trackOffsetR = 0;
  private readonly tex: [RawTexture, RawTexture];
  private readonly trackMaterials: StandardMaterial[] = [];
  private readonly hullRotation = new Quaternion();
  private readonly gauge: number;
  private disposed = false;

  constructor(
    scene: Scene,
    readonly def: TankDef,
    private readonly registry: PlaceholderRegistry,
    instance: string | number = 0,
  ) {
    const part = (n: string): string => phName(TANK_REQ, `${instance}_${n}`);
    const hullMat = placeholderMaterial(scene, TANK_REQ, '#5b6b34');
    const { size } = hullBox(def.dims);
    const hullH = def.dims.height * 0.55;
    this.root = new TransformNode(part('root'), scene);
    this.gauge = def.dims.width - TRACK_W;
    registry.declare(TANK_REQ);
    const add = (m: Mesh, parent: TransformNode): Mesh => {
      m.parent = parent;
      m.isPickable = false;
      this.meshes.push(m);
      registry.add(TANK_REQ);
      return m;
    };
    // hull: the lower 55 % of the data height; tracks sit on both sides, the turret on top
    const hull = add(MeshBuilder.CreateBox(part('hull'), { width: size[0] - 2 * TRACK_W, height: hullH, depth: size[2] * 0.92 }, scene), this.root);
    hull.position.y = hullH / 2 + 0.2;
    hull.material = hullMat;
    this.tex = [trackTexture(scene), trackTexture(scene)];
    const trackMat = (i: 0 | 1): StandardMaterial => {
      const m = new StandardMaterial(`PH_${TANK_REQ}__track_${instance}_${i}`, scene);
      m.diffuseTexture = this.tex[i];
      m.specularColor = Color3.Black();
      this.trackMaterials.push(m);
      return m;
    };
    const trackH = hullH * 0.9;
    for (const [i, sx] of [[0, -1], [1, 1]] as const) {
      const t = add(MeshBuilder.CreateBox(part(`track_${i === 0 ? 'l' : 'r'}`), { width: TRACK_W, height: trackH, depth: size[2], faceUV: Array.from({ length: 6 }, () => new Vector4(0, 0, 1, size[2] / TRACK_REPEAT_M)) }, scene), this.root);
      t.position.set((sx * (def.dims.width - TRACK_W)) / 2, trackH / 2, 0);
      t.material = trackMat(i);
    }
    this.turret = new TransformNode(part('turret'), scene);
    this.turret.parent = this.root;
    this.turret.position.set(0, hullH + 0.2, -size[2] * 0.04);
    const tH = def.dims.height - hullH - 0.2;
    const tur = add(MeshBuilder.CreateBox(part('turret_body'), { width: size[0] * 0.62, height: tH, depth: size[2] * 0.42 }, scene), this.turret);
    tur.position.y = tH / 2;
    tur.material = hullMat;
    this.gun = new TransformNode(part('gun'), scene);
    this.gun.parent = this.turret;
    this.gun.position.set(0, tH * 0.55, size[2] * 0.2);
    const gunLen = def.dims.length * 0.42;
    const barrel = add(MeshBuilder.CreateBox(part('gun_barrel'), { width: 0.12, height: 0.12, depth: gunLen }, scene), this.gun);
    barrel.position.z = gunLen / 2;
    barrel.material = placeholderMaterial(scene, TANK_REQ, '#3d4126', { tag: 'gun' });
    const sock = (name: string, parent: TransformNode, x: number, y: number, z: number): void => {
      const n = new TransformNode(phName(TANK_REQ, `${instance}_${name}`), scene);
      n.parent = parent;
      n.position.set(x, y, z);
      this.sockets.set(name, n);
    };
    sock('socket_gun_muzzle', this.gun, 0, 0, gunLen);
    sock('socket_turret', this.turret, 0, 0, 0);
    sock('socket_seat_driver', this.root, -0.4, hullH * 0.8, size[2] * 0.3);
    sock('socket_seat_gunner', this.turret, -0.3, tH * 0.5, 0);
    sock('socket_seat_commander', this.turret, 0.3, tH * 0.8, -0.3);
    sock('socket_hatch', this.turret, 0.3, tH, -0.3);
    sock('socket_exhaust', this.root, 0, hullH * 0.9, -size[2] * 0.5);
    sock('socket_track_l', this.root, -def.dims.width / 2, 0.2, 0);
    sock('socket_track_r', this.root, def.dims.width / 2, 0.2, 0);
    sock('socket_mg_coax', this.gun, 0.1, 0, 0.5);
  }

  /** Place the hull and gun. Only unsampled target orders need rate-limited turret following. */
  update(s: TankState, dtS: number, yawSource: 'target' | 'sampled' = 'target'): void {
    this.root.position.set(s.x, s.y, s.z);
    if (s.hullQuat) {
      const { x, y, z, w } = s.hullQuat;
      this.root.rotationQuaternion = this.hullRotation.set(x, y, z, w);
    } else {
      this.root.rotationQuaternion = null;
      this.root.rotation.set(0, s.yaw, 0);
    }
    this.shownTurretYaw = yawSource === 'sampled' ? s.turretYaw : followAngle(this.shownTurretYaw, s.turretYaw, dtS, this.def.turretTraverseRadS);
    this.turret.rotation.y = this.shownTurretYaw;
    // the sim pitches positive = up; Babylon's x rotation is positive = nose down
    this.gun.rotation.x = -(clampGunPitch(s.gunPitchDeg, this.def.gunElevationDeg) * Math.PI) / 180;
    const ts = trackSpeeds(s.speed, s.yawRate, this.gauge);
    this.trackOffsetL = (this.trackOffsetL + trackUvRate(ts.left, TRACK_REPEAT_M) * dtS) % 1;
    this.trackOffsetR = (this.trackOffsetR + trackUvRate(ts.right, TRACK_REPEAT_M) * dtS) % 1;
    this.tex[0].vOffset = this.trackOffsetL;
    this.tex[1].vOffset = this.trackOffsetR;
  }

  /** World position of a socket (fresh matrix; for spawning muzzle effects). */
  socketWorld(name: string, out: Vector3): Vector3 {
    const matrix = this.socketMatrix(name);
    if (!matrix) throw new Error(`tank socket ${name} missing`);
    return matrix.getTranslationToRef(out);
  }

  /** Borrowed world transform of a data socket or its legacy alias. */
  socketMatrix(name: string): Matrix | null {
    const node = this.sockets.get(TANK_SOCKET_ALIASES[name] ?? name);
    return node ? freshWorldMatrix(node) : null;
  }

  /** The tank's angular error between the displayed and the authoritative turret yaw (rad). */
  turretError(authoritative: number): number {
    return Math.abs(angleDiff(this.shownTurretYaw, authoritative));
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.root.dispose();
    for (const material of this.trackMaterials) material.dispose();
    for (const t of this.tex) t.dispose();
    this.registry.add(TANK_REQ, -this.meshes.length);
  }
}

export const requiredSockets = (): string[] => [...TANK_SOCKETS];
