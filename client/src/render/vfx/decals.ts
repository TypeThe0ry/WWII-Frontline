import { Ray } from '@babylonjs/core/Culling/ray';
import { VertexBuffer } from '@babylonjs/core/Buffers/buffer';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import { VertexData } from '@babylonjs/core/Meshes/mesh.vertexData';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import type { Scene } from '@babylonjs/core/scene';
import { LAYER } from '../terrain/terrain-splat';
import type { ImpactPreset } from './emitters';
import { DecalRing, FAR_MERGE_M, dustPreset } from './particle-pool';

// A small two-sided probe around the quantized C2 hit. Do not paint across empty space.
const PROBE_M = 0.12;
const DEPTH_BIAS_M = 0.003;
const FADE_START_M = 35;
export const DECAL_DISTANCE_M = 60;
const SIDES = 8;
const VERTICES = 1 + SIDES * 2;
const UNIT_CIRCLE = Array.from({ length: SIDES }, (_, i) => {
  const angle = i * Math.PI * 2 / SIDES;
  return [Math.cos(angle), Math.sin(angle)] as const;
});
const STYLES: Record<ImpactPreset, { radius: number; rim: readonly [number, number, number] }> = {
  earth: { radius: 0.07, rim: [0.25, 0.17, 0.09] },
  sand: { radius: 0.083, rim: [0.38, 0.29, 0.16] },
  wood: { radius: 0.055, rim: [0.30, 0.19, 0.09] },
  brick: { radius: 0.066, rim: [0.37, 0.18, 0.13] },
  metal: { radius: 0.042, rim: [0.48, 0.47, 0.45] },
  stone: { radius: 0.056, rim: [0.34, 0.32, 0.29] },
};

function terrainImpactPreset(layer: number): ImpactPreset {
  if (layer === LAYER.sand) return 'sand';
  if (layer === LAYER.rock || layer === LAYER.gravel) return 'stone';
  return 'earth';
}

/** Inverse of the server's 6+6 bit octahedral encoding, with Y up. */
export function decodeImpactNormal(packed: number, out: Vector3): Vector3 {
  let x = (packed >>> 6 & 63) * 2 / 63 - 1;
  let z = (packed & 63) * 2 / 63 - 1;
  const y = 1 - Math.abs(x) - Math.abs(z);
  if (y < 0) {
    const unfoldedX = (1 - Math.abs(z)) * Math.sign(x);
    z = (1 - Math.abs(x)) * Math.sign(z);
    x = unfoldedX;
  }
  return out.set(x, y, z).normalize();
}

/** A fixed ring of surface-aligned procedural holes, batched into one depth-tested mesh. */
export class ImpactDecals {
  readonly mesh: Mesh;
  readonly marks = new DecalRing();
  private readonly material: StandardMaterial;
  private readonly positions = new Float32Array(DecalRing.CAP * VERTICES * 3);
  private readonly colors = new Float32Array(DecalRing.CAP * VERTICES * 4);
  private readonly fades = new Float32Array(DecalRing.CAP);
  private readonly wireNormal = new Vector3();
  private readonly normal = new Vector3();
  private readonly center = new Vector3();
  private readonly tangent = new Vector3();
  private readonly bitangent = new Vector3();
  private readonly ray = new Ray(new Vector3(), new Vector3(), PROBE_M * 2);
  private readonly surfaces = new Set<AbstractMesh>();
  private readonly isReceiver = (mesh: AbstractMesh): boolean => this.surfaces.has(mesh) && mesh.isEnabled() && mesh.isVisible;
  private heightAt?: (x: number, z: number) => number;
  private getSurfaceAt?: (x: number, z: number) => number;
  private positionDirty = false;

  constructor(private readonly scene: Scene) {
    this.mesh = new Mesh('PH_V-FX-004__bullet_holes', scene);
    const indices: number[] = [];
    for (let slot = 0; slot < DecalRing.CAP; slot++) {
      const base = slot * VERTICES;
      for (let side = 0; side < SIDES; side++) {
        const a = base + side + 1;
        const b = base + (side + 1) % SIDES + 1;
        indices.push(base, a, b, a, a + SIDES, b, b, a + SIDES, b + SIDES);
      }
    }
    const data = new VertexData();
    data.positions = this.positions;
    data.colors = this.colors;
    data.indices = indices;
    data.applyToMesh(this.mesh, true);
    this.material = new StandardMaterial('PH_V-FX-004__bullet_holes_mat', scene);
    this.material.disableLighting = true;
    this.material.emissiveColor = Color3.White();
    this.material.backFaceCulling = false;
    this.material.transparencyMode = StandardMaterial.MATERIAL_ALPHABLEND;
    this.material.disableDepthWrite = true;
    this.mesh.material = this.material;
    this.mesh.hasVertexAlpha = true;
    this.mesh.isPickable = false;
    this.mesh.alwaysSelectAsActiveMesh = true;
    this.mesh.setEnabled(false);
  }

  setSurfaces(meshes: readonly AbstractMesh[], heightAt?: (x: number, z: number) => number, getSurfaceAt?: (x: number, z: number) => number): void {
    this.surfaces.clear();
    for (const mesh of meshes) {
      mesh.computeWorldMatrix(true);
      this.surfaces.add(mesh);
    }
    this.heightAt = heightAt;
    this.getSurfaceAt = getSurfaceAt;
    if (this.marks.active > 0) {
      this.marks.clear();
      this.positions.fill(0);
      this.colors.fill(0);
      this.positionDirty = true;
    }
    this.mesh.setEnabled(false);
  }

  private project(x: number, y: number, z: number, packedNormal: number): 'mesh' | 'terrain' | null {
    const normal = decodeImpactNormal(packedNormal, this.wireNormal);
    this.ray.origin.set(x + normal.x * PROBE_M, y + normal.y * PROBE_M, z + normal.z * PROBE_M);
    normal.negateToRef(this.ray.direction);
    if (this.surfaces.size > 0) {
      const hit = this.scene.pickWithRay(this.ray, this.isReceiver);
      if (hit?.pickedPoint && hit.faceId >= 0) {
        const surfaceNormal = hit.getNormal(true, false);
        if (surfaceNormal && Vector3.Dot(normal, surfaceNormal) > 0.75) {
          this.center.copyFrom(hit.pickedPoint);
          this.normal.copyFrom(surfaceNormal);
          return 'mesh';
        }
      }
    }
    const heightAt = this.heightAt;
    if (!heightAt || normal.y < 0.35) return null;
    const height = heightAt(x, z);
    if (!Number.isFinite(height) || Math.abs(y - height) > PROBE_M) return null;
    // The terrain's visible height is displaced on the GPU. Orient to its CPU height sampler.
    const step = 0.1;
    this.normal.set(heightAt(x - step, z) - heightAt(x + step, z), 2 * step, heightAt(x, z - step) - heightAt(x, z + step)).normalize();
    if (!(Vector3.Dot(normal, this.normal) > 0.75)) return null;
    this.center.set(x, height, z);
    return 'terrain';
  }

  /** Sample the static receiver for impact dust and leave a mark if it is within decal range. */
  impact(x: number, y: number, z: number, packedNormal: number, material: number, eye: Vector3): ImpactPreset | undefined {
    const distance = Math.hypot(x - eye.x, y - eye.y, z - eye.z);
    if (!(distance <= FAR_MERGE_M)) return;
    const surface = this.project(x, y, z, packedNormal);
    if (!surface) return;
    const preset = surface === 'terrain' && this.getSurfaceAt ? terrainImpactPreset(this.getSurfaceAt(x, z)) : dustPreset(material);
    if (distance < DECAL_DISTANCE_M) this.place(material, preset);
    return preset;
  }

  private place(material: number, preset: ImpactPreset): void {
    const { center, normal, tangent, bitangent } = this;
    const slot = this.marks.place(center.x, center.y, center.z, material);
    const style = STYLES[preset];
    if (Math.abs(normal.y) < 0.9) tangent.set(normal.z, 0, -normal.x);
    else tangent.set(0, -normal.z, normal.y);
    tangent.normalize();
    Vector3.CrossToRef(normal, tangent, bitangent);
    const cx = center.x + normal.x * DEPTH_BIAS_M;
    const cy = center.y + normal.y * DEPTH_BIAS_M;
    const cz = center.z + normal.z * DEPTH_BIAS_M;
    for (let i = 0; i < VERTICES; i++) {
      const vertex = slot * VERTICES + i;
      const outer = i > SIDES;
      const [u, v] = UNIT_CIRCLE[(i + SIDES - 1) % SIDES]!;
      const radius = i === 0 ? 0 : outer ? style.radius : style.radius * 0.52;
      this.positions[vertex * 3] = cx + radius * (u * tangent.x + v * bitangent.x);
      this.positions[vertex * 3 + 1] = cy + radius * (u * tangent.y + v * bitangent.y);
      this.positions[vertex * 3 + 2] = cz + radius * (u * tangent.z + v * bitangent.z);
      for (let channel = 0; channel < 3; channel++) this.colors[vertex * 4 + channel] = i === 0 ? 0.025 : style.rim[channel]!;
    }
    this.fades[slot] = -1;
    this.positionDirty = true;
  }

  update(eye: Vector3): void {
    let visible = false;
    let colorDirty = false;
    const { pos, active } = this.marks;
    for (let i = 0; i < active; i++) {
      const d = Math.hypot(pos[i * 3]! - eye.x, pos[i * 3 + 1]! - eye.y, pos[i * 3 + 2]! - eye.z);
      const fade = Math.fround(Math.min(1, Math.max(0, (DECAL_DISTANCE_M - d) / (DECAL_DISTANCE_M - FADE_START_M))));
      visible ||= fade > 0;
      if (this.fades[i] === fade) continue;
      this.fades[i] = fade;
      colorDirty = true;
      const base = i * VERTICES;
      for (let j = 0; j <= SIDES; j++) this.colors[(base + j) * 4 + 3] = fade * (j === 0 ? 0.91 : 0.72);
    }
    if (this.positionDirty) this.mesh.updateVerticesData(VertexBuffer.PositionKind, this.positions);
    if (this.positionDirty || colorDirty) this.mesh.updateVerticesData(VertexBuffer.ColorKind, this.colors);
    this.positionDirty = false;
    this.mesh.setEnabled(visible);
  }

  dispose(): void {
    this.mesh.dispose();
    this.material.dispose();
  }
}
