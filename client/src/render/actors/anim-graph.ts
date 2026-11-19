/**
 * Animation graph logic (M1-045, GAMEDESIGN 9.2): stance transition times, locomotion state choice,
 * footstep events and character LOD. The runtime samples the existing actor feed and produces
 * procedural pose weights, independently of scene nodes and assets.
 */
export type Stance = 'stand' | 'crouch' | 'prone';

export type LocoState =
  | 'idle' | 'walk' | 'run' | 'sprint'
  | 'crouch_idle' | 'crouch_walk'
  | 'prone_idle' | 'prone_crawl' | 'airborne';

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

/** Low three bits of `entities.anim`, as authored by the server's player snapshot scheduler. */
export const MOTION = { idle: 0, walk: 1, run: 2, sprint: 3, airborne: 4 } as const;
export const RELOAD_BIT = 0x8;
const HURT_BIT = 0x2;

const stanceName = (wire: number): Stance => wire === 2 ? 'prone' : wire === 0 ? 'stand' : 'crouch';
const STANCE_POSE: Record<Stance, number> = { stand: 0, crouch: 1, prone: 2 };
const STRIDE_GAIN: Record<LocoState, number> = {
  idle: 0, walk: 0.55, run: 0.8, sprint: 1,
  crouch_idle: 0, crouch_walk: 0.45, prone_idle: 0, prone_crawl: 0.3, airborne: 0,
};
const mix = (from: number, to: number, t: number): number => from + (to - from) * t;
const WIRE_GAIT = [undefined, 'walk', 'run', 'sprint'] as const;

function sampleLocomotion(stance: Stance, motion: number, speed: number): LocoState {
  if (motion === MOTION.airborne) return 'airborne';
  const gait = WIRE_GAIT[motion];
  if (stance === 'stand' && gait) return gait;
  // Grounded wire hints keep the gait selected when two render frames sample the same position.
  return locoState(stance, gait ? Math.max(0.3, speed) : speed, false);
}

export interface AnimSample {
  stance: number;
  anim: number;
  flags: number;
  health: number;
}

type LifeState = 'alive' | 'hit_react' | 'downed' | 'dead';
type UpperState = 'aim' | 'fire' | 'reload' | 'disabled';

/** Per-actor lower-body locomotion, upper-body actions and life override. No gameplay writes. */
export class AnimGraph {
  state: LocoState = 'idle';
  upper: UpperState = 'aim';
  life: LifeState = 'alive';
  stance: Stance = 'stand';
  /** Continuous stand (0) -> crouch (1) -> prone (2) pose coordinate. */
  stancePose = 0;
  transitionLeft = 0;
  strideWeight = 0;
  airWeight = 0;
  reloadWeight = 0;
  recoil = 0;
  hitWeight = 0;

  private ready = false;
  private stanceFrom = 0;
  private stanceElapsed = 0;
  private stanceDuration = 0;
  private viaCrouch = false;
  private fade = 1;
  private strideFrom = 0;
  private airFrom = 0;
  private wasHurt = false;
  private eventLife: 'downed' | 'dead' | null = null;
  private readonly feet = new FootstepClock();

  get cycle(): number { return this.feet.cycle; }
  get incapacitated(): boolean { return this.life === 'downed' || this.life === 'dead'; }

  /** Render transition diagnostic; it never gates a confirmed remote-shot event. */
  get canFire(): boolean { return this.transitionLeft === 0 && !this.incapacitated; }

  fire(): void { this.recoil = Math.min(0.12, this.recoil + 0.06); }
  resetWeaponLayer(): void { this.recoil = this.reloadWeight = 0; }
  death(canBeRevived: boolean): void { this.eventLife = canBeRevived ? 'downed' : 'dead'; }
  revive(): void { this.eventLife = null; }

  /** `distanceM` is measured from consecutive interpolated positions, excluding teleports. */
  update(dtS: number, sample: Readonly<AnimSample>, distanceM: number): 'left' | 'right' | undefined {
    this.updateStance(dtS, stanceName(sample.stance));

    // There is no remote velocity, distinct vault/throw code or snapshot downed bit in this feed.
    const motion = sample.anim & 7;
    const hurt = (sample.flags & HURT_BIT) !== 0;
    if (sample.health === 0) this.eventLife = null; // zero HP confirms death; positive HP can start a new life
    this.hitWeight = hurt && !this.wasHurt ? 1 : Math.max(0, this.hitWeight - dtS / 0.18);
    this.wasHurt = hurt;
    this.life = sample.health === 0 ? 'dead' : this.eventLife ?? (this.hitWeight > 0 ? 'hit_react' : 'alive');
    if (this.incapacitated) {
      this.hitWeight = 0;
      this.resetWeaponLayer();
      this.upper = 'disabled';
    } else {
      this.recoil = Math.max(0, this.recoil - dtS * 0.5);
      const reloading = (sample.anim & RELOAD_BIT) !== 0;
      const reloadStep = dtS / 0.15;
      this.reloadWeight = reloading ? Math.min(1, this.reloadWeight + reloadStep) : Math.max(0, this.reloadWeight - reloadStep);
      this.upper = this.recoil > 0 ? 'fire' : this.reloadWeight > 0 || reloading ? 'reload' : 'aim';
    }

    const speed = dtS > 0 ? distanceM / dtS : 0;
    const next = this.incapacitated ? locoState(this.stance, 0, false) : sampleLocomotion(this.stance, motion, speed);
    if (next !== this.state) {
      this.state = next;
      this.strideFrom = this.strideWeight;
      this.airFrom = this.airWeight;
      this.fade = 0;
    }
    this.fade = Math.min(1, this.fade + dtS / 0.2);
    this.strideWeight = mix(this.strideFrom, STRIDE_GAIN[next], this.fade);
    this.airWeight = mix(this.airFrom, Number(next === 'airborne'), this.fade);
    if (this.incapacitated || STRIDE_GAIN[next] === 0) return;
    const foot = this.feet.advance(distanceM, this.stance === 'stand' ? 1.6 : 1.2);
    return this.stance === 'prone' ? undefined : foot;
  }

  private updateStance(dtS: number, stance: Stance): void {
    if (!this.ready) {
      this.ready = true;
      this.stance = stance;
      this.stancePose = STANCE_POSE[stance];
      return;
    }
    if (stance !== this.stance) {
      this.stanceFrom = this.stancePose;
      this.stanceElapsed = 0;
      this.stanceDuration = stanceTransitionS(this.stance, stance);
      this.viaCrouch = this.stance === 'stand' && stance === 'prone' && this.stanceFrom < 1;
      this.stance = stance;
    }
    if (this.stanceDuration === 0) return;
    this.stanceElapsed = Math.min(this.stanceDuration, this.stanceElapsed + dtS);
    this.transitionLeft = this.stanceDuration - this.stanceElapsed;
    const crouchS = stanceTransitionS('stand', 'crouch');
    this.stancePose = this.viaCrouch
      ? this.stanceElapsed < crouchS
        ? mix(this.stanceFrom, STANCE_POSE.crouch, this.stanceElapsed / crouchS)
        : mix(STANCE_POSE.crouch, STANCE_POSE.prone, (this.stanceElapsed - crouchS) / (this.stanceDuration - crouchS))
      : mix(this.stanceFrom, STANCE_POSE[this.stance], this.stanceElapsed / this.stanceDuration);
    if (this.transitionLeft === 0) this.stanceDuration = 0;
  }
}

/**
 * Footstep events from the stride phase: one event each time the phase crosses a half stride,
 * alternating left/right. `strideM` = distance for a full left+right cycle.
 */
export class FootstepClock {
  private phase = 0;
  private nextLeft = true;

  get cycle(): number { return this.phase % 1; }

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
