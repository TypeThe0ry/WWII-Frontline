/**
 * TAA -> FXAA runtime fallback trigger (M1-044b / PI-118): TAA costs an extra full-screen pass plus a history texture, and on a
 * machine that is already missing its frame budget it makes things worse. The guard watches the frame intervals and asks for the
 * fallback once the 95th percentile of a full window exceeds the limit. Pure: no engine.
 */
export class PostBudgetGuard {
  private readonly ring: number[];
  private n = 0;
  private head = 0;
  triggered = false;

  constructor(
    /** Frame-interval limit (ms) for the window's P95; 1.5 x the 60 fps interval by default. */
    readonly limitMs = 25,
    /** Window length in frames (3 s at 30 fps, 1.5 s at 60 fps). */
    readonly window = 90,
  ) {
    this.ring = new Array<number>(window).fill(0);
  }

  /** Feeds one frame interval (ms); returns true exactly once, when the fallback should happen. */
  observe(frameMs: number): boolean {
    if (this.triggered) return false;
    this.ring[this.head] = frameMs;
    this.head = (this.head + 1) % this.window;
    if (this.n < this.window) this.n++;
    if (this.n < this.window) return false;
    const sorted = [...this.ring].sort((a, b) => a - b);
    const p95 = sorted[Math.min(this.window - 1, Math.ceil(0.95 * this.window) - 1)] as number;
    if (p95 > this.limitMs) {
      this.triggered = true;
      return true;
    }
    return false;
  }
}
