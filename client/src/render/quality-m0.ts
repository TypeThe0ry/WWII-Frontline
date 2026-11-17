/**
 * M0 render constants per quality tier (ARCHITECTURE §13.1 "植被密度/距离" row, §9.6 LOD distances).
 * Render constants only (not gameplay values), so they live in the client. `high` = the §13.1 High column
 * (grass 100 % / 45 m), `low` = the Low column (grass 35 % / 20 m, trees turn into impostors early) used for Iris Xe tests.
 * Distances in metres from the camera. The Medium/Ultra columns arrive with the settings UI (M1+).
 */
export type QualityTier = 'high' | 'low';

export interface SpeciesLodDistances {
  /** Mesh LOD0 up to this distance. */
  lod0: number;
  /** Mesh LOD1 up to this distance; beyond it the octahedral impostor. */
  lod1: number;
  /** Impostors are drawn up to this distance, beyond it the instance is dropped. */
  far: number;
}

export interface QualityM0 {
  name: QualityTier;
  /** Fraction of the grass instance table that is drawn (stable per-instance subset). */
  grassDensity: number;
  /** Grass draw distance: LOD0 to 0.55 x, LOD1 to 1.0 x (shrinking out over the last 25 %). */
  grassDistance: number;
  shrub: SpeciesLodDistances;
  tree: SpeciesLodDistances;
}

export const QUALITY_M0: Readonly<Record<QualityTier, QualityM0>> = {
  high: {
    name: 'high',
    grassDensity: 1,
    grassDistance: 45,
    shrub: { lod0: 25, lod1: 80, far: 700 },
    tree: { lod0: 30, lod1: 100, far: 1500 },
  },
  low: {
    name: 'low',
    grassDensity: 0.35,
    grassDistance: 20,
    shrub: { lod0: 10, lod1: 30, far: 400 },
    tree: { lod0: 12, lod1: 35, far: 800 },
  },
};

export function parseQuality(v: string | null): QualityTier {
  return v === 'low' ? 'low' : 'high';
}
