import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { PH_NAME_RE, phName } from '../actors/names';
import {
  CUSTOM_VARIANTS, enumerateVariants, MAX_VARIANTS, MAX_VARIANTS_LOW, MaterialFactory, VariantRegistry, variantKey, variantMask, type MaterialDesc,
} from './variants';

const d = (pass: MaterialDesc['pass'], flags: MaterialDesc['flags'] = {}): MaterialDesc => ({ shading: 'pbr', pass, flags });

describe('PBR variants', () => {
  it('total legal variants <= 60 (and exactly 32); Low <= 40 (exactly 20)', () => {
    const all = enumerateVariants(false);
    const low = enumerateVariants(true);
    expect(all.length).toBeLessThanOrEqual(MAX_VARIANTS);
    expect(all.length).toBe(32);
    expect(low.length).toBeLessThanOrEqual(MAX_VARIANTS_LOW);
    expect(low.length).toBe(20);
    expect(new Set(all.map((v) => variantKey(v))).size).toBe(all.length);
  });

  it.each([[false, 37], [true, 25]] as const)('gates the total PBR and custom variants (Low=%s)', (low, count) => {
    const inventory = new VariantRegistry(low);
    for (const d of enumerateVariants(low)) inventory.register(d);
    for (const key of CUSTOM_VARIANTS) {
      inventory.registerCustom(key);
      inventory.registerCustom(key);
    }
    expect(inventory.stats().count).toBeLessThanOrEqual(inventory.limit);
    expect(inventory.stats()).toMatchObject({ count, custom: [...CUSTOM_VARIANTS].sort() });
    console.info(`shader variant-count ${low ? 'Low' : 'High'}: ${inventory.stats().count}/${inventory.limit} per backend`);
  });

  it('macros a pass cannot use do not make new variants', () => {
    expect(variantMask(d('depth', { normalMap: true, detail: true }))).toBe(0);
    expect(variantKey(d('shadow', { normalMap: true }))).toBe(variantKey(d('shadow')));
    expect(variantKey(d('alpha', { alphaTest: true }))).toBe(variantKey(d('alpha')));
    expect(variantKey(d('opaque', { detail: true }), true)).toBe(variantKey(d('opaque'), true));
    expect(variantKey(d('opaque', { detail: true }))).not.toBe(variantKey(d('opaque')));
  });

  it('registry throws past its limit', () => {
    const r = new VariantRegistry(false);
    for (const v of enumerateVariants()) r.register(v);
    expect(r.stats().count).toBe(32);
    expect(r.stats().byPass).toEqual({ opaque: 16, alpha: 8, depth: 4, shadow: 4 });
    // Only 32 variants exist, so shrink the limit to prove the guard.
    const tiny = Object.assign(new VariantRegistry(true), {});
    Object.defineProperty(tiny, 'limit', { get: () => 2 });
    tiny.register(d('opaque'));
    tiny.register(d('alpha'));
    expect(() => tiny.registerCustom('velocity:terrain')).toThrow(/limit/);
    expect(() => tiny.register(d('depth'))).toThrow(/limit/);
  });

  it('factory returns the same instance for equal variants and caches by key', () => {
    let made = 0;
    const f = new MaterialFactory<{ key: string }>((_, key) => ({ key, n: ++made }) as { key: string });
    const a = f.getMaterial(d('opaque', { normalMap: true }));
    const b = f.getMaterial({ ...d('opaque', { normalMap: true }), placeholderId: 'V-CHR-001' });
    expect(a).toBe(b);
    expect(f.getMaterial(d('shadow', { detail: true }))).toBe(f.getMaterial(d('shadow')));
    expect(made).toBe(2);
  });

  it('custom CSM and TAA velocity passes register every CUSTOM_VARIANTS key', () => {
    const extract = (relPath: string): string[] => {
      const src = readFileSync(fileURLToPath(new URL(relPath, import.meta.url)), 'utf8');
      return [...src.matchAll(/registerCustom\(['"]([^'"]+)['"]\)/g)].map((m) => m[1]!);
    };
    const csmKeys = extract('../lighting/custom-csm.ts').sort();
    const velKeys = extract('../post/velocity.ts').sort();
    expect(csmKeys).toEqual(['csm:instanced', 'csm:plain']);
    expect(velKeys).toEqual(['velocity:instanced', 'velocity:plain', 'velocity:terrain']);
    expect([...new Set([...csmKeys, ...velKeys])].sort()).toEqual([...CUSTOM_VARIANTS].sort());
  });

  it('placeholder names follow PH_{requirement}', () => {
    expect(PH_NAME_RE.test(phName('V-NAT-007'))).toBe(true);
    expect(PH_NAME_RE.test('standard')).toBe(false);
  });
});
