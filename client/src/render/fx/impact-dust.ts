/**
 * Impact feedback (V-FX-006 placeholder): a few small dust cubes thrown up from the impact point plus a flat
 * square mark on the ground. Pooled instances, no textures, no particle system.
 */
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import '@babylonjs/core/Meshes/instancedMesh';
import type { InstancedMesh } from '@babylonjs/core/Meshes/instancedMesh';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { Scene } from '@babylonjs/core/scene';
import { PlaceholderRegistry, REQ, phName, placeholderMaterial } from '../actors/names';

export const DUST_PER_IMPACT = 6;
export const DUST_LIFE_S = 0.45;
export const MARK_LIFE_S = 1.5;
const GRAVITY = 9.8;

interface Dust {
  mesh: InstancedMesh;
  vx: number;
  vy: number;
  vz: number;
  age: number;
}
interface Mark {
  mesh: InstancedMesh;
  age: number;
}

export class ImpactDustPool {
  private readonly dustSrc: Mesh;
  private readonly markSrc: Mesh;
  private readonly freeDust: Dust[] = [];
  private readonly liveDust: Dust[] = [];
  private readonly freeMark: Mark[] = [];
  private readonly liveMark: Mark[] = [];
  private seed = 12345;

  constructor(scene: Scene, registry: PlaceholderRegistry, dust = 96, marks = 16) {
    this.dustSrc = MeshBuilder.CreateBox(phName(REQ.FX_TRACER, 'src_dust'), { size: 0.07 }, scene);
    this.dustSrc.material = placeholderMaterial(scene, REQ.FX_TRACER, '#b49a72', { tag: 'dust' });
    this.markSrc = MeshBuilder.CreateBox(phName(REQ.FX_TRACER, 'src_mark'), { width: 0.2, height: 0.01, depth: 0.2 }, scene);
    this.markSrc.material = placeholderMaterial(scene, REQ.FX_TRACER, '#3a3226', { tag: 'mark' });
    for (const m of [this.dustSrc, this.markSrc]) {
      m.isVisible = false;
      m.isPickable = false;
    }
    registry.declare(REQ.FX_TRACER);
    for (let i = 0; i < dust; i++) {
      const mesh = this.dustSrc.createInstance(phName(REQ.FX_TRACER, `d${i}`));
      mesh.setEnabled(false);
      mesh.isPickable = false;
      mesh.alwaysSelectAsActiveMesh = true;
      this.freeDust.push({ mesh, vx: 0, vy: 0, vz: 0, age: 0 });
      registry.add(REQ.FX_TRACER);
    }
    for (let i = 0; i < marks; i++) {
      const mesh = this.markSrc.createInstance(phName(REQ.FX_TRACER, `m${i}`));
      mesh.setEnabled(false);
      mesh.isPickable = false;
      mesh.alwaysSelectAsActiveMesh = true;
      this.freeMark.push({ mesh, age: 0 });
      registry.add(REQ.FX_TRACER);
    }
  }

  get activeDust(): number {
    return this.liveDust.length;
  }
  get activeMarks(): number {
    return this.liveMark.length;
  }

  private rnd(): number {
    this.seed = (Math.imul(this.seed, 1664525) + 1013904223) >>> 0;
    return this.seed / 4294967296;
  }

  spawn(x: number, y: number, z: number, mark = true): void {
    for (let i = 0; i < DUST_PER_IMPACT; i++) {
      const d = this.freeDust.pop();
      if (!d) break;
      d.age = 0;
      d.vx = (this.rnd() - 0.5) * 2.4;
      d.vz = (this.rnd() - 0.5) * 2.4;
      d.vy = 1.2 + this.rnd() * 1.6;
      d.mesh.position.set(x, y + 0.03, z);
      d.mesh.scaling.setAll(1);
      d.mesh.setEnabled(true);
      this.liveDust.push(d);
    }
    if (!mark) return;
    let m = this.freeMark.pop();
    if (!m && this.liveMark.length > 0) {
      m = this.liveMark.shift() as Mark; // recycle the oldest mark
    }
    if (m) {
      m.age = 0;
      m.mesh.position.set(x, y + 0.01, z);
      m.mesh.rotation.y = this.rnd() * Math.PI;
      m.mesh.setEnabled(true);
      this.liveMark.push(m);
    }
  }

  update(dtS: number): void {
    for (let i = this.liveDust.length - 1; i >= 0; i--) {
      const d = this.liveDust[i] as Dust;
      d.age += dtS;
      if (d.age >= DUST_LIFE_S) {
        d.mesh.setEnabled(false);
        this.liveDust.splice(i, 1);
        this.freeDust.push(d);
        continue;
      }
      d.vy -= GRAVITY * dtS;
      const p = d.mesh.position;
      p.set(p.x + d.vx * dtS, p.y + d.vy * dtS, p.z + d.vz * dtS);
      d.mesh.scaling.setAll(1 - d.age / DUST_LIFE_S);
    }
    for (let i = this.liveMark.length - 1; i >= 0; i--) {
      const m = this.liveMark[i] as Mark;
      m.age += dtS;
      if (m.age >= MARK_LIFE_S) {
        m.mesh.setEnabled(false);
        this.liveMark.splice(i, 1);
        this.freeMark.push(m);
      }
    }
  }

  dispose(): void {
    for (const d of [...this.freeDust, ...this.liveDust]) d.mesh.dispose();
    for (const m of [...this.freeMark, ...this.liveMark]) m.mesh.dispose();
    this.dustSrc.dispose();
    this.markSrc.dispose();
  }
}
