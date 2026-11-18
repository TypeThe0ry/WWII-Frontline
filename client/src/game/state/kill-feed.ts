/** Kill feed: a preallocated ring of the 8 most recent kills (newest last when read). */
export interface KillEntry {
  killer: number;
  victim: number;
  weapon: number;
  headshot: boolean;
  dist: number;
}

export class KillFeed {
  static readonly CAP = 8;
  private readonly slots: KillEntry[] = Array.from({ length: KillFeed.CAP }, () => ({ killer: 0, victim: 0, weapon: 0, headshot: false, dist: 0 }));
  private next = 0;
  /** Entries currently held (<= CAP). */
  length = 0;
  /** Bumped on every push so views can detect changes cheaply. */
  version = 0;

  push(killer: number, victim: number, weapon: number, headshot: boolean, dist: number): void {
    const e = this.slots[this.next] as KillEntry;
    e.killer = killer;
    e.victim = victim;
    e.weapon = weapon;
    e.headshot = headshot;
    e.dist = dist;
    this.next = (this.next + 1) % KillFeed.CAP;
    if (this.length < KillFeed.CAP) this.length++;
    this.version++;
  }

  /** `i`-th oldest held entry (0 = oldest). */
  at(i: number): Readonly<KillEntry> | undefined {
    if (i < 0 || i >= this.length) return undefined;
    return this.slots[(this.next - this.length + i + KillFeed.CAP * 2) % KillFeed.CAP];
  }

  clear(): void {
    this.length = 0;
    this.next = 0;
    this.version++;
  }
}
