/**
 * Faction selectability (GAMEDESIGN §2.4; server-authoritative, this is a UI pre-check only).
 * All numbers come from the server-sent rules object; defaults mirror the GAMEDESIGN values.
 */
export interface FactionRules {
  cap: number; // per-team cap (humans+bots)
  maxHumanDiff: number; // joining allowed while H_t − H_other ≤ this
  switchCooldownS: number;
  noSwitchAfterJoinS: number;
}
export const DEFAULT_RULES: FactionRules = { cap: 50, maxHumanDiff: 2, switchCooldownS: 600, noSwitchAfterJoinS: 180 };

export type FactionStatus = 'open' | 'full' | 'unbalanced' | 'cooldown';
export interface FactionState {
  team: 0 | 1;
  humans: number;
  bots: number;
  cap: number;
  state: FactionStatus;
  reasonKey?: string;
}
export interface TeamCounts {
  humans: [number, number];
  bots: [number, number];
}

export const REASON_KEY: Readonly<Record<Exclude<FactionStatus, 'open'>, string>> = {
  full: 'ui.faction.reason.full',
  unbalanced: 'ui.faction.reason.unbalanced',
  cooldown: 'ui.faction.reason.cooldown',
};

/** `cooldownLeftS` > 0 means the player switched/joined recently (only relevant when changing team). */
export function factionState(team: 0 | 1, c: TeamCounts, rules: FactionRules = DEFAULT_RULES, cooldownLeftS = 0): FactionState {
  const other = team === 0 ? 1 : 0;
  const humans = c.humans[team];
  const bots = c.bots[team];
  let state: FactionStatus = 'open';
  // A bot slot is released for a human, so fullness counts humans plus bots only beyond free bot slots: cap applies to humans+bots,
  // but bots yield — a team is "full" when humans alone reach the cap.
  if (humans >= rules.cap) state = 'full';
  else if (humans - c.humans[other] > rules.maxHumanDiff) state = 'unbalanced';
  else if (cooldownLeftS > 0) state = 'cooldown';
  return { team, humans, bots, cap: rules.cap, state, ...(state === 'open' ? {} : { reasonKey: REASON_KEY[state] }) };
}

/** Auto-assign: the team with fewer humans, ties → fewer total, then team 0. Skips non-open teams; null if none. */
export function autoAssign(c: TeamCounts, rules: FactionRules = DEFAULT_RULES): 0 | 1 | null {
  const open = ([0, 1] as const).filter((t) => factionState(t, c, rules).state === 'open');
  if (open.length === 0) return null;
  if (open.length === 1) return open[0]!;
  const key = (t: 0 | 1) => [c.humans[t], c.humans[t] + c.bots[t]] as const;
  const [a, b] = [key(0), key(1)];
  return a[0] !== b[0] ? (a[0] < b[0] ? 0 : 1) : a[1] <= b[1] ? 0 : 1;
}

/** Balance bar fill (share of team 0 among humans; 0.5 when empty). */
export function balanceShare(c: TeamCounts): number {
  const tot = c.humans[0] + c.humans[1];
  return tot === 0 ? 0.5 : c.humans[0] / tot;
}
