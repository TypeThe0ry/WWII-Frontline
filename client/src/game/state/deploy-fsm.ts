/**
 * Deploy state machine (M1-040, GAMEDESIGN 2.2). The server's `DeployResult` and `self_state` are
 * authoritative: this class only rejects transitions the client could not legally make itself and
 * lets `sync` overwrite the state from the server.
 *
 * Idle -> Choosing -> Requested -> Counting -> Spawned <-> Downed -> Dead -> Choosing
 */
export type DeployPhase = 'Idle' | 'Choosing' | 'Requested' | 'Counting' | 'Spawned' | 'Downed' | 'Dead';

export type DeployEvent =
  | { type: 'open' }
  | { type: 'request' }
  | { type: 'result'; ok: true; spawnTick: number }
  | { type: 'result'; ok: false; reason: string }
  | { type: 'spawned' }
  | { type: 'downed' }
  | { type: 'revived' }
  | { type: 'died' }
  | { type: 'respawnReady' }
  | { type: 'sync'; phase: DeployPhase }
  | { type: 'reset' };

export interface DeployState {
  phase: DeployPhase;
  /** Tick the spawn completes at (`Counting` only). */
  spawnTick: number | undefined;
  /** i18n key of the last rejection (`Choosing` after a refused request). */
  reason: string | undefined;
}

const NEXT: Record<DeployPhase, Partial<Record<DeployEvent['type'], DeployPhase>>> = {
  Idle: { open: 'Choosing' },
  Choosing: { request: 'Requested' },
  Requested: {}, // `result` decides
  Counting: { spawned: 'Spawned', died: 'Dead' },
  Spawned: { downed: 'Downed', died: 'Dead' },
  Downed: { revived: 'Spawned', died: 'Dead' },
  Dead: { respawnReady: 'Choosing' },
};

export class DeployFsm {
  state: DeployState = { phase: 'Idle', spawnTick: undefined, reason: undefined };
  /** Illegal transitions seen (diagnostic). */
  illegal = 0;

  get phase(): DeployPhase {
    return this.state.phase;
  }

  /** Applies `ev`; returns false (state unchanged) when the transition is not legal. */
  apply(ev: DeployEvent): boolean {
    const s = this.state;
    if (ev.type === 'reset') {
      this.state = { phase: 'Idle', spawnTick: undefined, reason: undefined };
      return true;
    }
    if (ev.type === 'sync') {
      this.state = { phase: ev.phase, spawnTick: ev.phase === 'Counting' ? s.spawnTick : undefined, reason: undefined };
      return true;
    }
    if (ev.type === 'result') {
      if (s.phase !== 'Requested') return this.reject();
      this.state = ev.ok
        ? { phase: 'Counting', spawnTick: ev.spawnTick, reason: undefined }
        : { phase: 'Choosing', spawnTick: undefined, reason: ev.reason };
      return true;
    }
    const to = NEXT[s.phase][ev.type];
    if (to === undefined) return this.reject();
    this.state = { phase: to, spawnTick: undefined, reason: undefined };
    return true;
  }

  private reject(): false {
    this.illegal++;
    return false;
  }
}
