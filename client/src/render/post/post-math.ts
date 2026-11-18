/** CPU reference maths of the post chain (M1-044, ARCHITECTURE 10.2); shaders must match these. */

/** Radical inverse of `i` in `base` (Halton sequence element, i >= 1). */
export function halton(i: number, base: number): number {
  let f = 1;
  let r = 0;
  let n = i;
  while (n > 0) {
    f /= base;
    r += f * (n % base);
    n = Math.floor(n / base);
  }
  return r;
}

export const JITTER_PHASES = 8;

/** Sub-pixel jitter of frame `frameIndex` in [-0.5, 0.5)^2 (Halton(2,3), 8 phases). */
export function getFrameJitter(frameIndex: number): [number, number] {
  const i = (((frameIndex % JITTER_PHASES) + JITTER_PHASES) % JITTER_PHASES) + 1;
  return [halton(i, 2) - 0.5, halton(i, 3) - 0.5];
}

/** Jitter as a clip-space projection offset for a `w x h` target. */
export const jitterToClip = (j: readonly [number, number], w: number, h: number): [number, number] => [(2 * j[0]) / w, (2 * j[1]) / h];

export const NUM_BINS = 64;

/** Log2-luminance histogram bin of `lum` over [minEv, maxEv]; bin 0 also holds black. */
export function lumBin(lum: number, minEv = -8, maxEv = 8): number {
  if (lum <= 0) return 0;
  const t = (Math.log2(lum) - minEv) / (maxEv - minEv);
  return Math.min(NUM_BINS - 1, Math.max(0, Math.floor(t * NUM_BINS)));
}

/** Average EV of the [lo, hi] quantile band of a histogram (count-weighted, bin centres). */
export function histogramAverageEv(hist: ArrayLike<number>, lo = 0.1, hi = 0.9, minEv = -8, maxEv = 8): number {
  let total = 0;
  for (let i = 0; i < hist.length; i++) total += hist[i] as number;
  if (total === 0) return 0;
  const a = total * lo;
  const b = total * hi;
  let seen = 0;
  let sum = 0;
  let wsum = 0;
  for (let i = 0; i < hist.length; i++) {
    const c = hist[i] as number;
    const from = Math.max(seen, a);
    const to = Math.min(seen + c, b);
    if (to > from) {
      const w = to - from;
      sum += w * (minEv + ((i + 0.5) / hist.length) * (maxEv - minEv));
      wsum += w;
    }
    seen += c;
  }
  return wsum === 0 ? 0 : sum / wsum;
}

/** Adapts the exposure value toward the measured one at a bounded rate (EV/s); never overshoots. */
export class ExposureController {
  constructor(
    public ev = 0,
    readonly rateEvPerS = 1.5,
  ) {}

  update(dtS: number, targetEv: number): number {
    const d = targetEv - this.ev;
    const step = this.rateEvPerS * dtS;
    this.ev += Math.abs(d) <= step ? d : Math.sign(d) * step;
    return this.ev;
  }
}

export interface Vec2 { x: number; y: number }

/** Previous-frame UV of a pixel given its motion vector (UV units, current - previous). */
export const reprojectUv = (uv: Vec2, motion: Vec2): Vec2 => ({ x: uv.x - motion.x, y: uv.y - motion.y });

/** History weight: falls with motion length (px) and with `motion_mask` (wind-moved foliage). */
export function historyWeight(motionPx: number, mask = 0, base = 0.9): number {
  const k = Math.max(0, 1 - motionPx / 8) * (1 - 0.5 * Math.min(Math.max(mask, 0), 1));
  return base * k;
}

/** Narkowicz ACES filmic approximation (input linear, output 0..1). */
export function acesApprox(x: number): number {
  const v = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
  return Math.min(Math.max(v, 0), 1);
}

/** YCoCg conversion used by the neighbourhood clip. */
export function rgbToYCoCg(r: number, g: number, b: number): [number, number, number] {
  return [0.25 * r + 0.5 * g + 0.25 * b, 0.5 * r - 0.5 * b, -0.25 * r + 0.5 * g - 0.25 * b];
}
