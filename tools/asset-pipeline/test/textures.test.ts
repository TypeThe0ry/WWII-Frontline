import { describe, expect, it } from 'vitest';
import { Document } from '@gltf-transform/core';
import { assertAllowedFor } from './helpers.ts';
import {
  decodePng, encodeKtx2, inspectKtx2, packOrm, padToMultipleOf4, processTextures, profileFor, selectFormat, toRrrg, type Raster,
} from '../src/textures.ts';
import { ffmpeg } from '../src/audio/ffmpeg.ts';
import { ExitCode, GateError } from '../src/errors.ts';
import { albedoPng, buildDoc, grayPng, normalPng, sphereData } from './synth.ts';

const ras = (w: number, h: number, f: (i: number) => [number, number, number, number]): Raster => {
  const data = new Uint8Array(w * h * 4);
  for (let i = 0; i < w * h; i++) data.set(f(i), i * 4);
  return { width: w, height: h, data };
};

describe('format selection policy', () => {
  it('profile x slot table', () => {
    const table: [Parameters<typeof selectFormat>[0], Parameters<typeof selectFormat>[1], string][] = [
      ['bg', 'color', 'ETC1S'], ['hero', 'color', 'UASTC'], ['ui', 'color', 'UASTC'], ['normal', 'color', 'UASTC'],
      ['bg', 'normal', 'UASTC'], ['hero', 'normal', 'UASTC'], ['bg', 'orm', 'ETC1S'], ['hero', 'orm', 'UASTC'],
    ];
    for (const [p, s, f] of table) expect(selectFormat(p, s), `${p}/${s}`).toBe(f);
  });
  it('profileFor: override > asset profile > bg; normals are always normal', () => {
    expect(profileFor({}, 'a', 'color')).toBe('bg');
    expect(profileFor({ texture_profile: 'hero' }, 'a', 'color')).toBe('hero');
    expect(profileFor({ texture_profile: 'hero', textures: { a: 'ui' } }, 'a', 'color')).toBe('ui');
    expect(profileFor({ texture_profile: 'bg', textures: { n: 'bg' } }, 'n', 'normal')).toBe('normal');
  });
});

describe('ORM packing (channel mapping)', () => {
  it('R = AO, G = roughness, B = metallic', () => {
    const ao = ras(4, 4, () => [10, 0, 0, 255]);
    const ro = ras(4, 4, () => [20, 0, 0, 255]);
    const me = ras(4, 4, () => [30, 0, 0, 255]);
    const o = packOrm({ ao, roughness: ro, metallic: me });
    expect([...o.data.slice(0, 4)]).toEqual([10, 20, 30, 255]);
  });
  it('missing AO -> 255; mismatching sizes are refused', () => {
    const ro = ras(4, 4, () => [20, 0, 0, 255]);
    expect(packOrm({ roughness: ro, metallic: ro }).data[0]).toBe(255);
    expect(() => packOrm({ roughness: ro, metallic: ras(8, 8, () => [0, 0, 0, 255]) })).toThrow(GateError);
  });
});

describe('padding and normal layout', () => {
  it('NPOT (non multiple of 4) is padded with edge replication', () => {
    const r = ras(6, 5, (i) => [i % 256, 0, 0, 255]);
    const p = padToMultipleOf4(r);
    expect(p.padded).toBe(true);
    expect([p.raster.width, p.raster.height]).toEqual([8, 8]);
    expect(p.raster.data[(0 * 8 + 7) * 4]).toBe(r.data[(0 * 6 + 5) * 4]);
    expect(padToMultipleOf4(ras(8, 4, () => [0, 0, 0, 255])).padded).toBe(false);
  });
  it('toRrrg: X in RGB, Y in A', () => {
    const o = toRrrg(ras(1, 1, () => [100, 200, 255, 255]));
    expect([...o.data]).toEqual([100, 100, 100, 200]);
  });
});

describe('KTX2 encoding (acceptance 4)', () => {
  it('normal map -> UASTC, 2 channels, mip chain; bg colour -> ETC1S', async () => {
    const n = await encodeKtx2(toRrrg(decodePng(new Uint8Array(normalPng(64)))), { format: selectFormat('bg', 'normal'), srgb: false, normal: true, channels: 2 });
    const ni = inspectKtx2(n);
    expect(ni.format).toBe('UASTC');
    expect(ni.channels).toBe(2);
    expect(ni.layout).toBe('RRRG');
    expect(ni.levels).toBe(7);
    const c = await encodeKtx2(decodePng(new Uint8Array(albedoPng(64))), { format: selectFormat('bg', 'color'), srgb: true });
    const ci = inspectKtx2(c);
    expect(ci.format).toBe('ETC1S');
    expect(ci.levels).toBe(7);
    expect([ci.width, ci.height]).toEqual([64, 64]);
  });
});

describe('processTextures on a document', () => {
  it('packs AO + MR into one ORM texture, encodes everything to KTX2 and pads NPOT', async () => {
    const doc = buildDoc(sphereData(8, 8), { baseColor: albedoPng(60), normal: normalPng(60), occlusion: grayPng(60, (u) => u), metalRough: grayPng(60, (_, v) => v) });
    const out = await processTextures(doc, { cls: { texture_profile: 'bg' } });
    expect(out.textures.map((t) => t.slot).sort()).toEqual(['color', 'normal', 'orm']);
    const byslot = Object.fromEntries(out.textures.map((t) => [t.slot, t]));
    expect(byslot.color!.format).toBe('ETC1S');
    expect(byslot.normal!.format).toBe('UASTC');
    expect(byslot.normal!.channels).toBe(2);
    expect(byslot.color!.padded).toBe(false);
    expect([byslot.color!.width, byslot.color!.height]).toEqual([60, 60]);
    expect(out.warnings.some((w) => /multiple of 4/.test(w))).toBe(false);
    const mat = doc.getRoot().listMaterials()[0]!;
    expect(mat.getOcclusionTexture()).toBe(mat.getMetallicRoughnessTexture());
    expect(doc.getRoot().listTextures().every((t) => t.getMimeType() === 'image/ktx2')).toBe(true);
    expect(doc.getRoot().listExtensionsRequired().map((e) => e.extensionName)).toContain('KHR_texture_basisu');
  }, 60_000);

  it('NPOT texture warns and is padded', async () => {
    const doc = buildDoc(sphereData(8, 8), { baseColor: albedoPng(30) });
    const out = await processTextures(doc, { cls: {} });
    expect(out.warnings.join()).toMatch(/multiple of 4/);
    expect(out.textures[0]!.width).toBe(32);
  });

  it('a document without textures is a no-op and needs no TEXCOMP', async () => {
    const out = await processTextures(new Document(), { cls: {}, assetId: 'AST-NONE', ledger: [] });
    expect(out.textures).toEqual([]);
  });

  it('TEXCOMP outside allowed_processing is refused', async () => {
    const doc = buildDoc(sphereData(8, 8), { baseColor: albedoPng(16) });
    const ledger = assertAllowedFor('LOD');
    await expect(processTextures(doc, { cls: {}, assetId: 'AST-V-TEST-001-01', ledger })).rejects.toMatchObject({ code: ExitCode.ProcessingNotAllowed });
  });

  it('JPEG source textures (as in Poly Haven CC0 glTF) decode via ffmpeg and encode to KTX2', async () => {
    const toJpeg = async (png: Buffer) => {
      const { stdout } = await ffmpeg(['-f', 'image2pipe', '-vcodec', 'png', '-i', 'pipe:0', '-f', 'image2pipe', '-vcodec', 'mjpeg', '-q:v', '3', 'pipe:1'], { input: png });
      return new Uint8Array(stdout.buffer, stdout.byteOffset, stdout.byteLength);
    };
    const doc = buildDoc(sphereData(8, 8), { baseColor: albedoPng(32), normal: normalPng(32) });
    for (const tex of doc.getRoot().listTextures()) {
      const jpg = await toJpeg(Buffer.from(tex.getImage()!));
      tex.setImage(jpg).setMimeType('image/jpeg');
    }
    const out = await processTextures(doc, { cls: { texture_profile: 'bg' } });
    const byslot = Object.fromEntries(out.textures.map((t) => [t.slot, t]));
    expect(byslot.color!.format).toBe('ETC1S');
    expect(byslot.normal!.format).toBe('UASTC');
    expect(byslot.normal!.channels).toBe(2);
    expect([byslot.color!.width, byslot.color!.height]).toEqual([32, 32]);
    expect(doc.getRoot().listTextures().every((t) => t.getMimeType() === 'image/ktx2')).toBe(true);
  }, 60_000);
});
