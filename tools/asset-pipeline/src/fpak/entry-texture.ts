/**
 * Texture entry layout v1 (codec `Codec.textureV1`): the KTX2 payload without the standard container header, re-ordered
 * smallest mip first (so a streaming loader can show the low mips early), behind an own small header. Not a protection
 * mechanism (see format.ts).
 */
import { read as readKtx2, write as writeKtx2, type KTX2Container, type KTX2Level } from 'ktx-parse';
import { FpakFormatError } from './format.ts';

export const TEXTURE_LAYOUT_VERSION = 1;

export interface TextureEntry {
  vkFormat: number;
  typeSize: number;
  width: number;
  height: number;
  depth: number;
  layerCount: number;
  faceCount: number;
  supercompressionScheme: number;
  /** Raw basic-format DFD block(s) (colour model, transfer function). */
  dfd: Uint8Array;
  /** Supercompression global data (ETC1S codebooks), may be empty. */
  sgd: Uint8Array;
  /** Pipeline tags carried over from the KTX2 key/value data. */
  channels: number;
  layout: string;
  /** Levels in KTX2 order (level 0 = largest). */
  levels: { data: Uint8Array; uncompressedByteLength: number }[];
}

/** Pure-JS ktx-parse `write` emits the DFD from structured data; keep the structured form by re-reading our own bytes. */
export function textureEntryFromKtx2(file: Uint8Array): TextureEntry {
  const c = readKtx2(file);
  const tag = (k: string) => {
    const v = c.keyValue[k];
    return v === undefined ? '' : typeof v === 'string' ? v : new TextDecoder().decode(v).replace(/\0+$/, '');
  };
  return {
    vkFormat: c.vkFormat, typeSize: c.typeSize, width: c.pixelWidth, height: c.pixelHeight, depth: c.pixelDepth,
    layerCount: c.layerCount, faceCount: c.faceCount, supercompressionScheme: c.supercompressionScheme,
    // The DFD is stored as the container's own serialisation of block 0 (see dfdBytes below).
    dfd: dfdBytes(c), sgd: c.globalData ? globalDataBytes(c) : new Uint8Array(0),
    channels: Number(tag('fl_channels') || 0), layout: tag('fl_layout'),
    levels: c.levels.map((l) => ({ data: l.levelData, uncompressedByteLength: l.uncompressedByteLength })),
  };
}

function dfdBytes(c: KTX2Container): Uint8Array {
  // Serialise the container once and cut out the DFD region (header: dfdByteOffset at 48, dfdByteLength at 52).
  const raw = writeKtx2(c);
  const dv = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  const off = dv.getUint32(48, true);
  const len = dv.getUint32(52, true);
  return raw.slice(off, off + len);
}

function globalDataBytes(c: KTX2Container): Uint8Array {
  const raw = writeKtx2(c);
  const dv = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  const off = Number(dv.getBigUint64(64, true));
  const len = Number(dv.getBigUint64(72, true));
  return raw.slice(off, off + len);
}

export function encodeTextureEntry(t: TextureEntry): Uint8Array {
  const layout = Buffer.from(t.layout);
  const head = 48 + 2 + layout.length;
  const levelTable = t.levels.length * 12;
  const dfdAt = Math.ceil((head + levelTable) / 4) * 4;
  const sgdAt = Math.ceil((dfdAt + t.dfd.length) / 4) * 4;
  const dataAt = Math.ceil((sgdAt + t.sgd.length) / 16) * 16;
  const total = dataAt + t.levels.reduce((s, l) => s + Math.ceil(l.data.length / 4) * 4, 0);
  const out = new Uint8Array(total);
  const dv = new DataView(out.buffer);
  dv.setUint8(0, TEXTURE_LAYOUT_VERSION);
  dv.setUint8(1, t.supercompressionScheme);
  dv.setUint8(2, t.levels.length);
  dv.setUint8(3, t.channels);
  dv.setUint32(4, t.vkFormat, true);
  dv.setUint32(8, t.typeSize, true);
  dv.setUint32(12, t.width, true);
  dv.setUint32(16, t.height, true);
  dv.setUint32(20, t.depth, true);
  dv.setUint32(24, t.layerCount, true);
  dv.setUint32(28, t.faceCount, true);
  dv.setUint32(32, dfdAt, true);
  dv.setUint32(36, t.dfd.length, true);
  dv.setUint32(40, sgdAt, true);
  dv.setUint32(44, t.sgd.length, true);
  dv.setUint16(48, layout.length, true);
  out.set(layout, 50);
  out.set(t.dfd, dfdAt);
  out.set(t.sgd, sgdAt);
  // Level table: smallest mip first. Each record: data offset (from dataAt), byte length, uncompressed byte length (u32 x3).
  let off = 0;
  const order = t.levels.map((_, i) => i).reverse();
  const recs: { i: number; at: number }[] = [];
  for (const i of order) {
    const l = t.levels[i] as TextureEntry['levels'][number];
    out.set(l.data, dataAt + off);
    recs.push({ i, at: off });
    off += Math.ceil(l.data.length / 4) * 4;
  }
  let q = head;
  for (const { i, at } of recs.sort((a, b) => a.i - b.i)) {
    const l = t.levels[i] as TextureEntry['levels'][number];
    dv.setUint32(q, at, true);
    dv.setUint32(q + 4, l.data.length, true);
    dv.setUint32(q + 8, l.uncompressedByteLength, true);
    q += 12;
  }
  void dataAt;
  return out.subarray(0, total);
}

export function decodeTextureEntry(b: Uint8Array): TextureEntry {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  if (dv.getUint8(0) !== TEXTURE_LAYOUT_VERSION) throw new FpakFormatError('unknown texture layout version');
  const nLevels = dv.getUint8(2);
  const dfdAt = dv.getUint32(32, true);
  const dfdLen = dv.getUint32(36, true);
  const sgdAt = dv.getUint32(40, true);
  const sgdLen = dv.getUint32(44, true);
  const layoutLen = dv.getUint16(48, true);
  const layout = new TextDecoder().decode(b.subarray(50, 50 + layoutLen));
  const head = 48 + 2 + layoutLen;
  const dataAt = Math.ceil((sgdAt + sgdLen) / 16) * 16;
  const levels: TextureEntry['levels'] = [];
  for (let i = 0; i < nLevels; i++) {
    const at = dv.getUint32(head + i * 12, true);
    const len = dv.getUint32(head + i * 12 + 4, true);
    levels.push({ data: b.slice(dataAt + at, dataAt + at + len), uncompressedByteLength: dv.getUint32(head + i * 12 + 8, true) });
  }
  return {
    vkFormat: dv.getUint32(4, true), typeSize: dv.getUint32(8, true), width: dv.getUint32(12, true), height: dv.getUint32(16, true),
    depth: dv.getUint32(20, true), layerCount: dv.getUint32(24, true), faceCount: dv.getUint32(28, true),
    supercompressionScheme: dv.getUint8(1), dfd: b.slice(dfdAt, dfdAt + dfdLen), sgd: b.slice(sgdAt, sgdAt + sgdLen),
    channels: dv.getUint8(3), layout, levels,
  };
}

export type { KTX2Level };
