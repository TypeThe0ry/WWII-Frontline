/**
 * Fixed-step accumulator (ARCHITECTURE §6.1): render frames accumulate real_dt, run 0..max_steps
 * simulation steps, and the remainder (alpha) is used for render interpolation.
 */
export const TICK_HZ = 30;
export const STEP_MS = 1000 / TICK_HZ;
export const MAX_STEPS = 5;

/** Tolerance (ms) so that e.g. exactly 3 * STEP_MS of real time yields exactly 3 steps. */
const EPS_MS = 1e-6;

export interface StepResult {
  /** Number of simulation steps to run this frame (0..maxSteps). */
  steps: number;
  /** Interpolation factor for rendering, in [0, 1). */
  alpha: number;
}

export class FixedStepper {
  readonly stepMs: number;
  readonly maxSteps: number;
  /** Number of frames where accumulated time exceeded maxSteps and was discarded. */
  stepperClamped = 0;
  /** Total number of steps handed out so far. */
  totalSteps = 0;
  private acc = 0;

  constructor(stepMs: number = STEP_MS, maxSteps: number = MAX_STEPS) {
    if (!(stepMs > 0)) throw new RangeError('stepMs must be > 0');
    if (!Number.isInteger(maxSteps) || maxSteps < 1) throw new RangeError('maxSteps must be an integer >= 1');
    this.stepMs = stepMs;
    this.maxSteps = maxSteps;
  }

  advance(realDtMs: number): StepResult {
    if (Number.isFinite(realDtMs) && realDtMs > 0) this.acc += realDtMs;
    const due = Math.floor((this.acc + EPS_MS) / this.stepMs);
    let steps = due;
    this.acc -= due * this.stepMs;
    if (steps > this.maxSteps) {
      // Spiral-of-death guard: run only maxSteps, drop the surplus time (acc already holds just the sub-step phase).
      steps = this.maxSteps;
      this.stepperClamped++;
    }
    if (this.acc < 0) this.acc = 0;
    this.totalSteps += steps;
    const alpha = Math.min(this.acc / this.stepMs, 1 - Number.EPSILON);
    return { steps, alpha };
  }

  reset(): void {
    this.acc = 0;
  }
}
