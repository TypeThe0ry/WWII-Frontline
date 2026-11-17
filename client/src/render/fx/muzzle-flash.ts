/**
 * Muzzle flash (V-FX-001 placeholder): a pooled, emissive, flattened sphere. It is shown at the weapon's
 * `socket_muzzle` for `fx.muzzle_flash_ms` (weapon data) and follows the socket while it lives.
 */
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import '@babylonjs/core/Meshes/instancedMesh';
import type { InstancedMesh } from '@babylonjs/core/Meshes/instancedMesh';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { Scene } from '@babylonjs/core/scene';
import { PlaceholderRegistry, REQ, phName, placeholderMaterial } from '../actors/names';
import { freshWorldMatrix } from '../actors/world';

interface Flash {
  mesh: InstancedMesh;
  until: number;
  follow: TransformNode | null;
  /** Frame (engine frame id) in which the flash was made visible. */
  frame: number;
}

export class MuzzleFlashPool {
  private readonly source: Mesh;
  private readonly free: Flash[] = [];
  private readonly live: Flash[] = [];

  constructor(scene: Scene, registry: PlaceholderRegistry, size = 16) {
    this.source = MeshBuilder.CreateSphere(phName(REQ.FX_FLASH, 'src'), { diameter: 0.16, segments: 6 }, scene);
    this.source.material = placeholderMaterial(scene, REQ.FX_FLASH, '#ffd35a', { emissive: true });
    this.source.isVisible = false;
    this.source.isPickable = false;
    registry.declare(REQ.FX_FLASH);
    for (let i = 0; i < size; i++) {
      const mesh = this.source.createInstance(phName(REQ.FX_FLASH, `f${i}`));
      mesh.setEnabled(false);
      mesh.isPickable = false;
      mesh.alwaysSelectAsActiveMesh = true;
      this.free.push({ mesh, until: 0, follow: null, frame: -1 });
      registry.add(REQ.FX_FLASH);
    }
  }

  get active(): number {
    return this.live.length;
  }

  /** Shows a flash at `socket` (or at x,y,z when there is none) until `now + ms`. Returns false when the pool is exhausted. */
  spawn(now: number, ms: number, socket: TransformNode | null, x: number, y: number, z: number, frame: number): boolean {
    const f = this.free.pop();
    if (!f) return false;
    f.until = now + ms;
    f.follow = socket;
    f.frame = frame;
    f.mesh.setEnabled(true);
    f.mesh.scaling.set(1, 1, 1.6);
    if (socket) {
      const m = freshWorldMatrix(socket).m;
      f.mesh.position.set(m[12] as number, m[13] as number, m[14] as number);
    } else f.mesh.position.set(x, y, z);
    this.live.push(f);
    return true;
  }

  update(now: number): void {
    for (let i = this.live.length - 1; i >= 0; i--) {
      const f = this.live[i] as Flash;
      if (now >= f.until) {
        f.mesh.setEnabled(false);
        f.follow = null;
        this.live.splice(i, 1);
        this.free.push(f);
      } else if (f.follow) {
        const m = freshWorldMatrix(f.follow).m;
        f.mesh.position.set(m[12] as number, m[13] as number, m[14] as number);
      }
    }
  }

  /** Instances currently enabled (test hook). */
  visibleMeshes(): InstancedMesh[] {
    return this.live.map((f) => f.mesh);
  }

  dispose(): void {
    for (const f of [...this.free, ...this.live]) f.mesh.dispose();
    this.source.dispose();
  }
}
