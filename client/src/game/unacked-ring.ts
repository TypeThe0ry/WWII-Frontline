/**
 * Unacknowledged input ring (ARCHITECTURE 6.3): the last 128 simulated commands together with the
 * predicted state right after each one, which is what reconciliation rolls back to and replays from.
 *
 * `base` is the state at `base.seq`, the newest command that is no longer in the ring (acknowledged by the
 * server, or dropped on overflow). A snapshot acknowledging exactly `base.seq` can therefore still be
 * reconciled. Sequence numbers are u16 and compared with wrap-around arithmetic. All storage is
 * preallocated; nothing here allocates per command.
 */
export const RING_CAPACITY = 128;
export const CMD_BYTES = 18;
export const AUTH_BYTES = 39;
export const WEAPON_BYTES = 33;

/** Signed distance `a - b` of two u16 sequence numbers. */
export const seqDiff = (a: number, b: number): number => ((a - b + 0x8000) & 0xffff) - 0x8000;

export interface StateRef {
  seq: number;
  tick: number;
  /** Predictor tick counter after this command. */
  predTick: number;
  x: number;
  y: number;
  z: number;
  /** `PlayerAuthState` bytes after the command. */
  auth: Uint8Array;
  /** `WeaponState` bytes after the command (`weaponLen` valid bytes, 0 without weapon). */
  weapon: Uint8Array;
  weaponLen: number;
}

export interface UnackedEntry extends StateRef {
  cmd: Uint8Array;
}

const newState = (): StateRef => ({ seq: 0, tick: 0, predTick: 0, x: 0, y: 0, z: 0, auth: new Uint8Array(AUTH_BYTES), weapon: new Uint8Array(WEAPON_BYTES), weaponLen: 0 });
const newEntry = (): UnackedEntry => ({ ...newState(), cmd: new Uint8Array(CMD_BYTES) });

export function copyState(dst: StateRef, src: StateRef): void {
  dst.seq = src.seq;
  dst.tick = src.tick;
  dst.predTick = src.predTick;
  dst.x = src.x;
  dst.y = src.y;
  dst.z = src.z;
  dst.auth.set(src.auth);
  dst.weapon.set(src.weapon);
  dst.weaponLen = src.weaponLen;
}

export type AckClass = 'base' | 'entry' | 'stale' | 'future';

export class UnackedRing {
  readonly capacity: number;
  readonly base: StateRef = newState();
  private readonly slots: UnackedEntry[];
  private head = 0;
  private n = 0;
  /** Entries dropped because the ring was full (no acknowledgement for `capacity` commands). */
  overflows = 0;
  /** Called on overflow (rate limiting is up to the sink). */
  onOverflow: (dropped: UnackedEntry) => void = () => undefined;

  constructor(capacity = RING_CAPACITY) {
    this.capacity = capacity;
    this.slots = Array.from({ length: capacity }, newEntry);
  }

  get length(): number {
    return this.n;
  }

  /** i = 0 is the oldest. */
  at(i: number): UnackedEntry {
    return this.slots[(this.head + i) % this.capacity] as UnackedEntry;
  }

  newest(): UnackedEntry | undefined {
    return this.n > 0 ? this.at(this.n - 1) : undefined;
  }

  /** Next free entry (the caller fills it). When full, the oldest entry becomes `base` and is reused. */
  push(): UnackedEntry {
    if (this.n === this.capacity) {
      const old = this.at(0);
      this.overflows++;
      this.onOverflow(old);
      copyState(this.base, old);
      this.head = (this.head + 1) % this.capacity;
      this.n--;
    }
    const e = this.at(this.n);
    this.n++;
    return e;
  }

  /** How an acknowledgement of `seq` relates to the ring. */
  classify(seq: number): AckClass {
    if (seq === this.base.seq) return 'base';
    const last = this.newest();
    const lastSeq = last ? last.seq : this.base.seq;
    if (seqDiff(seq, lastSeq) > 0) return 'future';
    if (seqDiff(seq, this.base.seq) < 0) return 'stale';
    for (let i = 0; i < this.n; i++) if (this.at(i).seq === seq) return 'entry';
    return 'stale'; // a hole (cannot happen with consecutive seq numbers)
  }

  stateAt(seq: number): StateRef | undefined {
    if (seq === this.base.seq) return this.base;
    for (let i = this.n - 1; i >= 0; i--) {
      const e = this.at(i);
      if (e.seq === seq) return e;
    }
    return undefined;
  }

  /** Drops every entry up to and including `seq`; that entry's state becomes `base`. Returns false when `seq` is unknown. */
  discardUpTo(seq: number): boolean {
    if (seq === this.base.seq) return true;
    let idx = -1;
    for (let i = 0; i < this.n; i++) {
      if (this.at(i).seq === seq) {
        idx = i;
        break;
      }
    }
    if (idx < 0) return false;
    copyState(this.base, this.at(idx));
    this.head = (this.head + idx + 1) % this.capacity;
    this.n -= idx + 1;
    return true;
  }

  /** Empties the ring and sets `base` (used at start / forced resets). */
  clear(seq: number): void {
    this.head = 0;
    this.n = 0;
    this.base.seq = seq & 0xffff;
  }
}
