/**
 * Vegetation species table (ARCHITECTURE §9.6). The instance table stores a species id (u8); everything else about a
 * species (size range, caps, placeholder asset ids) lives here, so M1's real assets only replace the mesh builders.
 */

/** Chunk edge (m): one instance table per 64 m chunk (§9.6). */
export const CHUNK_SIZE_M = 64;
/** Bytes per instance-table entry: x:u16 z:u16 species:u8 scale:u8 rot:u8. */
export const INSTANCE_BYTES = 6;

export const SPECIES_GRASS = 0;
export const SPECIES_SHRUB = 1;
export const SPECIES_TREE = 2;
export type SpeciesId = 0 | 1 | 2;

export interface SpeciesInfo {
  id: SpeciesId;
  key: 'grass' | 'shrub' | 'tree';
  /** Asset-gap requirement id (ASSET_GAPS §3); placeholder materials are named `PH_<reqId>`. */
  reqId: 'V-NAT-004' | 'V-NAT-002';
  /** Per-chunk instance cap (§9.6). */
  cap: number;
  /** Uniform instance scale range, mapped from the u8 `scale` field. */
  scaleMin: number;
  scaleMax: number;
  /** Poisson-disk minimum spacing (m); 0 = clustered noise scatter (grass). */
  minSpacing: number;
  /** Species height (m) of the unscaled placeholder (for bounding boxes). */
  height: number;
}

export const SPECIES: readonly SpeciesInfo[] = [
  { id: 0, key: 'grass', reqId: 'V-NAT-004', cap: 4096, scaleMin: 0.7, scaleMax: 1.4, minSpacing: 0, height: 0.6 },
  { id: 1, key: 'shrub', reqId: 'V-NAT-004', cap: 256, scaleMin: 0.7, scaleMax: 1.3, minSpacing: 1.5, height: 1.4 },
  { id: 2, key: 'tree', reqId: 'V-NAT-002', cap: 192, scaleMin: 0.75, scaleMax: 1.25, minSpacing: 4, height: 9.5 },
];

export function speciesScale(species: SpeciesInfo, scaleByte: number): number {
  return species.scaleMin + (scaleByte / 255) * (species.scaleMax - species.scaleMin);
}

/** rot:u8 -> yaw radians. */
export function rotYaw(rotByte: number): number {
  return (rotByte / 256) * Math.PI * 2;
}
