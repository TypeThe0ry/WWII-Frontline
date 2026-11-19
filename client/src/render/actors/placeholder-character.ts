/**
 * Grey-box soldier (V-CHR-US-001 / V-CHR-GE-001): capsule body + head sphere + shoulder bar / hip block / nose block
 * that show which way the body and the head face. Sizes come from `CapsuleDims` (the rifleman role data).
 *
 * The existing meshes are rigid instances parented to procedural joint pivots; there is no mesh skinning.
 * The third-person weapon hangs on the right-hand socket, with its muzzle socket on the weapon itself.
 */
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import '@babylonjs/core/Meshes/instancedMesh';
import type { InstancedMesh } from '@babylonjs/core/Meshes/instancedMesh';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { Scene } from '@babylonjs/core/scene';
import type { CapsuleDims } from './actor-data';
import { AnimGraph, type AnimSample } from './anim-graph';
import { FACTION_COLOR, type Faction, PlaceholderRegistry, SOCKET, characterReq, phName, placeholderMaterial } from './names';
import type { WeaponNode } from './placeholder-weapon';
import { PlaceholderSkeleton } from './skeleton';
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

export interface CharacterPose extends AnimSample {
  x: number;
  y: number;
  z: number;
  yaw: number;
  pitch: number;
}

export class PlaceholderCharacter {
  readonly root: TransformNode;
  readonly skeleton: PlaceholderSkeleton;
  readonly animation = new AnimGraph();
  weapon: WeaponNode | null = null;
  /** Current capsule envelope height (m). */
  height: number;
  /** Detail level 0 (all parts) .. 2 (capsule only), selected by the manager. */
  lod = 0;

  private readonly body: InstancedMesh;
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
    this.skeleton = new PlaceholderSkeleton(this.root, req, id, caps.radius);
    const inst = (part: PartName, parent: TransformNode): InstancedMesh => {
      const i = src.parts[part].createInstance(phName(req, `${id}_${part}`));
      i.parent = parent;
      i.isPickable = false;
      this.instances.push(i);
      return i;
    };
    this.body = inst('body', this.skeleton.joints.Hips);
    inst('hip', this.skeleton.joints.Hips).position.z = caps.radius * 0.7;
    inst('head', this.socketHead);
    inst('nose', this.socketHead).position.z = geo.headRadius + geo.noseSize[2] * 0.2;
    inst('shoulders', this.aim);
    registry.add(req);
    this.skeleton.layout(this.height, geo.headRadius, 0);
  }

  get aim(): TransformNode { return this.skeleton.joints.Chest; }
  get socketHead(): TransformNode { return this.skeleton.sockets[SOCKET.head]; }
  get socketHandR(): TransformNode { return this.skeleton.sockets[SOCKET.handR]; }
  get socketHandL(): TransformNode { return this.skeleton.sockets[SOCKET.handL]; }
  get dead(): boolean { return this.animation.life === 'dead'; }
  get kick(): number { return this.animation.recoil; }

  setWeapon(w: WeaponNode | null): void {
    this.weapon?.dispose();
    this.animation.resetWeaponLayer();
    this.weapon = w;
    if (w) {
      w.root.parent = this.socketHandR;
      w.root.position.set(0, 0, 0);
      w.root.setEnabled(this.lod < 2);
    }
  }

  update(p: Readonly<CharacterPose>, dtMs: number, distanceM: number): 'left' | 'right' | undefined {
    const a = this.animation;
    const foot = a.update(dtMs / 1000, p, distanceM);
    const { caps, geo } = this.src;
    const downed = a.life === 'downed';
    const pose = downed ? 2 : a.stancePose;
    const prone = Math.min(caps.heightCrouch, caps.radius * 2.2);
    this.height = pose <= 1
      ? caps.heightStand + (caps.heightCrouch - caps.heightStand) * pose
      : caps.heightCrouch + (prone - caps.heightCrouch) * (pose - 1);

    const phase = a.cycle * 2 * Math.PI;
    const stride = a.incapacitated ? 0 : a.strideWeight;
    const swing = Math.cos(phase) * stride * 0.4;
    this.skeleton.layout(this.height, geo.headRadius, Math.abs(Math.sin(phase)) * stride * caps.radius * 0.12);
    this.body.scaling.y = this.height / caps.heightStand;
    this.root.position.set(p.x, p.y, p.z);
    this.root.rotation.set(0, p.yaw, this.dead ? Math.PI / 2 : downed ? Math.PI / 6 : 0);
    const j = this.skeleton.joints;
    const flex = pose * 0.25;
    const air = a.incapacitated ? 0 : a.airWeight;
    j.LeftUpperLeg.rotation.x = -flex - air * 0.45 + swing;
    j.RightUpperLeg.rotation.x = -flex - air * 0.45 - swing;
    j.LeftLowerLeg.rotation.x = flex + air * 0.6 + Math.max(0, -swing);
    j.RightLowerLeg.rotation.x = flex + air * 0.6 + Math.max(0, swing);
    j.Spine.rotation.z = a.hitWeight * 0.12;
    this.aim.rotation.x = a.incapacitated ? 0 : -p.pitch - a.recoil;
    const reload = a.reloadWeight * (1 - Math.min(1, a.recoil / 0.06));
    j.RightLowerArm.rotation.x = reload * 0.45;
    j.LeftLowerArm.rotation.x = reload * 0.7;
    return foot;
  }

  /** Switches the detail level: 1 hides the small markers, 2 also the head and the weapon. */
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
    this.root.dispose(); // owns the joints, sockets and rigid mesh instances, but not their shared sources
    this.registry.add(this.req, -1);
  }
}
