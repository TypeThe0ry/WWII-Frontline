/**
 * Re-assembles a standard KTX2 file from a decoded texture entry (texture layout v1 keeps the payload without the
 * container header, smallest mip first), so the Basis transcoder can read it. Layout per the KTX2 spec: identifier,
 * 9-word header, index, level index (largest mip first), DFD, (empty KVD), SGD, then level data smallest mip first.
 */
import type { DecodedTexture } from './entry-decode';

const IDENTIFIER = [0xab, 0x4b, 0x54, 0x58, 0x20, 0x32, 0x30, 0xbb, 0x0d, 0x0a, 0x1a, 0x0a];
const align = (n: number, a: number) => Math.ceil(n / a) * a;

export function rebuildKtx2(t: DecodedTexture): Uint8Array {
  const n = t.levels.length;
  const levelIndexAt = 80;
  const dfdAt = levelIndexAt + n * 24;
  const sgdAt = align(dfdAt + t.dfd.length, 8);
  const dataAt = align(sgdAt + t.sgd.length, 8);
  // Level data: smallest mip first (level n-1 ... 0), each 8-byte aligned.
  const offsets = new Array<number>(n);
  let cur = dataAt;
  for (let i = n - 1; i >= 0; i--) {
    offsets[i] = cur;
    cur = align(cur + (t.levels[i] as DecodedTexture['levels'][number]).data.length, 8);
  }
  const out = new Uint8Array(cur);
  const dv = new DataView(out.buffer);
  out.set(IDENTIFIER, 0);
  dv.setUint32(12, t.vkFormat, true);
  dv.setUint32(16, t.typeSize, true);
  dv.setUint32(20, t.width, true);
  dv.setUint32(24, t.height, true);
  dv.setUint32(28, t.depth, true);
  dv.setUint32(32, t.layerCount, true);
  dv.setUint32(36, t.faceCount, true);
  dv.setUint32(40, n, true);
  dv.setUint32(44, t.supercompressionScheme, true);
  dv.setUint32(48, dfdAt, true);
  dv.setUint32(52, t.dfd.length, true);
  dv.setUint32(56, 0, true); // kvdByteOffset
  dv.setUint32(60, 0, true); // kvdByteLength
  dv.setBigUint64(64, BigInt(t.sgd.length ? sgdAt : 0), true);
  dv.setBigUint64(72, BigInt(t.sgd.length), true);
  t.levels.forEach((l, i) => {
    const q = levelIndexAt + i * 24;
    dv.setBigUint64(q, BigInt(offsets[i] as number), true);
    dv.setBigUint64(q + 8, BigInt(l.data.length), true);
    dv.setBigUint64(q + 16, BigInt(l.uncompressedByteLength), true);
    out.set(l.data, offsets[i] as number);
  });
  out.set(t.dfd, dfdAt);
  out.set(t.sgd, sgdAt);
  return out;
}
