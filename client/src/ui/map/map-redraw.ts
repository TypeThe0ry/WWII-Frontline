/** Redraw budgeting (UI_SPEC §8.4): static base only on demand, dynamic layer ≤ 15 fps (30 when open), paused when hidden. */
export interface Redraw {
  baseBakes: number;
  dynamicDraws: number;
}

export class MapScheduler {
  readonly stats: Redraw = { baseBakes: 0, dynamicDraws: 0 };
  private baseDirty = true;
  private dragging = false;
  private lastDynamic = -Infinity;
  /** Dynamic frame interval in ms (15 fps closed/deploy, 30 fps open). */
  constructor(private intervalMs = 1000 / 30) {}

  setInterval(ms: number): void {
    this.intervalMs = ms;
  }
  /** Zoom/pan changed. While dragging the cached bitmap is CSS-translated (no bake); the bake happens on release. */
  viewChanged(): void {
    this.baseDirty = true;
  }
  dragStart(): void {
    this.dragging = true;
  }
  dragEnd(): void {
    this.dragging = false;
  }
  /** One animation frame: returns which work to do. */
  frame(now: number, visible: boolean): { bakeBase: boolean; drawDynamic: boolean } {
    if (!visible) return { bakeBase: false, drawDynamic: false };
    const bakeBase = this.baseDirty && !this.dragging;
    if (bakeBase) {
      this.baseDirty = false;
      this.stats.baseBakes++;
    }
    const drawDynamic = now - this.lastDynamic >= this.intervalMs - 1e-6;
    if (drawDynamic) {
      this.lastDynamic = now;
      this.stats.dynamicDraws++;
    }
    return { bakeBase, drawDynamic };
  }
}

/** Unit staleness (TEAM data > 5 s old → dashed box + "N s ago"). */
export const UNIT_STALE_S = 5;
export function unitStaleness(lastUpdateS: number, nowS: number): { stale: boolean; agoS: number } {
  const ago = Math.max(0, nowS - lastUpdateS);
  return { stale: ago > UNIT_STALE_S, agoS: Math.floor(ago) };
}
