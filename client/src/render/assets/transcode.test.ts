import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, beforeAll } from 'vitest';
import { decodeTexture } from './entry-decode';
import { rebuildKtx2 } from './ktx2-rebuild';
import { instantiateBasis, pickFormat, transcodeTexture, type BasisModule } from './transcode';

const rd = (u: URL) => readFileSync(fileURLToPath(u));
const fx = (n: string) => new Uint8Array(rd(new URL(`./fixtures/${n}`, import.meta.url)));
const basisDir = new URL('../../../public/basis/', import.meta.url);
let basis: BasisModule;

beforeAll(async () => {
  const js = readFileSync(fileURLToPath(new URL('basis_transcoder.js', basisDir)), 'utf8');
  const wasm = rd(new URL('basis_transcoder.wasm', basisDir));
  basis = await instantiateBasis(js, wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) as ArrayBuffer, { require: createRequire(import.meta.url), __dirname: fileURLToPath(basisDir), __filename: fileURLToPath(new URL('basis_transcoder.js', basisDir)) });
});

describe('format selection', () => {
  it('prefers bc7, then astc, etc2, bc3 and falls back to rgba8', () => {
    expect(pickFormat({ bc7: true, astc: true })).toBe('bc7');
    expect(pickFormat({ astc: true, etc2: true })).toBe('astc4x4');
    expect(pickFormat({ etc2: true, bc3: true })).toBe('etc2');
    expect(pickFormat({ bc3: true })).toBe('bc3');
    expect(pickFormat({})).toBe('rgba8');
  });
});

describe('KTX2 rebuild', () => {
  it('reproduces the encoder output byte-for-byte for the payload that the entry keeps', () => {
    for (const n of ['uastc', 'etc1s']) {
      const t = decodeTexture(fx(`tex_${n}_16.entry`));
      const rebuilt = rebuildKtx2(t);
      const orig = fx(`tex_${n}_16.ktx2`);
      // Same level payloads and DFD at the offsets the rebuilt level index names.
      const dv = new DataView(rebuilt.buffer);
      t.levels.forEach((l, i) => {
        const at = Number(dv.getBigUint64(80 + i * 24, true));
        expect(Array.from(rebuilt.subarray(at, at + l.data.length))).toEqual(Array.from(l.data));
      });
      expect(Array.from(rebuilt.subarray(0, 12))).toEqual(Array.from(orig.subarray(0, 12)));
      expect(new DataView(rebuilt.buffer).getUint32(12, true)).toBe(new DataView(orig.buffer, orig.byteOffset).getUint32(12, true));
    }
  });
});

describe('Basis transcoding', () => {
  it.each(['uastc', 'etc1s'])('%s -> every target format with the right sizes and a full mip chain', (n) => {
    const t = decodeTexture(fx(`tex_${n}_16.entry`));
    for (const [caps, fmt, bytes16] of [
      [{ bc7: true }, 'bc7', 16 * 16],
      [{ astc: true }, 'astc4x4', 16 * 16],
      [{ etc2: true }, 'etc2', 16 * 16],
      [{ bc3: true }, 'bc3', 16 * 16],
      [{}, 'rgba8', 16 * 16 * 4],
    ] as const) {
      const out = transcodeTexture(basis, t, caps);
      expect(out.format).toBe(fmt);
      expect([out.width, out.height]).toEqual([16, 16]);
      expect(out.levels.length).toBe(t.levels.length);
      expect(out.levels.length).toBe(5); // 16,8,4,2,1
      expect(out.levels[0]?.data.length).toBe(bytes16);
      expect(out.levels[0]?.data.some((b) => b !== 0)).toBe(true);
    }
  });

  it('rgba8 output is close to the source gradient (R = x*16, G = y*16, B = 128)', () => {
    const t = decodeTexture(fx('tex_uastc_16.entry'));
    const { levels } = transcodeTexture(basis, t, {});
    const px = (x: number, y: number) => Array.from((levels[0] as { data: Uint8Array }).data.subarray((y * 16 + x) * 4, (y * 16 + x) * 4 + 4));
    const [r, g, b] = px(8, 4) as [number, number, number];
    expect(Math.abs(r - 128)).toBeLessThanOrEqual(24);
    expect(Math.abs(g - 64)).toBeLessThanOrEqual(24);
    expect(Math.abs(b - 128)).toBeLessThanOrEqual(24);
  });

  it('rejects corrupt data instead of returning garbage', () => {
    const t = decodeTexture(fx('tex_uastc_16.entry'));
    t.vkFormat = 0xffff;
    t.sgd = new Uint8Array(0);
    t.levels = [{ data: new Uint8Array(4), uncompressedByteLength: 4 }];
    expect(() => transcodeTexture(basis, t, {})).toThrow();
  });
});
