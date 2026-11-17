import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { BufferSource, FpakReader, FpakWriter, HttpRangeSource, bytesEqual, FOOTER_BYTES } from '../src/fpak/index.ts';

function rng(seed: number) {
  let s = seed >>> 0;
  return () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 4294967296);
}

const r = rng(11);
const entries = Array.from({ length: 220 }, (_, i) => ({
  id: `prop_${i}@lod0`,
  payload: Uint8Array.from({ length: 1 + Math.floor(r() * 4000) }, () => Math.floor(r() * 256)),
}));
const w = new FpakWriter();
for (const e of entries) w.add(e.id, 'mesh', 1, e.payload);
const pak = w.finish();
// 16 MB pack with a few large entries for the transfer-size test.
const bigW = new FpakWriter();
for (let i = 0; i < 8; i++) bigW.add(`big_${i}@lod0`, 'mesh', 1, Uint8Array.from({ length: 2 * 1024 * 1024 }, (_, k) => (k * 31 + i) & 255));
const big = bigW.finish();

let server: Server;
let base = '';
const hits: { range: string | undefined; method: string }[] = [];
let ignoreRange = false;
const files: Record<string, Uint8Array> = { '/a.fpak': pak.bytes, '/big.fpak': big.bytes };

beforeAll(async () => {
  server = createServer((req, res) => {
    const body = files[req.url ?? ''];
    hits.push({ range: req.headers.range, method: req.method ?? '' });
    if (!body) return void res.writeHead(404).end();
    if (req.method === 'HEAD') return void res.writeHead(200, { 'content-length': body.length }).end();
    const m = /^bytes=(\d+)-(\d+)$/.exec(req.headers.range ?? '');
    if (!m || ignoreRange) return void res.writeHead(200, { 'content-length': body.length }).end(Buffer.from(body));
    const a = Number(m[1]);
    const b = Math.min(Number(m[2]), body.length - 1);
    res.writeHead(206, { 'content-range': `bytes ${a}-${b}/${body.length}`, 'content-length': b - a + 1 }).end(Buffer.from(body.subarray(a, b + 1)));
  });
  await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});
afterAll(() => new Promise<void>((ok) => server.close(() => ok())));

describe('random Range reads (acceptance 2, 6)', () => {
  it('1000 random entry reads in random order match the source (buffer backend)', async () => {
    const reader = await FpakReader.open(new BufferSource(pak.bytes));
    const rand = rng(5);
    for (let i = 0; i < 1000; i++) {
      const e = entries[Math.floor(rand() * entries.length)]!;
      expect(bytesEqual(await reader.read(e.id), e.payload)).toBe(true);
    }
  });

  it('1000 random sub-range reads over HTTP 206 equal the source bytes', async () => {
    ignoreRange = false;
    const src = new HttpRangeSource(`${base}/a.fpak`);
    const reader = await FpakReader.open(src);
    const rand = rng(9);
    for (let i = 0; i < 1000; i++) {
      const e = entries[Math.floor(rand() * entries.length)]!;
      expect(bytesEqual(await reader.read(e.id), e.payload)).toBe(true);
      const a = Math.floor(rand() * pak.bytes.length);
      const b = Math.min(pak.bytes.length - 1, a + Math.floor(rand() * 500));
      expect(bytesEqual(await src.read(a, b), pak.bytes.subarray(a, b + 1))).toBe(true);
    }
    expect(src.fellBack).toBe(false);
  });

  it('16 MB pack: open + one entry transfers <= index + entry + 64 B', async () => {
    ignoreRange = false;
    hits.length = 0;
    const src = new HttpRangeSource(`${base}/big.fpak`);
    const reader = await FpakReader.open(src);
    const e = await reader.read('big_5@lod0');
    expect(e.length).toBe(2 * 1024 * 1024);
    const indexBytes = reader.footer.indexBytes;
    expect(big.bytes.length).toBeGreaterThan(16 * 1024 * 1024);
    expect(src.bytesTransferred).toBeLessThanOrEqual(indexBytes + e.length + 64);
    expect(src.bytesTransferred).toBe(FOOTER_BYTES + indexBytes + e.length);
    // HEAD + footer + index + entry
    expect(hits.map((h) => h.method + (h.range ? ' ' + h.range : ''))).toHaveLength(4);
  });

  it('server ignoring Range (200): falls back to one whole-file download and still reads correctly', async () => {
    ignoreRange = true;
    const src = new HttpRangeSource(`${base}/a.fpak`);
    const reader = await FpakReader.open(src);
    for (const e of entries.slice(0, 20)) expect(bytesEqual(await reader.read(e.id), e.payload)).toBe(true);
    expect(src.fellBack).toBe(true);
    expect(src.bytesTransferred).toBe(pak.bytes.length);
    ignoreRange = false;
  });
});
