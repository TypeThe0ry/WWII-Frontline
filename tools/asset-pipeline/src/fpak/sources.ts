/** Range backends shared by the reader: in-memory, Node file, HTTP `Range:` (with whole-file fallback). */
import { closeSync, fstatSync, openSync, readSync } from 'node:fs';

export interface RangeSource {
  /** Read bytes [start, endInclusive]. */
  read(start: number, endInclusive: number): Promise<Uint8Array>;
  size(): Promise<number>;
}

export class BufferSource implements RangeSource {
  readonly #b: Uint8Array;
  /** Bytes handed out by `read` (for transfer-size assertions). */
  bytesRead = 0;
  readonly log: [number, number][] = [];
  constructor(b: Uint8Array) {
    this.#b = b;
  }
  async read(start: number, endInclusive: number): Promise<Uint8Array> {
    const out = this.#b.subarray(start, endInclusive + 1);
    this.bytesRead += out.byteLength;
    this.log.push([start, endInclusive]);
    return out;
  }
  async size(): Promise<number> {
    return this.#b.byteLength;
  }
}

export class FileSource implements RangeSource {
  readonly #fd: number;
  readonly #size: number;
  constructor(path: string) {
    this.#fd = openSync(path, 'r');
    this.#size = fstatSync(this.#fd).size;
  }
  async read(start: number, endInclusive: number): Promise<Uint8Array> {
    const n = endInclusive - start + 1;
    const buf = new Uint8Array(n);
    let got = 0;
    while (got < n) {
      const r = readSync(this.#fd, buf, got, n - got, start + got);
      if (r === 0) break;
      got += r;
    }
    return got === n ? buf : buf.subarray(0, got);
  }
  async size(): Promise<number> {
    return this.#size;
  }
  close(): void {
    closeSync(this.#fd);
  }
}

/**
 * `fetch` + `Range: bytes=a-b`. A server that ignores Range (200 instead of 206) makes the source fall back to downloading
 * the whole file once and serving every later read from memory (ARCHITECTURE 11.8 streaming clause).
 */
export class HttpRangeSource implements RangeSource {
  readonly #url: string;
  readonly #fetch: typeof fetch;
  #whole: Uint8Array | undefined;
  #size: number | undefined;
  /** Response body bytes received (what the network carried). */
  bytesTransferred = 0;
  requests = 0;
  fellBack = false;
  constructor(url: string, fetchImpl: typeof fetch = fetch) {
    this.#url = url;
    this.#fetch = fetchImpl;
  }
  async #full(): Promise<Uint8Array> {
    if (!this.#whole) {
      const r = await this.#fetch(this.#url);
      if (!r.ok) throw new Error(`GET ${this.#url}: ${r.status}`);
      this.#whole = new Uint8Array(await r.arrayBuffer());
      this.bytesTransferred += this.#whole.byteLength;
      this.requests++;
      this.#size = this.#whole.byteLength;
      this.fellBack = true;
    }
    return this.#whole;
  }
  async size(): Promise<number> {
    if (this.#size !== undefined) return this.#size;
    const r = await this.#fetch(this.#url, { method: 'HEAD' });
    this.requests++;
    const len = Number(r.headers.get('content-length'));
    if (r.ok && Number.isFinite(len) && len > 0) {
      this.#size = len;
      return len;
    }
    return (await this.#full()).byteLength;
  }
  async read(start: number, endInclusive: number): Promise<Uint8Array> {
    if (this.#whole) return this.#whole.subarray(start, endInclusive + 1);
    const r = await this.#fetch(this.#url, { headers: { Range: `bytes=${start}-${endInclusive}` } });
    this.requests++;
    if (r.status === 206) {
      const b = new Uint8Array(await r.arrayBuffer());
      this.bytesTransferred += b.byteLength;
      if (b.byteLength !== endInclusive - start + 1) throw new Error(`short Range response (${b.byteLength} B)`);
      return b;
    }
    if (r.status === 200) {
      this.#whole = new Uint8Array(await r.arrayBuffer());
      this.bytesTransferred += this.#whole.byteLength;
      this.#size = this.#whole.byteLength;
      this.fellBack = true;
      return this.#whole.subarray(start, endInclusive + 1);
    }
    throw new Error(`GET ${this.#url} Range ${start}-${endInclusive}: ${r.status}`);
  }
}
