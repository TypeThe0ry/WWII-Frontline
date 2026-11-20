/** Stable `VoteKind` and `TeamBalanceCode` values from protocol/src/msg/vote.rs. */
export const VoteKind = { Kick: 0, RemoveCommander: 1, Mute: 2 } as const;
export type VoteKind = (typeof VoteKind)[keyof typeof VoteKind];

export const TeamBalanceCode = {
  Invite: 1, Moved: 2, Denied: 3, AfkWarning: 4, AfkCountdown: 5, TkWarning: 6,
  LockRole: 7, LockVehicle: 8, VoteDenied: 9, DriverVacant: 10, Muted: 11, RepeatInput: 12, AfkCancelled: 13,
} as const;
export type TeamBalanceCode = (typeof TeamBalanceCode)[keyof typeof TeamBalanceCode];
export const TeamMoveFlags = { Forced: 1, VolunteerBonus: 2 } as const;

export const ScoreCategory = { Combat: 0, Offense: 1, Defense: 2, Support: 3 } as const;
export type ScoreCategory = (typeof ScoreCategory)[keyof typeof ScoreCategory];

/** Decoded C4 reliable events, including category and reason unpacked from the score kind. */
export type C4Event =
  | { type: 'kill'; victim: number; killer: number; weapon: number; headshot: boolean; distQ: number }
  | { type: 'death'; victim: number; respawnAfter: number; canBeRevived: boolean }
  | { type: 'revive'; medic: number; target: number }
  | { type: 'score'; player: number; kind: number; category: ScoreCategory; reason: number; value: number }
  | { type: 'objective'; objective: number; owner: number; reason: number }
  | { type: 'spawnpoint'; id: number; kind: number; team: number; pos: readonly [number, number]; state: number }
  // Positions use the same quantized integer representation as spawnpoints.
  | { type: 'structure'; id: number; kind: number; pos: readonly [number, number]; state: number }
  | { type: 'roster'; player: number; nick: string; team: number; squad: number; role: number; flags: number; seq: number; left?: boolean }
  | { type: 'notice'; code: number; args: readonly [number, number, number] }
  | { type: 'phase'; phase: number; endsAt: number }
  | { type: 'voteState'; voteId: number; kind: VoteKind; target: number; yes: number; no: number; endsInS: number }
  | { type: 'voteResult'; voteId: number; passed: boolean }
  | { type: 'tkPrompt'; killer: number; expiresS: number; victimEvent: number }
  | { type: 'teamBalanceNotice'; code: TeamBalanceCode; args: readonly [number, number] }
  | { type: 'mapVoteState'; voteId: number; candidates: readonly [number, number, number]; votes: readonly [number, number, number]; endsInS: number }
  | { type: 'mapVoteResult'; voteId: number; winner: number };

/** `Kill.dist_q` is 0.25 m steps. */
export const DIST_Q_M = 0.25;

/** `PlayerRoster.flags::ALIVE` in protocol/src/msg/c4.rs. */
export const ROSTER_ALIVE = 1 << 5;
// Existing C4 Notice envelope; keep identifiers in sync with protocol::msg::c4::notice_code.
export const NET_DEGRADED = 0x0200;
export const NET_RECOVERED = 0x0201;
