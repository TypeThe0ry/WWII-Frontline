/**
 * Grey-box rifle (V-WPN-US-001 / V-WPN-GE-001): receiver box + barrel cylinder + magazine box, sized from
 * `WeaponModelDims` (data). Local frame: +z forward, +y up, +x right, origin at the grip; the muzzle is at
 * `+z = muzzleZ`. Sockets (`socket_muzzle`, `socket_ejection`, `socket_magazine`) are plain transform nodes where the
 * real model's attachment points will be.
 *
 * Third-person weapons are instances of three shared source meshes per weapon (a handful of draw calls for any number
 * of actors); the first-person weapon is built from its own meshes in rendering group 1 (drawn over the terrain).
 */
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import '@babylonjs/core/Meshes/instancedMesh';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import type { Scene } from '@babylonjs/core/scene';
import type { WeaponActorDef, WeaponModelDims } from './actor-data';
import { PlaceholderRegistry, SOCKET, phName, placeholderMaterial } from './names';

export interface WeaponLayout {
  receiver: { size: [number, number, number]; pos: [number, number, number] };
  barrel: { length: number; diameter: number; pos: [number, number, number] };
  magazine: { size: [number, number, number]; pos: [number, number, number] };
  sockets: { muzzle: [number, number, number]; ejection: [number, number, number]; magazine: [number, number, number] };
  /** z of the muzzle (front of the barrel). */
  muzzleZ: number;
}

/** Pure geometry of the grey-box weapon. The grip sits 45 % of the overall length from the rear end. */
export function weaponLayout(d: WeaponModelDims): WeaponLayout {
  const rear = -0.45 * d.length;
  const recvLen = d.length - d.barrelLength;
  const recvCz = rear + recvLen / 2;
  const barrelY = d.receiverHeight * 0.25;
  const barrelCz = rear + recvLen + d.barrelLength / 2;
  const muzzleZ = rear + d.length;
  return {
    receiver: { size: [d.receiverWidth, d.receiverHeight, recvLen], pos: [0, 0, recvCz] },
    barrel: { length: d.barrelLength, diameter: d.barrelDiameter, pos: [0, barrelY, barrelCz] },
    magazine: { size: [d.magWidth, d.magHeight, d.magLength], pos: [0, -(d.receiverHeight + d.magHeight) / 2, 0] },
    sockets: {
      muzzle: [0, barrelY, muzzleZ],
      ejection: [d.receiverWidth / 2, d.receiverHeight * 0.25, 0.05 * d.length],
      magazine: [0, -d.receiverHeight / 2 - d.magHeight, 0],
    },
    muzzleZ,
  };
}

export interface WeaponNode {
  root: TransformNode;
  muzzle: TransformNode;
  ejection: TransformNode;
  magazine: TransformNode;
  def: WeaponActorDef;
  /** Meshes owned by this node (instances for third person, real meshes for first person). */
  meshes: AbstractMesh[];
  dispose(): void;
}

function addSockets(root: TransformNode, l: WeaponLayout, scene: Scene): Pick<WeaponNode, 'muzzle' | 'ejection' | 'magazine'> {
  const mk = (name: string, p: [number, number, number]): TransformNode => {
    const n = new TransformNode(name, scene);
    n.parent = root;
    n.position.set(p[0], p[1], p[2]);
    return n;
  };
  return { muzzle: mk(SOCKET.muzzle, l.sockets.muzzle), ejection: mk(SOCKET.ejection, l.sockets.ejection), magazine: mk(SOCKET.magazine, l.sockets.magazine) };
}

interface Parts {
  receiver: Mesh;
  barrel: Mesh;
  magazine: Mesh;
}

function buildParts(scene: Scene, def: WeaponActorDef, prefix: string, layout: WeaponLayout): Parts {
  const req = def.requirementId;
  const mat = placeholderMaterial(scene, req, '#3b3b3b');
  const receiver = MeshBuilder.CreateBox(phName(req, `${prefix}_receiver`), { width: layout.receiver.size[0], height: layout.receiver.size[1], depth: layout.receiver.size[2] }, scene);
  const barrel = MeshBuilder.CreateCylinder(phName(req, `${prefix}_barrel`), { height: layout.barrel.length, diameter: layout.barrel.diameter, tessellation: 8 }, scene);
  const magazine = MeshBuilder.CreateBox(phName(req, `${prefix}_magazine`), { width: layout.magazine.size[0], height: layout.magazine.size[1], depth: layout.magazine.size[2] }, scene);
  for (const m of [receiver, barrel, magazine]) m.material = mat;
  return { receiver, barrel, magazine };
}

function place(m: AbstractMesh, p: [number, number, number], parent: TransformNode): void {
  m.parent = parent;
  m.position.set(p[0], p[1], p[2]);
}

/** Shared sources of one weapon for instanced (third-person) copies. */
export class WeaponSources {
  readonly layout: WeaponLayout;
  readonly parts: Parts;

  constructor(
    private readonly scene: Scene,
    readonly def: WeaponActorDef,
    private readonly registry: PlaceholderRegistry,
  ) {
    this.layout = weaponLayout(def.dims);
    this.parts = buildParts(scene, def, 'src', this.layout);
    this.parts.barrel.rotation.x = Math.PI / 2; // cylinder axis y -> z
    this.parts.barrel.bakeCurrentTransformIntoVertices();
    for (const m of Object.values(this.parts)) {
      m.isVisible = false; // sources never draw themselves; their instances do
      m.isPickable = false;
    }
    registry.declare(def.requirementId);
  }

  /** One third-person weapon (3 instances + sockets); `scale` makes it a size smaller than the first-person one. */
  instantiate(name: string, scale: number): WeaponNode {
    const { scene, def, layout, parts } = this;
    const req = def.requirementId;
    const root = new TransformNode(phName(req, name), scene);
    root.scaling.setAll(scale);
    const meshes: AbstractMesh[] = [];
    const mk = (src: Mesh, part: string, p: [number, number, number]): void => {
      const i = src.createInstance(phName(req, `${name}_${part}`));
      place(i, p, root);
      i.isPickable = false;
      meshes.push(i);
    };
    mk(parts.receiver, 'receiver', layout.receiver.pos);
    mk(parts.barrel, 'barrel', layout.barrel.pos);
    mk(parts.magazine, 'magazine', layout.magazine.pos);
    const sockets = addSockets(root, layout, scene);
    this.registry.add(req);
    return {
      root,
      ...sockets,
      def,
      meshes,
      dispose: () => {
        for (const m of meshes) m.dispose();
        root.dispose(false, false);
        this.registry.add(req, -1);
      },
    };
  }

  dispose(): void {
    for (const m of Object.values(this.parts)) m.dispose();
  }
}

/** First-person weapon: own meshes in rendering group 1 (always drawn, over the world). */
export function buildFirstPersonWeapon(scene: Scene, def: WeaponActorDef, registry: PlaceholderRegistry): WeaponNode {
  const layout = weaponLayout(def.dims);
  const req = def.requirementId;
  const parts = buildParts(scene, def, 'fp', layout);
  parts.barrel.rotation.x = Math.PI / 2;
  parts.barrel.bakeCurrentTransformIntoVertices();
  const root = new TransformNode(phName(req, 'fp'), scene);
  place(parts.receiver, layout.receiver.pos, root);
  place(parts.barrel, layout.barrel.pos, root);
  place(parts.magazine, layout.magazine.pos, root);
  const meshes: AbstractMesh[] = [parts.receiver, parts.barrel, parts.magazine];
  for (const m of meshes) {
    m.renderingGroupId = 1;
    m.alwaysSelectAsActiveMesh = true;
    m.isPickable = false;
  }
  const sockets = addSockets(root, layout, scene);
  registry.add(req);
  return {
    root,
    ...sockets,
    def,
    meshes,
    dispose: () => {
      for (const m of meshes) m.dispose();
      root.dispose(false, false);
      registry.add(req, -1);
    },
  };
}
