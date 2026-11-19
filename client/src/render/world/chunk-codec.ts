import { CHUNK_M, type ChunkCoord } from './chunk-streamer-core';

const HEIGHT_SAMPLES = (CHUNK_M + 1) ** 2;
const SPLAT_BYTES = CHUNK_M ** 2 * 8;
const COLLISION_BYTES = 16 + HEIGHT_SAMPLES * 2 + CHUNK_M ** 2;
const MAX_SECTION_BYTES = 1024 * 1024;
const FCHK = 0x4643484b;
const FCOL = 0x46434f4c;

export interface BakedChunk {
  /** Row-major quantized heights, including the shared boundary. */
  heights: Uint16Array;
  /** Eight planar 64×64 weights. */
  splat: Uint8Array;
  props: Uint8Array;
  vegetation: Uint8Array;
  hedges: Uint8Array;
}

async function inflate(bytes: Uint8Array, size: number): Promise<Uint8Array> {
  const compressed = new Blob([Uint8Array.from(bytes)]).stream();
  const reader = compressed.pipeThrough(new DecompressionStream('deflate')).getReader();
  const decoded = new Uint8Array(size);
  let offset = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (offset + value.length > size) throw new Error('Chunk section exceeds declared size');
      decoded.set(value, offset);
      offset += value.length;
    }
    if (offset !== size) throw new Error('Invalid decoded chunk section size');
    return decoded;
  } finally {
    await reader.cancel();
  }
}

export async function decodeBakedChunk(chunk: ChunkCoord, bytes: Uint8Array): Promise<BakedChunk> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (bytes.length < 12 || view.getUint32(0) !== FCHK || view.getUint8(4) !== 1 ||
      view.getUint16(6, true) !== chunk.bx || view.getUint16(8, true) !== chunk.bz || view.getUint16(10, true) !== 5) {
    throw new Error('Invalid FCHK v1 header');
  }
  const dataStart = 12 + 5 * 16;
  if (bytes.length < dataStart) throw new Error('Truncated FCHK section table');
  const sections: { offset: number; length: number; size: number }[] = [];
  let previousEnd = dataStart;
  for (let i = 0; i < 5; i++) {
    const p = 12 + i * 16;
    const offset = view.getUint32(p + 4, true);
    const length = view.getUint32(p + 8, true);
    const size = view.getUint32(p + 12, true);
    if (view.getUint8(p) !== i + 1 || view.getUint8(p + 1) !== 1 || offset !== previousEnd || length === 0 ||
        offset + length > bytes.length || size > MAX_SECTION_BYTES) throw new Error('Invalid FCHK section');
    previousEnd = offset + length;
    sections.push({ offset, length, size });
  }
  if (previousEnd !== bytes.length || sections[0]!.size !== HEIGHT_SAMPLES * 2 || sections[1]!.size !== SPLAT_BYTES) {
    throw new Error('Invalid FCHK payload size');
  }
  const [delta, splat, props, vegetation, hedges] = await Promise.all(sections.map(({ offset, length, size }) =>
    inflate(bytes.subarray(offset, offset + length), size)));
  const heights = new Uint16Array(HEIGHT_SAMPLES);
  for (let i = 0; i < heights.length; i++) {
    const previous = i === 0 ? 0 : heights[i % (CHUNK_M + 1) === 0 ? i - CHUNK_M - 1 : i - 1]!;
    heights[i] = previous + delta![i]! + (delta![i + HEIGHT_SAMPLES]! << 8);
  }
  return { heights, splat: splat!, props: props!, vegetation: vegetation!, hedges: hedges! };
}

export async function decodeBakedCollision(bytes: Uint8Array): Promise<Uint8Array> {
  const raw = await inflate(bytes, COLLISION_BYTES);
  const view = new DataView(raw.buffer);
  const min = view.getFloat32(8, true);
  const max = view.getFloat32(12, true);
  if (view.getUint32(0) !== FCOL || view.getUint8(4) !== 1 || view.getUint16(6, true) !== CHUNK_M + 1 ||
      !Number.isFinite(min) || !Number.isFinite(max) || min > max) throw new Error('Invalid FCOL v1 header');
  return raw;
}
