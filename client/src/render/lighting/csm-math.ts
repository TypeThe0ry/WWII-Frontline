/** Cascaded shadow map parameters and the static-cascade update scheduler (M1-043, ARCHITECTURE 10.1). */
export type ShadowTier = 'low' | 'medium' | 'high' | 'ultra';

export interface CsmConfig {
  /** Far edge of each cascade, metres. */
  splits: readonly number[];
  /** Shadow map size of each cascade. */
  sizes: readonly number[];
  /** PCF kernel width in taps (3 = 3x3). */
  pcf: 3 | 5;
}

/** WebGPU: 4 cascades. WebGL2 uses 3 (splits marked [tbd] in the task, kept in data once tuned). */
export function csmConfig(tier: ShadowTier, webgl2: boolean): CsmConfig {
  if (webgl2) return { splits: [15, 60, 250], sizes: [2048, 1024, 1024], pcf: 3 };
  switch (tier) {
    case 'low': return { splits: [20, 80, 250], sizes: [1024, 1024, 1024], pcf: 3 };
    case 'medium': return { splits: [12, 40, 120, 350], sizes: [1024, 1024, 1024, 1024], pcf: 3 };
    default: return { splits: [12, 40, 120, 350], sizes: [2048, 2048, 2048, 1024], pcf: 5 };
  }
}

/** Index of the cascade covering view-space depth `z` (metres), or -1 beyond the shadow distance. */
export function cascadeAt(cfg: CsmConfig, z: number): number {
  for (let i = 0; i < cfg.splits.length; i++) if (z <= (cfg.splits[i] as number)) return i;
  return -1;
}

/** Characters cast shadows only in cascades 0-2 (task 2). */
export const characterCastsIn = (cascade: number): boolean => cascade >= 0 && cascade <= 2;

/**
 * Decides which cascades to redraw this frame. Cascades 0/1 hold mostly static geometry: when the
 * camera moved less than `maxMoveM` since last frame they refresh every 2nd frame; all others, and
 * any frame with a larger camera move, refresh every frame.
 */
export class CascadeScheduler {
  static readonly STATIC_CASCADES = 2;
  static readonly MAX_MOVE_M = 0.5;
  /** Redraw counters per cascade (acceptance 3). */
  readonly redraws: number[];
  private frame = 0;

  constructor(readonly cascades: number) {
    this.redraws = new Array<number>(cascades).fill(0);
  }

  /** Fills `out[i]` with whether cascade i is redrawn this frame. */
  next(cameraMoveM: number, out: boolean[]): void {
    const still = cameraMoveM < CascadeScheduler.MAX_MOVE_M;
    for (let i = 0; i < this.cascades; i++) {
      const stat = i < CascadeScheduler.STATIC_CASCADES && still;
      const draw = !stat || this.frame % 2 === 0;
      out[i] = draw;
      if (draw) (this.redraws[i] as number)++;
    }
    this.frame++;
  }
}
