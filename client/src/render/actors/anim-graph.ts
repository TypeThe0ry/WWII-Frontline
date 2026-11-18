/**
 * Animation graph logic (M1-045, GAMEDESIGN 9.2): stance transition times, locomotion state choice,
 * footstep events and character LOD. Pure and clip-agnostic -- the renderer maps states to clips
 * (real or procedural) and applies the cross-fade weights computed here.
 */
export type Stance = 'stand' | 'crouch' | 'prone';

export type LocoState =
  | 'idle' | 'walk' | 'run' | 'sprint'
  | 'crouch_idle' | 'crouch_walk'
  | 'prone_idle' | 'prone_crawl';

/** Seconds to switch stance (GD 9.2); 0 for the same stance. */
export function stanceTransitionS(from: Stance, to: Stance, diving = false): number {
  if (from === to) return 0;
  if (from === 'stand' && to === 'crouch') return 0.25;
  if (from === 'crouch' && to === 'stand') return 0.25;
  if (from === 'crouch' && to === 'prone') return 0.5;
  if (from === 'prone' && to === 'crouch') return 0.6;
  if (from === 'stand' && to === 'prone') return diving ? 0.7 : 0.25 + 0.5; // via crouch unless diving
  return 0.9; // prone -> stand
}

/** Firing is blocked while the stance is changing (reload / ADS stay allowed). */
export const canFireDuringTransition = false;

export function locoState(stance: Stance, speedMps: number, sprinting: boolean): LocoState {
  const moving = speedMps > 0.15;
  if (stance === 'prone') return moving ? 'prone_crawl' : 'prone_idle';
  if (stance === 'crouch') return moving ? 'crouch_walk' : 'crouch_idle';
  if (!moving) return 'idle';
  if (sprinting) return 'sprint';
  return speedMps > 2.6 ? 'run' : 'walk';
}

/** Cross-fade between locomotion states and stance transitions (blend weights 0..1). */
export class AnimGraph {
  state: LocoState = 'idle';
  prev: LocoState = 'idle';
  /** 0 -> 1 progress of the cross-fade from `prev` to `state`. */
  blend = 1;
  private fadeS = 0.2;
  stance: Stance = 'stand';
  /** Seconds left of the current stance change (0 = none). */
  transitionLeft = 0;

  /** Whether the weapon may fire right now. */
  get canFire(): boolean {
    return this.transitionLeft <= 0;
  }

  update(dtS: number, stance: Stance, speedMps: number, sprinting: boolean, diving = false): void {
    if (stance !== this.stance) {
      this.transitionLeft = stanceTransitionS(this.stance, stance, diving);
      this.stance = stance;
    }
    if (this.transitionLeft > 0) this.transitionLeft = Math.max(0, this.transitionLeft - dtS);
    const next = locoState(this.stance, speedMps, sprinting);
    if (next !== this.state) {
      this.prev = this.state;
      this.state = next;
      this.blend = 0;
      this.fadeS = this.transitionLeft > 0 ? this.transitionLeft : 0.2;
    }
    if (this.blend < 1) this.blend = Math.min(1, this.blend + dtS / this.fadeS);
  }
}

/**
 * Footstep events from the stride phase: one event each time the phase crosses a half stride,
 * alternating left/right. `strideM` = distance for a full left+right cycle.
 */
export class FootstepClock {
  private phase = 0;
  private nextLeft = true;

  /** Returns 'left' | 'right' when a foot lands during this step, else undefined. */
  advance(distM: number, strideM = 1.6): 'left' | 'right' | undefined {
    const before = Math.floor(this.phase * 2 + 1e-9);
    this.phase += distM / strideM;
    if (Math.floor(this.phase * 2 + 1e-9) > before) {
      const foot = this.nextLeft ? 'left' : 'right';
      this.nextLeft = !this.nextLeft;
      return foot;
    }
    return undefined;
  }
}

/** LOD index for a distance with hysteresis so a character on a boundary does not flicker. */
export function lodFor(distM: number, current: number, thresholds: readonly number[], hysteresisM = 5): number {
  let lod = current;
  while (lod < thresholds.length && distM > (thresholds[lod] as number) + hysteresisM) lod++;
  while (lod > 0 && distM < (thresholds[lod - 1] as number) - hysteresisM) lod--;
  return lod;
}
