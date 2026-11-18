/**
 * PBR variant bookkeeping (M1-042, ARCHITECTURE 10.4): one shader, four macros, at most 60 variants
 * (40 on Low). A `MaterialDesc` is first canonicalised -- macros a pass cannot use are dropped -- so
 * equal visuals share one variant, and `MaterialFactory` hands out one instance per variant.
 */
export type Pass = 'opaque' | 'alpha' | 'depth' | 'shadow';
export interface MaterialFlags {
  normalMap?: boolean;
  alphaTest?: boolean;
  vertexWind?: boolean;
  detail?: boolean;
}
export interface MaterialDesc {
  shading: 'pbr';
  flags: MaterialFlags;
  pass: Pass;
  /** `PH_{requirement id}` placeholder name, if any (does not affect the variant). */
  placeholderId?: string;
}

export const MAX_VARIANTS = 60;
export const MAX_VARIANTS_LOW = 40;
export const PASSES: readonly Pass[] = ['opaque', 'alpha', 'depth', 'shadow'];

/** Macros a pass can use: depth/shadow are unlit (no normal map, no detail); alpha blends (no alpha test). */
const USABLE: Record<Pass, readonly (keyof MaterialFlags)[]> = {
  opaque: ['normalMap', 'alphaTest', 'vertexWind', 'detail'],
  alpha: ['normalMap', 'vertexWind', 'detail'],
  depth: ['alphaTest', 'vertexWind'],
  shadow: ['alphaTest', 'vertexWind'],
};
const BIT: Record<keyof MaterialFlags, number> = { normalMap: 1, alphaTest: 2, vertexWind: 4, detail: 8 };

/** Canonical macro bitmask of a description. `low` additionally drops `detail`. */
export function variantMask(d: MaterialDesc, low = false): number {
  let m = 0;
  for (const f of USABLE[d.pass]) {
    if (low && f === 'detail') continue;
    if (d.flags[f]) m |= BIT[f];
  }
  return m;
}

export const variantKey = (d: MaterialDesc, low = false): string => `${d.pass}:${variantMask(d, low)}`;

/** Every distinct legal variant of the macro set. */
export function enumerateVariants(low = false): MaterialDesc[] {
  const out = new Map<string, MaterialDesc>();
  for (const pass of PASSES) {
    for (let m = 0; m < 16; m++) {
      const flags: MaterialFlags = {
        normalMap: (m & 1) !== 0,
        alphaTest: (m & 2) !== 0,
        vertexWind: (m & 4) !== 0,
        detail: (m & 8) !== 0,
      };
      const d: MaterialDesc = { shading: 'pbr', flags, pass };
      const k = variantKey(d, low);
      if (!out.has(k)) out.set(k, { shading: 'pbr', pass, flags: maskToFlags(variantMask(d, low)) });
    }
  }
  return [...out.values()];
}

function maskToFlags(m: number): MaterialFlags {
  return {
    normalMap: (m & 1) !== 0,
    alphaTest: (m & 2) !== 0,
    vertexWind: (m & 4) !== 0,
    detail: (m & 8) !== 0,
  };
}

export interface VariantStats {
  count: number;
  byPass: Record<Pass, number>;
}

/** Counts the variants currently in use; throws (dev-time error) above the limit. */
export class VariantRegistry {
  private readonly keys = new Set<string>();
  constructor(readonly low = false) {}

  get limit(): number {
    return this.low ? MAX_VARIANTS_LOW : MAX_VARIANTS;
  }

  register(d: MaterialDesc): string {
    const k = variantKey(d, this.low);
    if (!this.keys.has(k)) {
      if (this.keys.size >= this.limit) throw new Error(`PBR variant limit ${this.limit} exceeded by ${k}`);
      this.keys.add(k);
    }
    return k;
  }

  stats(): VariantStats {
    const byPass: Record<Pass, number> = { opaque: 0, alpha: 0, depth: 0, shadow: 0 };
    for (const k of this.keys) byPass[k.split(':')[0] as Pass]++;
    return { count: this.keys.size, byPass };
  }
}

/** One material instance per variant; `create` builds the engine material for a canonical description. */
export class MaterialFactory<M> {
  readonly registry: VariantRegistry;
  private readonly cache = new Map<string, M>();
  constructor(
    private readonly create: (d: MaterialDesc, key: string) => M,
    low = false,
  ) {
    this.registry = new VariantRegistry(low);
  }

  getMaterial(d: MaterialDesc): M {
    const key = this.registry.register(d);
    let m = this.cache.get(key);
    if (m === undefined) {
      m = this.create({ shading: 'pbr', pass: d.pass, flags: maskToFlags(variantMask(d, this.registry.low)) }, key);
      this.cache.set(key, m);
    }
    return m;
  }
}
