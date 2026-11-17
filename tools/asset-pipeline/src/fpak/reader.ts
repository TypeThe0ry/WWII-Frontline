import { createHash } from 'node:crypto';
import {
  FOOTER_BYTES, FPAK_VERSION, FpakFormatError, IntegrityError, INDEX_FIXED_BYTES, KIND_NAMES, MAGIC_END, bytesEqual, crc32,
} from './format.ts';
import type { RangeSource } from './sources.ts';
import type { IndexEntry } from './writer.ts';

export interface Footer {
  indexOffset: number;
  indexBytes: number;
  entryCount: number;
  crc: number;
  version: number;
}

export function parseFooter(b: Uint8Array): Footer {
  if (b.byteLength !== FOOTER_BYTES) throw new FpakFormatError(`footer must be ${FOOTER_BYTES} bytes`);
  if (!bytesEqual(b.subarray(28, 32), MAGIC_END)) throw new FpakFormatError('not a .fpak file (end magic mismatch)');
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const version = dv.getUint16(20, true);
  if (version !== FPAK_VERSION) throw new FpakFormatError(`unsupported .fpak version ${version} (this reader knows ${FPAK_VERSION})`);
  return {
    indexOffset: Number(dv.getBigUint64(0, true)),
    indexBytes: dv.getUint32(8, true),
    entryCount: dv.getUint32(12, true),
    crc: dv.getUint32(16, true),
    version,
  };
}

export function parseIndex(b: Uint8Array, count: number): IndexEntry[] {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const out: IndexEntry[] = [];
  let p = 0;
  for (let i = 0; i < count; i++) {
    if (p + INDEX_FIXED_BYTES > b.byteLength) throw new FpakFormatError('index truncated');
    const idLen = dv.getUint16(p, true);
    const kind = KIND_NAMES[dv.getUint8(p + 2)];
    if (!kind) throw new FpakFormatError(`unknown entry kind ${dv.getUint8(p + 2)}`);
    if (p + INDEX_FIXED_BYTES + idLen > b.byteLength) throw new FpakFormatError('index truncated');
    out.push({
      logical_id: new TextDecoder().decode(b.subarray(p + INDEX_FIXED_BYTES, p + INDEX_FIXED_BYTES + idLen)),
      kind,
      codec: dv.getUint16(p + 4, true),
      offset: dv.getBigUint64(p + 6, true),
      bytes: dv.getUint32(p + 14, true),
      sha8: b.slice(p + 18, p + 26),
    });
    p += INDEX_FIXED_BYTES + idLen;
  }
  if (p !== b.byteLength) throw new FpakFormatError('index has trailing bytes');
  return out;
}

export interface ReadOptions {
  /** Check `sha256[0:8]` of every payload read (default true). */
  verify?: boolean;
}

/** Opens a pak through a RangeSource: one footer read + one index read, then one Range read per entry. */
export class FpakReader {
  readonly #src: RangeSource;
  readonly #index: IndexEntry[];
  readonly #byId = new Map<string, IndexEntry>();
  readonly #verify: boolean;
  readonly footer: Footer;

  private constructor(src: RangeSource, footer: Footer, index: IndexEntry[], verify: boolean) {
    this.#src = src;
    this.footer = footer;
    this.#index = index;
    this.#verify = verify;
    for (const e of index) this.#byId.set(e.logical_id, e);
  }

  static async open(src: RangeSource, opts: ReadOptions = {}): Promise<FpakReader> {
    const size = await src.size();
    if (size < FOOTER_BYTES) throw new FpakFormatError('file too small to be a .fpak');
    const footer = parseFooter(await src.read(size - FOOTER_BYTES, size - 1));
    if (footer.indexOffset + footer.indexBytes + FOOTER_BYTES !== size) throw new FpakFormatError('footer does not match the file size');
    const raw = footer.indexBytes === 0 ? new Uint8Array(0) : await src.read(footer.indexOffset, footer.indexOffset + footer.indexBytes - 1);
    if (crc32(raw) !== footer.crc) throw new IntegrityError('index checksum mismatch');
    return new FpakReader(src, footer, parseIndex(raw, footer.entryCount), opts.verify ?? true);
  }

  entries(): IndexEntry[] {
    return this.#index;
  }

  has(id: string): boolean {
    return this.#byId.has(id);
  }

  entry(id: string): IndexEntry {
    const e = this.#byId.get(id);
    if (!e) throw new FpakFormatError(`no entry "${id}"`);
    return e;
  }

  async read(id: string): Promise<Uint8Array> {
    const e = this.entry(id);
    if (e.bytes === 0) return new Uint8Array(0);
    const start = Number(e.offset);
    const b = await this.#src.read(start, start + e.bytes - 1);
    if (b.byteLength !== e.bytes) throw new IntegrityError(`${id}: short read (${b.byteLength} of ${e.bytes} B)`, id);
    if (this.#verify && !bytesEqual(new Uint8Array(createHash('sha256').update(b).digest().subarray(0, 8)), e.sha8)) {
      throw new IntegrityError(`${id}: payload sha256 mismatch`, id);
    }
    return b;
  }
}
