/**
 * Mesh compression (ARCHITECTURE 11.1 step 5, M1-002): dedup, prune, quantize (position 16 bit, uv 16 bit, normal 8 bit),
 * EXT_meshopt_compression. Quantized normals are written as 8-bit and filtered with the meshopt octahedral filter.
 */
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { Document, NodeIO, type Primitive } from '@gltf-transform/core';
import { EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu } from '@gltf-transform/extensions';
import { dedup, getBounds, prune, quantize } from '@gltf-transform/functions';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';

const ALL_EXTENSIONS = [EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu];

export interface Bounds {
  min: [number, number, number];
  max: [number, number, number];
}

export interface CompressResult {
  /** Diagonal of the bounding box before / after quantization (metres). */
  diagonalBefore: number;
  diagonalAfter: number;
  /** max |corner error| / diagonal: the card's 0.1 % criterion. */
  bboxError: number;
}

function diagonal(b: Bounds): number {
  return Math.hypot(b.max[0] - b.min[0], b.max[1] - b.min[1], b.max[2] - b.min[2]);
}

export function sceneBounds(doc: Document): Bounds {
  const scene = doc.getRoot().getDefaultScene() ?? doc.getRoot().listScenes()[0];
  if (!scene) return { min: [0, 0, 0], max: [0, 0, 0] };
  const b = getBounds(scene);
  return { min: [b.min[0], b.min[1], b.min[2]], max: [b.max[0], b.max[1], b.max[2]] };
}

/** Largest per-coordinate deviation between two boxes, relative to the diagonal of `ref`. */
export function bboxError(ref: Bounds, other: Bounds): number {
  const d = diagonal(ref) || 1;
  let e = 0;
  for (let k = 0; k < 3; k++) e = Math.max(e, Math.abs(ref.min[k]! - other.min[k]!), Math.abs(ref.max[k]! - other.max[k]!));
  return e / d;
}

/** Compress `doc` in place. Skinned / animated documents keep their data untouched except for dedup/prune. */
export async function compressMesh(doc: Document): Promise<CompressResult> {
  await MeshoptEncoder.ready;
  const before = sceneBounds(doc);
  await doc.transform(dedup(), prune());
  const skinned = doc.getRoot().listSkins().length > 0 || doc.getRoot().listAnimations().length > 0;
  if (!skinned) {
    await doc.transform(
      quantize({ quantizePosition: 16, quantizeTexcoord: 16, quantizeNormal: 8, quantizeColor: 8, quantizeWeight: 8, quantizeGeneric: 12 }),
    );
  }
  doc.createExtension(KHRMeshQuantization).setRequired(!skinned);
  doc
    .createExtension(EXTMeshoptCompression)
    .setRequired(true)
    .setEncoderOptions({ method: EXTMeshoptCompression.EncoderMethod.FILTER });
  const after = sceneBounds(doc);
  return { diagonalBefore: diagonal(before), diagonalAfter: diagonal(after), bboxError: bboxError(before, after) };
}

/** Serialize to a binary glb (meshopt-compressed when the extension is present). Returns the bytes. */
export async function writeGlb(doc: Document, path: string): Promise<number> {
  await MeshoptEncoder.ready;
  await MeshoptDecoder.ready;
  const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.encoder': MeshoptEncoder, 'meshopt.decoder': MeshoptDecoder });
  const glb = await io.writeBinary(doc);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, glb);
  return glb.byteLength;
}

export async function readGlb(path: string): Promise<Document> {
  await MeshoptDecoder.ready;
  const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.decoder': MeshoptDecoder });
  return io.read(path);
}

/** Triangle count of a primitive (0 for non-triangle modes). */
export function primitiveTris(p: Primitive): number {
  if (p.getMode() !== 4) return 0;
  return (p.getIndices()?.getCount() ?? p.getAttribute('POSITION')?.getCount() ?? 0) / 3;
}
