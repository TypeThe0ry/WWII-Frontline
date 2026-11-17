/**
 * Browser `.fpak` reader (M1-006). Byte layout: tools/asset-pipeline/src/fpak/SPEC.md. No Node APIs.
 * Opening = footer read + one index read; each entry = one Range read. A server that ignores `Range` (200) makes the source
 * download the file once and serve every later read from memory (ARCHITECTURE 11.8).
 */
export const HEADER_BYTES = 32;
export const FOOTER_BYTES = 32;

export interface RangeSource {
  size(): Promise<number>;
  /** Inclusive byte range. */
  read(start: number, endInclusive: number, signal?: AbortSignal): Promise<Uint8Array>;
}

export class FpakError extends Error {
  constructor(message: string, readonly status?: number) {
    super(message);
    this.name = 'FpakError';
  }
}

export type Fetcher = (url: string, init?: RequestInit) => Promise<Response>;

export class FetchRangeSource implements RangeSource {
  #whole: Uint8Array | undefined;
  #size: number | undefined;
  /** Bytes actually transferred (progress reporting). */
  transferred = 0;
  rangeSupported: boolean | undefined;
  constructor(readonly url: string, private readonly fetcher: Fetcher = (u, i) => fetch(u, i), private readonly onBytes?: (n: number) => void) {}

  async #downloadWhole(signal?: AbortSignal): Promise<Uint8Array> {
    if (this.#whole) return this.#whole;
    const r = await this.fetcher(this.url, signal ? { signal } : undefined);
    if (!r.ok) throw new FpakError(`${this.url}: HTTP ${r.status}`, r.status);
    const b = new Uint8Array(await r.arrayBuffer());
    this.#whole = b;
    this.#size = b.byteLength;
    this.#count(b.byteLength);
    return b;
  }

  #count(n: number): void {
    this.transferred += n;
    this.onBytes?.(n);
  }

  async size(): Promise<number> {
    if (this.#size !== undefined) return this.#size;
    // Ask for the last byte: 206 + Content-Range tells the size without a HEAD request.
    const r = await this.fetcher(this.url, { headers: { Range: 'bytes=-1' } });
    if (r.status === 206) {
      const m = /\/(\d+)\s*$/.exec(r.headers.get('Content-Range') ?? '');
      if (m) {
        this.rangeSupported = true;
        this.#size = Number(m[1]);
        this.#count((await r.arrayBuffer()).byteLength);
        return this.#size;
      }
    }
    if (!r.ok) throw new FpakError(`${this.url}: HTTP ${r.status}`, r.status);
    this.rangeSupported = false;
    const b = new Uint8Array(await r.arrayBuffer());
    this.#whole = b;
    this.#size = b.byteLength;
    this.#count(b.byteLength);
    return this.#size;
  }

  async read(start: number, endInclusive: number, signal?: AbortSignal): Promise<Uint8Array> {
    if (this.#whole) return this.#whole.subarray(start, endInclusive + 1);
    const r = await this.fetcher(this.url, { headers: { Range: `bytes=${start}-${endInclusive}` }, ...(signal ? { signal } : {}) });
    if (r.status === 206) {
      this.rangeSupported = true;
      const b = new Uint8Array(await r.arrayBuffer());
      this.#count(b.byteLength);
      return b;
    }
    if (r.status === 200) {
      // Range ignored: this response is the whole file.
      this.rangeSupported = false;
      const b = new Uint8Array(await r.arrayBuffer());
      this.#whole = b;
      this.#size = b.byteLength;
      this.#count(b.byteLength);
      return b.subarray(start, endInclusive + 1);
    }
    throw new FpakError(`${this.url}: HTTP ${r.status}`, r.status);
  }

  /** Force the whole-file fallback (used when a proxy breaks Range responses). */
  async readWhole(signal?: AbortSignal): Promise<Uint8Array> {
    return this.#downloadWhole(signal);
  }
}

export class BytesSource implements RangeSource {
  constructor(private readonly bytes: Uint8Array) {}
  async size(): Promise<number> {
    return this.bytes.byteLength;
  }
  async read(start: number, endInclusive: number): Promise<Uint8Array> {
    return this.bytes.subarray(start, endInclusive + 1);
  }
}

export interface PakEntry {
  id: string;
  kind: 'mesh' | 'texture' | 'audio' | 'meta';
  codec: number;
  offset: number;
  bytes: number;
  sha8: string;
}

const KINDS = ['mesh', 'texture', 'audio', 'meta'] as const;
const MAGIC_END = [0x4b, 0x41, 0x50, 0x46];
let table: Uint32Array | undefined;
export function crc32(data: Uint8Array): number {
  if (!table) {
    table = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c >>> 0;
    }
  }
  let c = 0xffffffff;
  for (let i = 0; i < data.length; i++) c = (table[(c ^ (data[i] as number)) & 0xff] as number) ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

export interface Footer {
  indexOffset: number;
  indexBytes: number;
  entryCount: number;
  crc: number;
  version: number;
}

export function parseFooter(b: Uint8Array): Footer {
  if (b.byteLength !== FOOTER_BYTES) throw new FpakError('footer must be 32 bytes');
  if (MAGIC_END.some((v, i) => b[28 + i] !== v)) throw new FpakError('bad end magic (not a .fpak)');
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const indexOffset = Number(v.getBigUint64(0, true));
  const version = v.getUint16(20, true);
  if (version !== 1) throw new FpakError(`unsupported .fpak version ${version}`);
  return { indexOffset, indexBytes: v.getUint32(8, true), entryCount: v.getUint32(12, true), crc: v.getUint32(16, true), version };
}

export function parseIndex(b: Uint8Array, count: number): PakEntry[] {
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const out: PakEntry[] = [];
  const dec = new TextDecoder();
  let p = 0;
  for (let i = 0; i < count; i++) {
    if (p + 26 > b.byteLength) throw new FpakError('index truncated');
    const idLen = v.getUint16(p, true);
    const kind = KINDS[v.getUint8(p + 2)];
    if (!kind) throw new FpakError(`unknown entry kind ${v.getUint8(p + 2)}`);
    const sha8 = Array.from(b.subarray(p + 18, p + 26), (x) => x.toString(16).padStart(2, '0')).join('');
    if (p + 26 + idLen > b.byteLength) throw new FpakError('index truncated');
    out.push({ id: dec.decode(b.subarray(p + 26, p + 26 + idLen)), kind, codec: v.getUint16(p + 4, true), offset: Number(v.getBigUint64(p + 6, true)), bytes: v.getUint32(p + 14, true), sha8 });
    p += 26 + idLen;
  }
  if (p !== b.byteLength) throw new FpakError('index has trailing bytes');
  return out;
}

export class FpakReader {
  readonly #byId = new Map<string, PakEntry>();
  private constructor(readonly source: RangeSource, readonly footer: Footer, readonly entries: PakEntry[]) {
    for (const e of entries) this.#byId.set(e.id, e);
  }

  static async open(source: RangeSource, signal?: AbortSignal): Promise<FpakReader> {
    const size = await source.size();
    if (size < FOOTER_BYTES) throw new FpakError('file too small to be a .fpak');
    const footer = parseFooter(await source.read(size - FOOTER_BYTES, size - 1, signal));
    if (footer.indexOffset + footer.indexBytes + FOOTER_BYTES !== size) throw new FpakError('footer does not match the file size');
    const raw = footer.indexBytes === 0 ? new Uint8Array(0) : await source.read(footer.indexOffset, footer.indexOffset + footer.indexBytes - 1, signal);
    if (crc32(raw) !== footer.crc) throw new FpakError('index checksum mismatch');
    return new FpakReader(source, footer, parseIndex(raw, footer.entryCount));
  }

  has(id: string): boolean {
    return this.#byId.has(id);
  }

  entry(id: string): PakEntry {
    const e = this.#byId.get(id);
    if (!e) throw new FpakError(`no entry "${id}"`, 404);
    return e;
  }

  async read(id: string, signal?: AbortSignal): Promise<Uint8Array> {
    const e = this.entry(id);
    if (e.bytes === 0) return new Uint8Array(0);
    const b = await this.source.read(e.offset, e.offset + e.bytes - 1, signal);
    if (b.byteLength !== e.bytes) throw new FpakError(`${id}: short read (${b.byteLength} of ${e.bytes} B)`);
    return b;
  }
}
