import type { SettingsStore } from '../../game/settings';
import type { ActionId, Bindings, Context } from '../../input/bindings';
import type { InputSampler } from '../../input/sampler';

/** The M0 sampler's logical input codes; physical bindings are resolved here instead of changing its keymap. */
const SAMPLER_CODES: Readonly<Partial<Record<ActionId, string>>> = {
  move_fwd: 'KeyW', move_back: 'KeyS', move_left: 'KeyA', move_right: 'KeyD',
  sprint: 'ShiftLeft', jump: 'Space', crouch: 'KeyC', fire: 'Mouse0', ads: 'Mouse2', reload: 'KeyR',
};

export class MatchInputAdapter {
  private readonly held = new Map<string, readonly ActionId[]>();
  private readonly counts = new Map<ActionId, number>();
  private walking = false;
  private context: Context | undefined;

  constructor(
    private readonly sampler: InputSampler,
    private readonly bindings: Bindings,
    private readonly settings: SettingsStore,
    private readonly getContext: () => Context,
    private readonly onAction?: (action: ActionId, pressed: boolean) => void,
  ) {}

  press(code: string): boolean {
    const context = this.syncContext();
    if (this.held.has(code)) return true; // idempotent key repeats
    const actions = this.bindings.actionsFor(code, context).filter((a) => a !== 'pause');
    if (!actions.length) {
      // Preserve the legacy M0 debug walk key if the user hasn't bound CapsLock here.
      if (code !== 'CapsLock' || context !== 'onFoot') return false;
      this.walking = true;
      this.sampler.press(code);
    }
    this.held.set(code, actions);
    for (const a of actions) {
      const count = this.counts.get(a) ?? 0;
      this.counts.set(a, count + 1);
      if (!count) this.apply(a, true);
    }
    return true;
  }

  release(code: string): void {
    this.syncContext();
    const actions = this.held.get(code);
    if (!actions) return;
    this.held.delete(code);
    if (code === 'CapsLock' && this.walking) {
      this.walking = false;
      this.sampler.release(code);
    }
    for (const a of actions) {
      const count = this.counts.get(a)! - 1;
      if (count) this.counts.set(a, count);
      else {
        this.counts.delete(a);
        this.apply(a, false);
      }
    }
  }

  mouseMove(dx: number, dy: number): void {
    const context = this.syncContext();
    if (context === 'menu' || context === 'map' || context === 'chat') return;
    const ads = this.counts.has('ads') ? this.settings.get<number>('input.adsMult') : 1;
    const invert = this.settings.get<boolean>('input.invertY') === this.sampler.invertY ? 1 : -1;
    // Retain the sampler's degrees-per-pixel base (including legacy local sensitivity).
    this.sampler.mouseMove(dx * this.settings.get<number>('input.sensH') * ads,
      dy * this.settings.get<number>('input.sensV') * ads * invert);
  }

  releaseAll(): void {
    const actions = [...this.counts.keys()];
    this.counts.clear();
    this.held.clear();
    this.walking = false;
    this.sampler.releaseAll();
    for (const a of actions) if (!SAMPLER_CODES[a]) this.onAction?.(a, false);
  }

  private syncContext(): Context {
    const next = this.getContext();
    if (this.context !== next) {
      this.context = next;
      this.releaseAll();
    }
    return next;
  }

  private apply(action: ActionId, pressed: boolean): void {
    const code = SAMPLER_CODES[action];
    if (code) {
      if (pressed) this.sampler.press(code);
      else this.sampler.release(code);
    } else this.onAction?.(action, pressed);
  }
}
