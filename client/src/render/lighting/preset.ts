/** Lighting preset from map data (never hard-coded in render code) and the A/B blend (M1-043). */
export interface LightingPreset {
  id: string;
  sunDir: [number, number, number];
  sunColor: [number, number, number];
  sunIntensity: number;
  iblA: string;
  iblB?: string;
  /** 0 = all A, 1 = all B. */
  blend: number;
  cloudSpeed: number;
}

const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

/** Interpolates two presets at `t` in [0,1]; the sun direction is renormalised. */
export function mixPresets(a: LightingPreset, b: LightingPreset, t: number): LightingPreset {
  const k = Math.min(Math.max(t, 0), 1);
  const d = [0, 1, 2].map((i) => lerp(a.sunDir[i] as number, b.sunDir[i] as number, k));
  const len = Math.hypot(d[0] as number, d[1] as number, d[2] as number) || 1;
  return {
    id: k < 0.5 ? a.id : b.id,
    sunDir: [(d[0] as number) / len, (d[1] as number) / len, (d[2] as number) / len],
    sunColor: [0, 1, 2].map((i) => lerp(a.sunColor[i] as number, b.sunColor[i] as number, k)) as [number, number, number],
    sunIntensity: lerp(a.sunIntensity, b.sunIntensity, k),
    iblA: a.iblA,
    iblB: b.iblA,
    blend: k,
    cloudSpeed: lerp(a.cloudSpeed, b.cloudSpeed, k),
  };
}

/** Parses a preset from untrusted map JSON; throws a descriptive error on a bad field. */
export function parsePreset(j: unknown): LightingPreset {
  const o = j as Record<string, unknown>;
  const vec = (k: string): [number, number, number] => {
    const v = o[k];
    if (!Array.isArray(v) || v.length !== 3 || v.some((x) => typeof x !== 'number' || !Number.isFinite(x))) throw new Error(`lighting preset: bad ${k}`);
    return v as [number, number, number];
  };
  const num = (k: string): number => {
    const v = o[k];
    if (typeof v !== 'number' || !Number.isFinite(v)) throw new Error(`lighting preset: bad ${k}`);
    return v;
  };
  const str = (k: string): string => {
    const v = o[k];
    if (typeof v !== 'string' || v === '') throw new Error(`lighting preset: bad ${k}`);
    return v;
  };
  return {
    id: str('id'),
    sunDir: vec('sunDir'),
    sunColor: vec('sunColor'),
    sunIntensity: num('sunIntensity'),
    iblA: str('iblA'),
    iblB: typeof o.iblB === 'string' ? o.iblB : undefined,
    blend: typeof o.blend === 'number' ? o.blend : 0,
    cloudSpeed: num('cloudSpeed'),
  };
}
