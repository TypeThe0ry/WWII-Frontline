/** Decoded C4 reliable events, field-for-field with `crates/protocol/src/msg/c4.rs`. */
export type C4Event =
  | { type: 'kill'; victim: number; killer: number; weapon: number; headshot: boolean; distQ: number }
  | { type: 'death'; victim: number; respawnAfter: number; canBeRevived: boolean }
  | { type: 'revive'; medic: number; target: number }
  | { type: 'score'; player: number; kind: number; value: number }
  | { type: 'objective'; objective: number; owner: number; reason: number }
  | { type: 'spawnpoint'; id: number; kind: number; team: number; pos: readonly [number, number]; state: number }
  | { type: 'roster'; player: number; nick: string; team: number; squad: number; role: number; flags: number; seq: number; left?: boolean }
  | { type: 'notice'; code: number; args: readonly [number, number, number] }
  | { type: 'phase'; phase: number; endsAt: number };

/** `Kill.dist_q` is 0.25 m steps. */
export const DIST_Q_M = 0.25;
