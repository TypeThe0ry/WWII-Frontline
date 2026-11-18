/**
 * Roster with incremental merge. Every delta carries a per-player `seq`; the highest `seq` wins
 * per player (removal is a tombstone with its own `seq`), so deltas applied in any order reach the
 * same roster as the in-order application.
 */
export interface RosterEntry {
  id: number;
  team: number;
  squad: number;
  role: number;
  name: string;
}

export interface RosterDelta {
  id: number;
  seq: number;
  /** `undefined` = the player left. */
  entry?: Omit<RosterEntry, 'id'>;
}

export class Roster {
  private readonly entries = new Map<number, RosterEntry>();
  private readonly seqs = new Map<number, number>();
  version = 0;

  apply(d: RosterDelta): boolean {
    const cur = this.seqs.get(d.id);
    if (cur !== undefined && d.seq <= cur) return false;
    this.seqs.set(d.id, d.seq);
    if (d.entry === undefined) this.entries.delete(d.id);
    else this.entries.set(d.id, { id: d.id, ...d.entry });
    this.version++;
    return true;
  }

  get(id: number): Readonly<RosterEntry> | undefined {
    return this.entries.get(id);
  }

  get size(): number {
    return this.entries.size;
  }

  /** Entries sorted by id (stable, for comparisons and UI). */
  list(): RosterEntry[] {
    return [...this.entries.values()].sort((a, b) => a.id - b.id);
  }

  /** Full rebuild after `Welcome` / reconnect. */
  replaceAll(entries: readonly RosterEntry[], seq: number): void {
    this.entries.clear();
    this.seqs.clear();
    for (const e of entries) {
      this.entries.set(e.id, { ...e });
      this.seqs.set(e.id, seq);
    }
    this.version++;
  }
}
