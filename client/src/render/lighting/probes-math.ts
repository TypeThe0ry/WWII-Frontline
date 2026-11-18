/** Light-probe grid maths (M1-043): trilinear weights, storage size. L2 spherical harmonics, f16. */
export const SH_L2_COEFFS = 9;
/** 9 coefficients x RGB x 2 bytes (f16). */
export const PROBE_BYTES = SH_L2_COEFFS * 3 * 2;
export const PROBE_BUDGET_BYTES = 1024 * 1024;

export interface ProbeGrid {
  /** World origin of probe (0,0,0). */
  origin: readonly [number, number, number];
  /** Probe spacing, metres. */
  spacing: number;
  /** Probe counts per axis. */
  dims: readonly [number, number, number];
}

export const probeCount = (g: ProbeGrid): number => g.dims[0] * g.dims[1] * g.dims[2];

/** Loader assertion: the grid must fit the 1 MB probe budget. */
export function checkProbeBudget(g: ProbeGrid): void {
  const bytes = probeCount(g) * PROBE_BYTES;
  if (bytes > PROBE_BUDGET_BYTES) throw new Error(`probe data ${bytes} B exceeds ${PROBE_BUDGET_BYTES} B`);
}

/**
 * The 8 surrounding probes of `p` and their trilinear weights (sum 1). Positions outside the grid
 * clamp to the border. `idx[k]` is the linear index (x fastest), `w[k]` its weight.
 */
export function trilinear(g: ProbeGrid, p: readonly [number, number, number], idx: Int32Array, w: Float32Array): void {
  const f = [0, 0, 0];
  const i0 = [0, 0, 0];
  for (let a = 0; a < 3; a++) {
    const max = (g.dims[a] as number) - 1;
    const t = Math.min(Math.max(((p[a] as number) - (g.origin[a] as number)) / g.spacing, 0), max);
    const fl = Math.min(Math.floor(t), Math.max(max - 1, 0));
    i0[a] = fl;
    f[a] = max === 0 ? 0 : t - fl;
  }
  let k = 0;
  for (let dz = 0; dz < 2; dz++) {
    for (let dy = 0; dy < 2; dy++) {
      for (let dx = 0; dx < 2; dx++) {
        const x = Math.min((i0[0] as number) + dx, g.dims[0] - 1);
        const y = Math.min((i0[1] as number) + dy, g.dims[1] - 1);
        const z = Math.min((i0[2] as number) + dz, g.dims[2] - 1);
        idx[k] = x + g.dims[0] * (y + g.dims[1] * z);
        w[k] =
          (dx ? (f[0] as number) : 1 - (f[0] as number)) *
          (dy ? (f[1] as number) : 1 - (f[1] as number)) *
          (dz ? (f[2] as number) : 1 - (f[2] as number));
        k++;
      }
    }
  }
}
