import { createHash } from 'node:crypto';
import {
  ALIGN, FPAK_VERSION, FOOTER_BYTES, FpakFormatError, HEADER_BYTES, INDEX_FIXED_BYTES, Kind, LOGICAL_ID_RE, MAGIC, MAGIC_END,
  MAX_ENTRY_BYTES, MAX_LOGICAL_ID_BYTES, alignUp, crc32, type KindName,
} from './format.ts';

export interface IndexEntry {
  logical_id: string;
  kind: KindName;
  offset: bigint;
  bytes: number;
  codec: number;
  /** First 8 bytes of the payload's sha256. */
  sha8: Uint8Array;
}

export interface FpakBuild {
  bytes: Uint8Array;
  index: IndexEntry[];
  /** sha256 of the whole file (decides the published file name). */
  sha256: string;
}

/** Content-addressed publish path: `/a/{sha[0:2]}/{sha[2:16]}.fpak` (ARCHITECTURE 11.2). */
export function publishPath(sha256: string): string {
  return `/a/${sha256.slice(0, 2)}/${sha256.slice(2, 16)}.fpak`;
}

export function sha8Of(payload: Uint8Array): Uint8Array {
  return new Uint8Array(createHash('sha256').update(payload).digest().subarray(0, 8));
}

/** Accumulates entries; `finish()` lays out header, 16-byte aligned entries, index and footer. */
export class FpakWriter {
  readonly #entries: { id: string; kind: KindName; codec: number; payload: Uint8Array }[] = [];
  readonly #ids = new Set<string>();
  #finished = false;

  add(id: string, kind: KindName, codec: number, payload: Uint8Array): void {
    if (this.#finished) throw new FpakFormatError('writer already finished');
    if (!LOGICAL_ID_RE.test(id) || Buffer.byteLength(id) > MAX_LOGICAL_ID_BYTES) {
      throw new FpakFormatError(`invalid logical id "${id}" (lower-case snake_case with optional @variant, no paths or vendor names)`);
    }
    if (!(kind in Kind)) throw new FpakFormatError(`unknown kind "${String(kind)}"`);
    if (!Number.isInteger(codec) || codec < 0 || codec > 0xffff) throw new FpakFormatError(`codec ${codec} out of range`);
    if (payload.byteLength > MAX_ENTRY_BYTES) throw new FpakFormatError(`${id}: entry larger than 4 GiB`);
    if (this.#ids.has(id)) throw new FpakFormatError(`duplicate logical id "${id}"`);
    this.#ids.add(id);
    this.#entries.push({ id, kind, codec, payload });
  }

  get size(): number {
    return this.#entries.reduce((s, e) => s + alignUp(e.payload.byteLength), HEADER_BYTES);
  }

  get count(): number {
    return this.#entries.length;
  }

  finish(): FpakBuild {
    this.#finished = true;
    const index: IndexEntry[] = [];
    let off = HEADER_BYTES;
    for (const e of this.#entries) {
      index.push({ logical_id: e.id, kind: e.kind, offset: BigInt(off), bytes: e.payload.byteLength, codec: e.codec, sha8: sha8Of(e.payload) });
      off += alignUp(e.payload.byteLength, ALIGN);
    }
    const indexOffset = off;
    const indexBytes = index.reduce((s, e) => s + INDEX_FIXED_BYTES + Buffer.byteLength(e.logical_id), 0);
    const total = indexOffset + indexBytes + FOOTER_BYTES;
    const out = new Uint8Array(total);
    const dv = new DataView(out.buffer);
    out.set(MAGIC, 0);
    dv.setUint16(4, FPAK_VERSION, true);
    dv.setUint16(6, 0, true); // flags
    dv.setUint32(8, index.length, true);
    dv.setBigUint64(16, BigInt(indexOffset), true);
    this.#entries.forEach((e, i) => out.set(e.payload, Number((index[i] as IndexEntry).offset)));
    let p = indexOffset;
    for (const e of index) {
      const id = Buffer.from(e.logical_id);
      dv.setUint16(p, id.length, true);
      dv.setUint8(p + 2, Kind[e.kind]);
      dv.setUint16(p + 4, e.codec, true);
      dv.setBigUint64(p + 6, e.offset, true);
      dv.setUint32(p + 14, e.bytes, true);
      out.set(e.sha8, p + 18);
      out.set(id, p + INDEX_FIXED_BYTES);
      p += INDEX_FIXED_BYTES + id.length;
    }
    const f = indexOffset + indexBytes;
    dv.setBigUint64(f, BigInt(indexOffset), true);
    dv.setUint32(f + 8, indexBytes, true);
    dv.setUint32(f + 12, index.length, true);
    dv.setUint32(f + 16, crc32(out.subarray(indexOffset, indexOffset + indexBytes)), true);
    dv.setUint16(f + 20, FPAK_VERSION, true);
    out.set(MAGIC_END, f + 28);
    return { bytes: out, index, sha256: createHash('sha256').update(out).digest('hex') };
  }
}
