/**
 * Debug tracers (V-FX-006 placeholder): a thin emissive box that flies along the shot direction and disappears at
 * the end of its range or at the impact point (an `impact` event close to its path shortens it). On by default in M0
 * (`?tracers=0` turns it off).
 */
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import '@babylonjs/core/Meshes/instancedMesh';
import type { InstancedMesh } from '@babylonjs/core/Meshes/instancedMesh';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { Scene } from '@babylonjs/core/scene';
import { PlaceholderRegistry, REQ, phName, placeholderMaterial } from '../actors/names';

export const TRACER_LENGTH_M = 2.5;
/** Visual range cap when no impact arrives (m). */
export const TRACER_MAX_RANGE_M = 400;
/** Visual speed cap (m/s): the real muzzle velocity makes a 2.5 m streak invisible at 60 fps. */
export const TRACER_MAX_SPEED_MPS = 600;
/** An impact within this distance of a tracer's line ends that tracer. */
export const TRACER_IMPACT_MATCH_M = 2;
const TRACER_MAX_LIFE_S = 0.3;
const VISUAL_GRAVITY_MPS2 = 9.8;

interface Tracer {
  mesh: InstancedMesh;
  ox: number;
  oy: number;
  oz: number;
  dx: number;
  dy: number;
  dz: number;
  speed: number;
  /** Distance travelled by the tail end of the streak. */
  travelled: number;
  range: number;
  ballistic: boolean;
  fresh: boolean;
}

export class TracerPool {
  private readonly source: Mesh;
  private readonly free: Tracer[] = [];
  private readonly live: Tracer[] = [];

  constructor(scene: Scene, registry: PlaceholderRegistry, size = 48) {
    this.source = MeshBuilder.CreateBox(phName(REQ.FX_TRACER, 'src_tracer'), { width: 0.03, height: 0.03, depth: TRACER_LENGTH_M }, scene);
    this.source.material = placeholderMaterial(scene, REQ.FX_TRACER, '#fff2a0', { emissive: true, tag: 'tracer' });
    this.source.isVisible = false;
    this.source.isPickable = false;
    registry.declare(REQ.FX_TRACER);
    for (let i = 0; i < size; i++) {
      const mesh = this.source.createInstance(phName(REQ.FX_TRACER, `t${i}`));
      mesh.setEnabled(false);
      mesh.isPickable = false;
      mesh.alwaysSelectAsActiveMesh = true;
      this.free.push({ mesh, ox: 0, oy: 0, oz: 0, dx: 0, dy: 0, dz: 1, speed: 0, travelled: 0, range: 0, ballistic: false, fresh: false });
      registry.add(REQ.FX_TRACER);
    }
  }

  get active(): number {
    return this.live.length;
  }

  spawn(ox: number, oy: number, oz: number, dx: number, dy: number, dz: number, muzzleVelocityMps: number, ballistic = false): boolean {
    if (!(muzzleVelocityMps > 0)) return false;
    const t = this.free.pop();
    if (!t) return false;
    t.ox = ox;
    t.oy = oy;
    t.oz = oz;
    t.dx = dx;
    t.dy = dy;
    t.dz = dz;
    t.speed = Math.min(TRACER_MAX_SPEED_MPS, muzzleVelocityMps);
    t.travelled = 0;
    t.range = TRACER_MAX_RANGE_M;
    t.ballistic = ballistic;
    t.fresh = ballistic;
    t.mesh.setEnabled(true);
    this.place(t);
    this.live.push(t);
    return true;
  }

  private place(t: Tracer): void {
    const c = t.travelled + TRACER_LENGTH_M / 2; // centre of the streak
    const age = c / t.speed;
    const gravity = t.ballistic ? VISUAL_GRAVITY_MPS2 : 0;
    t.mesh.position.set(t.ox + t.dx * c, t.oy + t.dy * c - 0.5 * gravity * age * age, t.oz + t.dz * c);
    const vertical = t.dy - gravity * age / t.speed;
    t.mesh.rotation.set(-Math.atan2(vertical, Math.hypot(t.dx, t.dz)), Math.atan2(t.dx, t.dz), 0);
  }

  /** The shot hit something at (x,y,z): the nearest tracer whose line passes within `TRACER_IMPACT_MATCH_M` ends there. */
  impactAt(x: number, y: number, z: number): void {
    let best: Tracer | null = null;
    let bestD = TRACER_IMPACT_MATCH_M;
    let bestAlong = 0;
    for (const t of this.live) {
      const px = x - t.ox;
      const py = y - t.oy;
      const pz = z - t.oz;
      const along = px * t.dx + py * t.dy + pz * t.dz;
      if (along < 0) continue;
      const d = Math.hypot(px - t.dx * along, py - t.dy * along, pz - t.dz * along);
      if (d < bestD) {
        bestD = d;
        best = t;
        bestAlong = along;
      }
    }
    if (best) best.range = Math.min(best.range, bestAlong);
  }

  update(dtS: number): void {
    for (let i = this.live.length - 1; i >= 0; i--) {
      const t = this.live[i] as Tracer;
      if (!t.fresh) t.travelled += t.speed * dtS;
      t.fresh = false;
      // the head of the streak is `travelled + length`; it disappears when the head reaches the range end
      if (t.travelled + TRACER_LENGTH_M >= t.range || (t.ballistic && t.travelled >= t.speed * TRACER_MAX_LIFE_S)) {
        t.mesh.setEnabled(false);
        this.live.splice(i, 1);
        this.free.push(t);
      } else this.place(t);
    }
  }

  dispose(): void {
    for (const t of [...this.free, ...this.live]) t.mesh.dispose();
    this.source.dispose();
  }
}
