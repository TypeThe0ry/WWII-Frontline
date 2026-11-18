import { describe, expect, it } from 'vitest';
import { pbrGlsl, pbrWgsl } from './pbr-shader';
import { enumerateVariants } from './variants';

describe('PBR shader sources (M1-042b)', () => {
  const all = enumerateVariants(false);
  it('covers the 32 legal variants (16 opaque... as counted by M1-042a)', () => {
    expect(all.length).toBe(32);
  });
  it('both languages declare every uniform / attribute / sampler of the shared interface', () => {
    for (const d of all) {
      const g = pbrGlsl(d);
      const w = pbrWgsl(d);
      expect(w.attributes, JSON.stringify(d)).toEqual(g.attributes);
      expect(w.uniforms).toEqual(g.uniforms);
      expect(w.samplers).toEqual(g.samplers);
      for (const u of g.uniforms.filter((n) => n !== 'world' && n !== 'viewProjection')) {
        const used = g.vertex.includes(u) || g.fragment.includes(u);
        expect(used, `GLSL ${u} ${JSON.stringify(d)}`).toBe(true);
        expect(w.vertex.includes(u) || w.fragment.includes(u), `WGSL ${u}`).toBe(true);
      }
      for (const s of g.samplers) {
        expect(g.fragment.includes(`sampler2D ${s}`), `GLSL sampler ${s}`).toBe(true);
        expect(w.fragment.includes(`var ${s}: texture_2d<f32>`), `WGSL texture ${s}`).toBe(true);
        expect(w.fragment.includes(`${s}Sampler`), `WGSL sampler ${s}`).toBe(true);
      }
    }
  });
  it('macros expand to source text only when the flag is on', () => {
    const base = { shading: 'pbr' as const, pass: 'opaque' as const, flags: {} };
    expect(pbrGlsl(base).fragment).not.toMatch(/dFdx|discard|detailTex/);
    expect(pbrGlsl({ ...base, flags: { normalMap: true } }).fragment).toMatch(/dFdx/);
    expect(pbrGlsl({ ...base, flags: { alphaTest: true } }).fragment).toMatch(/discard/);
    expect(pbrGlsl({ ...base, flags: { detail: true } }).fragment).toMatch(/detailTex/);
    expect(pbrGlsl({ ...base, flags: { vertexWind: true } }).vertex).toMatch(/uWind\.w/);
    expect(pbrGlsl(base).vertex).not.toMatch(/uWind\.w/);
    expect(pbrWgsl({ ...base, flags: { alphaTest: true } }).fragment).toMatch(/discard/);
  });
  it('depth / shadow passes have no lighting inputs', () => {
    for (const pass of ['depth', 'shadow'] as const) {
      const s = pbrGlsl({ shading: 'pbr', pass, flags: {} });
      expect(s.uniforms).not.toContain('uLightDir');
      expect(s.attributes).toEqual(['position']);
    }
  });
});
