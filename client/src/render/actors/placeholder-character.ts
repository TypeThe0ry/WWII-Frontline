/**
 * Grey-box soldier (V-CHR-US-001 / V-CHR-GE-001): capsule body + head sphere + shoulder bar / hip block / nose block
 * that show which way the body and the head face. Sizes come from `CapsuleDims` (the rifleman role data).
 *
 * All parts are instances of a few shared source meshes per faction, so any number of actors costs about ten
 * draw calls. Sockets (`socket_head`, `socket_hand_r`, `socket_hand_l`) are transform nodes where the real skeleton's
 * attachment points will be; the third-person weapon hangs on `socket_hand_r`.
 */
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import '@babylonjs/core/Meshes/instancedMesh';
import type { InstancedMesh } from '@babylonjs/core/Meshes/instancedMesh';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { Scene } from '@babylonjs/core/scene';
import type { CapsuleDims } from './actor-data';
import { FACTION_COLOR, type Faction, PlaceholderRegistry, SOCKET, characterReq, phName, placeholderMaterial } from './names';
import type { WeaponNode } from './placeholder-weapon';
import { freshWorldMatrix } from './world';

export interface CharacterGeometry {
  headRadius: number;
  shoulderSize: [number, number, number];
  hipSize: [number, number, number];
  noseSize: [number, number, number];
}

/** Proportions of the markers relative to the capsule radius (pure; tested). */
export function characterGeometry(c: CapsuleDims): CharacterGeometry {
  const r = c.radius;
  return {
    headRadius: r * 0.62,
    shoulderSize: [r * 2.7, r * 0.3, r * 0.6],
    hipSize: [r * 1.6, r * 0.3, r * 0.45],
    noseSize: [r * 0.3, r * 0.3, r * 0.5],
  };
}

type PartName = 'body' | 'head' | 'shoulders' | 'hip' | 'nose';

/** Per-faction shared source meshes. */
export class CharacterSources {
  readonly parts: Record<PartName, Mesh>;
  readonly geo: CharacterGeometry;

  constructor(
    scene: Scene,
    readonly faction: Faction,
    readonly caps: CapsuleDims,
    private readonly registry: PlaceholderRegistry,
  ) {
    const req = characterReq(faction);
    const g = (this.geo = characterGeometry(caps));
    const body = placeholderMaterial(scene, req, FACTION_COLOR[faction]);
    const mark = placeholderMaterial(scene, req, '#e6e6e6', { tag: 'marker' });
    const mk = (m: Mesh, mat: typeof body): Mesh => {
      m.material = mat;
      m.isVisible = false; // sources never draw themselves; their instances do
      m.isPickable = false;
      return m;
    };
    this.parts = {
      body: mk(MeshBuilder.CreateCapsule(phName(req, 'src_body'), { radius: caps.radius, height: caps.heightStand, tessellation: 10, subdivisions: 2, capSubdivisions: 3 }, scene), body),
      head: mk(MeshBuilder.CreateSphere(phName(req, 'src_head'), { diameter: g.headRadius * 2, segments: 8 }, scene), body),
      shoulders: mk(MeshBuilder.CreateBox(phName(req, 'src_shoulders'), { width: g.shoulderSize[0], height: g.shoulderSize[1], depth: g.shoulderSize[2] }, scene), mark),
      hip: mk(MeshBuilder.CreateBox(phName(req, 'src_hip'), { width: g.hipSize[0], height: g.hipSize[1], depth: g.hipSize[2] }, scene), mark),
      nose: mk(MeshBuilder.CreateBox(phName(req, 'src_nose'), { width: g.noseSize[0], height: g.noseSize[1], depth: g.noseSize[2] }, scene), mark),
    };
    registry.declare(req);
  }

  dispose(): void {
    for (const m of Object.values(this.parts)) m.dispose();
  }
}

export interface CharacterPose {
  x: number;
  y: number;
  z: number;
  yaw: number;
  pitch: number;
  /** 0 stand, 1 crouch (other values are treated as crouch). */
  stance: number;
  /** 0..255 wire health; 0 = dead. */
  health: number;
}

export class PlaceholderCharacter {
  readonly root: TransformNode;
  /** Pivot at chest height; rotated by the view pitch. Head, shoulders and hands hang on it. */
  readonly aim: TransformNode;
  readonly socketHead: TransformNode;
  readonly socketHandR: TransformNode;
  readonly socketHandL: TransformNode;
  weapon: WeaponNode | null = null;
  /** Current body height (metres), eased towards the stance height. */
  height: number;
  dead = false;
  /** Detail level 0 (all parts) .. 2 (capsule only), set by the manager from the camera distance with hysteresis. */
  lod = 0;
  /** Upper-body recoil kick (rad), added to the aim pitch and decayed by `update`. */
  kick = 0;

  private readonly body: InstancedMesh;
  private readonly hip: InstancedMesh;
  private readonly instances: InstancedMesh[] = [];
  private readonly req: string;

  constructor(
    readonly scene: Scene,
    readonly id: number,
    private readonly src: CharacterSources,
    private readonly registry: PlaceholderRegistry,
  ) {
    const { caps, geo } = src;
    const req = (this.req = characterReq(src.faction));
    this.height = caps.heightStand;
    this.root = new TransformNode(phName(req, id), scene);
    const inst = (part: PartName, suffix: string, parent: TransformNode): InstancedMesh => {
      const i = src.parts[part].createInstance(phName(req, `${id}_${suffix}`));
      i.parent = parent;
      i.isPickable = false;
      this.instances.push(i);
      return i;
    };
    this.body = inst('body', 'body', this.root);
    this.hip = inst('hip', 'hip', this.root);
    this.aim = new TransformNode(`${phName(req, id)}_aim`, scene);
    this.aim.parent = this.root;
    this.socketHead = new TransformNode(SOCKET.head, scene);
    this.socketHead.parent = this.aim;
    this.socketHandR = new TransformNode(SOCKET.handR, scene);
    this.socketHandR.parent = this.aim;
    this.socketHandL = new TransformNode(SOCKET.handL, scene);
    this.socketHandL.parent = this.aim;
    const head = inst('head', 'head', this.socketHead);
    head.position.set(0, 0, 0);
    const nose = inst('nose', 'nose', this.socketHead);
    nose.position.set(0, 0, geo.headRadius + geo.noseSize[2] * 0.2);
    const sh = inst('shoulders', 'shoulders', this.aim);
    sh.position.set(0, 0, 0);
    const r = caps.radius;
    this.socketHandR.position.set(r * 0.9, -r * 0.2, r * 1.2);
    this.socketHandL.position.set(-r * 0.3, -r * 0.2, r * 2.4);
    registry.add(req);
    this.layout();
  }

  /** Re-derives body / marker placement from the current height. */
  private layout(): void {
    const { caps, geo } = this.src;
    const h = this.height;
    this.body.scaling.y = h / caps.heightStand;
    this.body.position.set(0, h / 2, 0);
    this.hip.position.set(0, h * 0.5, caps.radius * 0.7);
    this.aim.position.set(0, h * 0.78, 0);
    this.socketHead.position.set(0, h - h * 0.78 - geo.headRadius * 0.7, 0);
  }

  setWeapon(w: WeaponNode | null): void {
    if (this.weapon) this.weapon.dispose();
    this.weapon = w;
    if (w) {
      w.root.parent = this.socketHandR;
      w.root.position.set(0, 0, 0);
    }
  }

  update(p: CharacterPose, dtMs: number, stanceMs: number): void {
    const { caps } = this.src;
    // stance 0 stand, 1 crouch, 2 prone (lying: about twice the capsule radius high [tbd], no data field yet)
    const target = p.stance === 0 ? caps.heightStand : p.stance === 2 ? Math.min(caps.heightCrouch, caps.radius * 2.2) : caps.heightCrouch;
    const step = ((caps.heightStand - Math.min(caps.heightCrouch, caps.radius * 2.2)) * dtMs) / Math.max(1, stanceMs);
    if (this.height < target) this.height = Math.min(target, this.height + step);
    else if (this.height > target) this.height = Math.max(target, this.height - step);
    this.layout();
    this.root.position.set(p.x, p.y, p.z);
    this.root.rotation.y = p.yaw;
    this.kick = Math.max(0, this.kick - (dtMs / 1000) * 0.5);
    this.aim.rotation.x = -p.pitch - this.kick;
    this.setDead(p.health === 0);
  }

  /** Switches the detail level: 1 hides the small markers (nose, shoulders, hip), 2 also the head and the weapon. */
  setLod(lod: number): void {
    if (lod === this.lod) return;
    this.lod = lod;
    for (const i of this.instances) {
      const n = i.name;
      const small = n.endsWith('_nose') || n.endsWith('_shoulders') || n.endsWith('_hip');
      const head = n.endsWith('_head');
      i.setEnabled(lod === 0 || (lod === 1 && !small) || (lod >= 2 && !small && !head));
    }
    this.weapon?.root.setEnabled(lod < 2);
  }

  /** Dead: lies on its side, half transparent. (Death animation proper is M1.) */
  private setDead(dead: boolean): void {
    if (dead === this.dead) return;
    this.dead = dead;
    this.root.rotation.z = dead ? Math.PI / 2 : 0;
    for (const i of this.instances) i.visibility = dead ? 0.5 : 1;
  }

  /** Top of the head in world space (for debug labels). */
  headPosition(out: { x: number; y: number; z: number }): void {
    const m = freshWorldMatrix(this.socketHead).m;
    out.x = m[12] as number;
    out.y = (m[13] as number) + this.src.geo.headRadius;
    out.z = m[14] as number;
  }

  dispose(): void {
    this.weapon?.dispose();
    this.weapon = null;
    for (const i of this.instances) i.dispose();
    for (const n of [this.socketHead, this.socketHandR, this.socketHandL, this.aim, this.root]) n.dispose(false, false);
    this.registry.add(this.req, -1);
  }
}

