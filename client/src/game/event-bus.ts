/**
 * Client event bus (ARCHITECTURE §8.4): typed, synchronous, allocation-free on the emit path.
 *
 * - `emit` calls the subscribers of that event in subscription order, on the caller's stack.
 * - A throwing subscriber does not stop the others; the error goes to `onError` (default: console.error).
 * - Subscriber lists are copy-on-write, so `on`/`off` during an `emit` are safe (they take effect
 *   from the next emit) and `emit` itself allocates nothing.
 * - Event payload objects on hot paths come from an `ObjectPool` (acquire, fill, emit, release).
 *
 * M0-022 and later add entries to `GameEvents`; they must not change this implementation.
 */

/** Event catalogue. Keys are `<producer>.<event>`; extend by declaration merging or by editing here. */
export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface GameEvents {
  'net.connected': { transport: 'wt' | 'ws'; rtt: number };
  'net.disconnected': { reason: string };
  // --- M0-022 (game): snapshot / entity / combat events. Payload objects of the hot events are
  // reused by the publisher: read them inside the handler, never keep a reference. ---
  'snapshot.applied': { tick: number; entityCount: number };
  'entity.spawned': { id: number; kind: 'player'; pos: Vec3 };
  'entity.removed': { id: number; reason: 'removed' | 'timeout' };
  /** The local player fired (prediction): never carries a hit result. `dir` is a unit vector. */
  'fire.local': { weapon: number; muzzlePos: Vec3; dir: Vec3 };
  'fire.remote': { shooter: number; weapon: number; muzzlePos: Vec3; dir: Vec3; distance: number };
  /** A remote or local character's foot lands (M1-045b): from the stride phase of its movement, alternating feet. Read inside the handler (the payload is reused). */
  'entity.footstep': { entity: number; foot: 'left' | 'right'; pos: Vec3; stance: number };
  'impact': { pos: Vec3; normal: number; material: number };
  /** An explosion at `pos` with damage radius `radius` (m, from the explosive's `damage_radius_m`); `own` = caused by the local player. Decoded from C2 type 5 (`own` is always false: the wire does not say who threw it). */
  'explosion': { pos: Vec3; radius: number; own: boolean };
  /** Bullets cracked past the local player; `angle` in radians (0 = ahead, clockwise) of the shooter, `intensity` 0..1 (C2 type 6). */
  'suppress': { angle: number; intensity: number };
  /** Server-confirmed hit by the local player (the only place hits are ever reported). */
  'hit.confirm': { target: number; zone: number; damage: number; killed: boolean };
  /** `angle` in radians (0 = ahead of the player's view, clockwise), `amount` 0..1. */
  'damage.taken': { angle: number; amount: number };
  // --- M1-040 (game state, from C0 / C4): server-authoritative state changes. ---
  'kill.feed': { killer: number; victim: number; weapon: number; headshot: boolean; dist: number };
  'deploy.state': { phase: string; spawnTick?: number; reason?: string };
  'objective.changed': { objective: number; owner: number; reason: number };
  'revive': { medic: number; target: number };
  'roster.changed': { player: number; joined: boolean };
  'spawnpoint.changed': { id: number; team: number; state: number };
  'match.phase': { phase: number; endsAt: number };
  'notice': { code: number; args: readonly [number, number, number] };
  'death': { victim: number; respawnAfter: number; canBeRevived: boolean };
}

export type Listener<P> = (payload: P) => void;

export class EventBus<M extends object = GameEvents> {
  private readonly lists = new Map<keyof M, Listener<never>[]>();

  /** Called with (error, eventName) when a subscriber throws. */
  onError: (error: unknown, event: keyof M) => void = (error, event) => {
    console.error(`[event-bus] listener for '${String(event)}' threw`, error);
  };

  /** Subscribes; returns an unsubscribe function. The same function may be subscribed twice (called twice). */
  on<K extends keyof M>(type: K, fn: Listener<M[K]>): () => void {
    const cur = this.lists.get(type);
    this.lists.set(type, cur ? [...cur, fn as Listener<never>] : [fn as Listener<never>]);
    return () => this.off(type, fn);
  }

  /** Subscribes for exactly one emit. */
  once<K extends keyof M>(type: K, fn: Listener<M[K]>): () => void {
    const off = this.on(type, (p) => {
      off();
      fn(p);
    });
    return off;
  }

  /** Removes the most recent subscription of `fn`; no-op when it is not subscribed. */
  off<K extends keyof M>(type: K, fn: Listener<M[K]>): void {
    const cur = this.lists.get(type);
    if (!cur) return;
    const i = cur.lastIndexOf(fn as Listener<never>);
    if (i < 0) return;
    if (cur.length === 1) this.lists.delete(type);
    else this.lists.set(type, [...cur.slice(0, i), ...cur.slice(i + 1)]);
  }

  /** Synchronous dispatch. Allocation-free unless a subscriber throws. */
  emit<K extends keyof M>(type: K, payload: M[K]): void {
    const cur = this.lists.get(type);
    if (!cur) return;
    for (let i = 0; i < cur.length; i++) {
      try {
        (cur[i] as Listener<M[K]>)(payload);
      } catch (e) {
        this.onError(e, type);
      }
    }
  }

  listenerCount(type: keyof M): number {
    return this.lists.get(type)?.length ?? 0;
  }

  clear(): void {
    this.lists.clear();
  }
}

/** Free-list of reusable payload objects, so hot-path events do not allocate. */
export class ObjectPool<T extends object> {
  private readonly free: T[] = [];

  constructor(
    private readonly create: () => T,
    private readonly reset: (o: T) => void = () => undefined,
    prewarm = 0,
  ) {
    for (let i = 0; i < prewarm; i++) this.free.push(create());
  }

  acquire(): T {
    return this.free.pop() ?? this.create();
  }

  release(o: T): void {
    this.reset(o);
    this.free.push(o);
  }

  get available(): number {
    return this.free.length;
  }
}
