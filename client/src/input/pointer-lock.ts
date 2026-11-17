/**
 * Pointer Lock + DOM event wiring for the sampler. Targets are injected (any `EventTarget`), so the
 * logic is unit-testable without a DOM. Losing pointer lock (Esc, alt-tab, tab hidden, window blur)
 * clears every key bit so a stuck key can never keep the character running.
 */
import type { InputSampler } from './sampler';

export interface LockTarget extends EventTarget {
  requestPointerLock?: (opts?: { unadjustedMovement?: boolean }) => Promise<void> | void;
}
export interface LockDocument extends EventTarget {
  pointerLockElement: unknown;
  hidden?: boolean;
  exitPointerLock?: () => void;
}

interface KeyLike {
  code: string;
  repeat?: boolean;
  preventDefault?: () => void;
}
interface MouseLike {
  button: number;
  movementX?: number;
  movementY?: number;
}

export class PointerLockController {
  private readonly cleanups: (() => void)[] = [];
  /** `true` after a successful lock (updated from `pointerlockchange`). */
  locked = false;
  /** Number of times the lock disappeared (diagnostic). */
  lostCount = 0;
  /** Keys that must not trigger the browser default while playing. */
  private readonly swallow = new Set(['Space', 'KeyW', 'KeyA', 'KeyS', 'KeyD']);

  constructor(
    private readonly sampler: InputSampler,
    private readonly canvas: LockTarget,
    private readonly doc: LockDocument,
    /** window-like object for keyboard / blur events */
    private readonly win: EventTarget,
  ) {}

  attach(): void {
    const on = (t: EventTarget, type: string, fn: (e: never) => void): void => {
      t.addEventListener(type, fn as EventListener);
      this.cleanups.push(() => t.removeEventListener(type, fn as EventListener));
    };
    on(this.canvas, 'click', () => this.request());
    on(this.doc, 'pointerlockchange', () => {
      const now = this.doc.pointerLockElement === this.canvas;
      if (this.locked && !now) {
        this.lostCount++;
        this.sampler.releaseAll();
      }
      this.locked = now;
    });
    on(this.doc, 'pointerlockerror', () => this.sampler.releaseAll());
    on(this.doc, 'visibilitychange', () => {
      if (this.doc.hidden) this.sampler.releaseAll();
    });
    on(this.win, 'blur', () => this.sampler.releaseAll());
    on(this.win, 'keydown', (e: KeyLike) => {
      if (!this.locked) return;
      if (this.swallow.has(e.code)) e.preventDefault?.();
      this.sampler.press(e.code);
    });
    on(this.win, 'keyup', (e: KeyLike) => this.sampler.release(e.code));
    on(this.canvas, 'mousedown', (e: MouseLike) => {
      if (this.locked) this.sampler.press(`Mouse${e.button}`);
    });
    on(this.win, 'mouseup', (e: MouseLike) => this.sampler.release(`Mouse${e.button}`));
    on(this.canvas, 'mousemove', (e: MouseLike) => {
      if (this.locked) this.sampler.mouseMove(e.movementX ?? 0, e.movementY ?? 0);
    });
  }

  request(): void {
    try {
      // unadjustedMovement (raw mouse, no OS acceleration) is optional; browsers without it reject.
      const r = this.canvas.requestPointerLock?.({ unadjustedMovement: true });
      if (r && typeof (r as Promise<void>).catch === 'function') {
        (r as Promise<void>).catch(() => {
          try {
            void this.canvas.requestPointerLock?.();
          } catch {
            /* lock refused: stays unlocked */
          }
        });
      }
    } catch {
      /* ignored */
    }
  }

  detach(): void {
    for (const c of this.cleanups.splice(0)) c();
    this.sampler.releaseAll();
  }
}
