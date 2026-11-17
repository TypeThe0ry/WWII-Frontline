import { randomBytes } from 'node:crypto';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  BufferSource, FileSource, FOOTER_BYTES, FpakFormatError, FpakReader, FpakWriter, HEADER_BYTES, parseIndex, publishPath, Codec, bytesEqual, type KindName,
} from '../src/fpak/index.ts';

function rng(seed: number) {
  let s = seed >>> 0;
  return () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 4294967296);
}

function sample(sizes: number[], seed = 7) {
  const r = rng(seed);
  const kinds: KindName[] = ['mesh', 'texture', 'audio', 'meta'];
  return sizes.map((n, i) => ({
    id: `asset_${i}@lod${i % 4}`,
    kind: kinds[i % 4] as KindName,
    codec: i % 5,
    payload: Uint8Array.from({ length: n }, () => Math.floor(r() * 256)),
  }));
}

describe('fpak writer / reader (acceptance 1)', () => {
  it('round-trips >= 50 entries incl. 0 B, 1 B, 1 MB and 16 MB byte for byte (buffer + file backends)', async () => {
    const sizes = [0, 1, 2, 15, 16, 17, 1_000_000, 16 * 1024 * 1024, ...Array.from({ length: 50 }, (_, i) => (i * 7919) % 5000)];
    const entries = sample(sizes);
    const w = new FpakWriter();
    for (const e of entries) w.add(e.id, e.kind, e.codec, e.payload);
    const built = w.finish();
    expect(built.index).toHaveLength(entries.length);
    const dir = mkdtempSync(join(tmpdir(), 'fl-fpak-'));
    try {
      writeFileSync(join(dir, 'a.fpak'), built.bytes);
      const fs = new FileSource(join(dir, 'a.fpak'));
      for (const src of [new BufferSource(built.bytes), fs]) {
        const r = await FpakReader.open(src);
        expect(r.entries()).toHaveLength(entries.length);
        for (const e of entries) expect(bytesEqual(await r.read(e.id), e.payload), e.id).toBe(true);
        for (const ix of r.entries()) {
          expect(ix.offset % 16n).toBe(0n);
          const src2 = entries.find((e) => e.id === ix.logical_id)!;
          expect(ix.bytes).toBe(src2.payload.length);
          expect(ix.kind).toBe(src2.kind);
          expect(ix.codec).toBe(src2.codec);
        }
      }
      fs.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }, 60_000);

  it('layout: header magic, aligned entries, footer fields and publish path', () => {
    const w = new FpakWriter();
    w.add('a_b@lod0', 'mesh', Codec.meshV1, new Uint8Array(5).fill(1));
    w.add('c_d', 'meta', Codec.json, new Uint8Array(33).fill(2));
    const b = w.finish();
    const dv = new DataView(b.bytes.buffer);
    expect(String.fromCharCode(...b.bytes.subarray(0, 4))).toBe('FPAK');
    expect(String.fromCharCode(...b.bytes.subarray(b.bytes.length - 4))).toBe('KAPF');
    expect(dv.getUint16(4, true)).toBe(1);
    expect(dv.getUint32(8, true)).toBe(2);
    expect(Number(b.index[0]!.offset)).toBe(HEADER_BYTES);
    expect(Number(b.index[1]!.offset)).toBe(HEADER_BYTES + 16);
    const f = b.bytes.length - FOOTER_BYTES;
    expect(Number(dv.getBigUint64(f, true))).toBe(Number(dv.getBigUint64(16, true)));
    expect(dv.getUint32(f + 12, true)).toBe(2);
    expect(publishPath(b.sha256)).toMatch(/^\/a\/[0-9a-f]{2}\/[0-9a-f]{14}\.fpak$/);
  });

  it('rejects bad ids (paths, vendors/upper case, too long), duplicates, unknown kind and codec', () => {
    const w = new FpakWriter();
    for (const bad of ['', 'Foo', 'a/b', 'a\\b', '../x', 'wpn_x.glb', 'a b', 'x'.repeat(256), 'a@', '@a']) {
      expect(() => w.add(bad, 'meta', 0, new Uint8Array(1)), bad).toThrow(FpakFormatError);
    }
    w.add('ok_id', 'meta', 0, new Uint8Array(1));
    expect(() => w.add('ok_id', 'meta', 0, new Uint8Array(1))).toThrow(/duplicate/);
    expect(() => w.add('z', 'bogus' as KindName, 0, new Uint8Array(1))).toThrow(/unknown kind/);
    expect(() => w.add('z', 'meta', 70000, new Uint8Array(1))).toThrow(/codec/);
    expect(() => w.add('x'.repeat(255).replace(/^./, 'a'), 'meta', 0, new Uint8Array(1))).not.toThrow();
  });

  it('index serialization handles long ids and u64 offsets', () => {
    const id = 'a' + 'b'.repeat(254);
    const rec = new Uint8Array(26 + id.length);
    const dv = new DataView(rec.buffer);
    dv.setUint16(0, id.length, true);
    dv.setUint8(2, 1);
    dv.setUint16(4, 2, true);
    dv.setBigUint64(6, 0x1_2345_6789_abcdn, true);
    dv.setUint32(14, 0xffff_ffff, true);
    rec.set(Buffer.from(id), 26);
    const [e] = parseIndex(rec, 1);
    expect(e!.logical_id).toBe(id);
    expect(e!.offset).toBe(0x1_2345_6789_abcdn);
    expect(e!.bytes).toBe(0xffff_ffff);
    expect(() => parseIndex(rec.subarray(0, 30), 1)).toThrow(FpakFormatError);
    rec[2] = 9;
    expect(() => parseIndex(rec, 1)).toThrow(/unknown entry kind/);
  });

  it('performance: 16 MB pack written <= 1 s, 1000 random reads <= 5 s', async () => {
    const entries = sample(Array.from({ length: 64 }, () => 256 * 1024));
    const w = new FpakWriter();
    for (const e of entries) w.add(e.id, e.kind, e.codec, e.payload);
    const t0 = performance.now();
    const b = w.finish();
    const writeMs = performance.now() - t0;
    const r = await FpakReader.open(new BufferSource(b.bytes));
    const rand = rng(3);
    const t1 = performance.now();
    for (let i = 0; i < 1000; i++) await r.read(entries[Math.floor(rand() * entries.length)]!.id);
    const readMs = performance.now() - t1;
    writeFileSync(join(tmpdir(), 'fpak-perf.txt'), `write ${writeMs.toFixed(0)} ms (${(b.bytes.length / 1048576).toFixed(1)} MiB), 1000 reads ${readMs.toFixed(0)} ms\n`);
    expect(writeMs).toBeLessThan(1000);
    expect(readMs).toBeLessThan(5000);
  });

  it('randomBytes sanity: sha8 differs for different payloads', () => {
    const w = new FpakWriter();
    w.add('a', 'meta', 0, randomBytes(64));
    w.add('b', 'meta', 0, randomBytes(64));
    const { index } = w.finish();
    expect(bytesEqual(index[0]!.sha8, index[1]!.sha8)).toBe(false);
  });
});
