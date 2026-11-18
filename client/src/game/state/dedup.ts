/** Sliding-window de-duplication of reliable event ids (C4 / C2.events are sent redundantly). */
export class EventDedup {
  static readonly WINDOW = 256;
  private readonly seen = new Set<number>();
  private readonly ring = new Int32Array(EventDedup.WINDOW).fill(-1);
  private head = 0;
  duplicates = 0;

  /** True the first time `id` is seen inside the window; false for a repeat. No allocation. */
  firstTime(id: number): boolean {
    if (this.seen.has(id)) {
      this.duplicates++;
      return false;
    }
    const old = this.ring[this.head] as number;
    if (old >= 0) this.seen.delete(old);
    this.ring[this.head] = id;
    this.head = (this.head + 1) % EventDedup.WINDOW;
    this.seen.add(id);
    return true;
  }

  reset(): void {
    this.seen.clear();
    this.ring.fill(-1);
    this.head = 0;
  }
}
