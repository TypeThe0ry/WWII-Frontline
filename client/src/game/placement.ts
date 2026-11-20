/**
 * Placement mode and vehicle-seat input logic (M1-060a/b). Legality itself is decided by sim (`PlaceError`, mirrored here
 * as a code → i18n key table) — the client only displays what the sim-wasm pre-check or the server returns.
 */
export type PlacementKind = 'garrison' | 'supply_crate' | 'ammo_crate' | 'outpost' | 'repair_station' | 'at_gun';
export type PlacementMode = 'idle' | 'placing' | 'invalid' | 'committing';

/** Mirrors `crates/sim/src/spawn/garrison.rs::PlaceError` (same order/names). */
export const PLACE_ERRORS = ['WrongRole', 'TooCloseHq', 'TooCloseOwnGarrison', 'TooCloseEnemyGarrison', 'TooCloseObjective', 'Slope', 'Water', 'EnemyNear', 'NoSupplies', 'TeamCap', 'SectorCap', 'NotAlive', 'InNoBuildZone'] as const;
export type PlaceError = (typeof PLACE_ERRORS)[number];
export const PLACE_ERROR_KEY: Readonly<Record<PlaceError, string>> = {
  WrongRole: 'ui.officer.err.wrongRole',
  TooCloseHq: 'ui.officer.err.tooCloseHq',
  TooCloseOwnGarrison: 'ui.officer.err.tooCloseOwnGarrison',
  TooCloseEnemyGarrison: 'ui.officer.err.tooCloseEnemyGarrison',
  TooCloseObjective: 'ui.officer.err.tooCloseObjective',
  Slope: 'ui.officer.err.slope',
  Water: 'ui.officer.err.water',
  EnemyNear: 'ui.officer.err.enemyNear',
  NoSupplies: 'ui.officer.err.noSupplies',
  TeamCap: 'ui.officer.err.teamCap',
  SectorCap: 'ui.officer.err.sectorCap',
  NotAlive: 'ui.officer.err.notAlive',
  InNoBuildZone: 'ui.officer.err.noBuildZone',
};

export interface PlacementResult {
  legal: boolean;
  reasonKey?: string;
  supplyCost?: number;
  remaining?: number;
}

/** Ghost visual state (UI_SPEC §4.12 wins over INDEX "green/red"): blue = legal, hatched+warn = illegal, grey = not enough supplies. */
export type GhostStyle = { color: 'blue' | 'warn' | 'grey'; pattern: 'solid' | 'hatched' | 'dotted'; icon: 'check' | 'warn' | 'minus' };
export function ghostStyle(r: PlacementResult, reasonCode?: PlaceError): GhostStyle {
  if (r.legal) return { color: 'blue', pattern: 'solid', icon: 'check' };
  if (reasonCode === 'NoSupplies' || r.reasonKey === PLACE_ERROR_KEY.NoSupplies) return { color: 'grey', pattern: 'dotted', icon: 'minus' };
  return { color: 'warn', pattern: 'hatched', icon: 'warn' };
}
export function resultFromError(code: PlaceError | null, supplyCost?: number, remaining?: number): PlacementResult {
  return code === null ? { legal: true, supplyCost, remaining } : { legal: false, reasonKey: PLACE_ERROR_KEY[code], supplyCost, remaining };
}

export type PlacementEvent =
  | { t: 'enter'; kind: PlacementKind }
  | { t: 'check'; result: PlacementResult }
  | { t: 'commit' }
  | { t: 'accepted' }
  | { t: 'rejected'; reasonKey: string }
  | { t: 'cancel' };

export interface PlacementState {
  mode: PlacementMode;
  kind: PlacementKind | null;
  reasonKey?: string;
  result?: PlacementResult;
}
export const PLACEMENT_IDLE: PlacementState = { mode: 'idle', kind: null };

export function placementNext(s: PlacementState, e: PlacementEvent): PlacementState {
  switch (e.t) {
    case 'enter':
      return s.mode === 'idle' ? { mode: 'placing', kind: e.kind } : s;
    case 'check':
      return s.mode === 'placing' || s.mode === 'invalid' ? { mode: e.result.legal ? 'placing' : 'invalid', kind: s.kind, reasonKey: e.result.reasonKey, result: e.result } : s;
    case 'commit':
      // Only a legal ghost may be committed; an invalid one stays invalid.
      return s.mode === 'placing' && s.result?.legal ? { mode: 'committing', kind: s.kind } : s;
    case 'accepted':
      return s.mode === 'committing' ? PLACEMENT_IDLE : s;
    case 'rejected':
      // Server refusal returns to the ghost with the server's reason.
      return s.mode === 'committing' ? { mode: 'placing', kind: s.kind, reasonKey: e.reasonKey } : s;
    case 'cancel':
      return PLACEMENT_IDLE;
  }
}

/** Tool availability: Garrison kit = Officer in a squad of ≥ 2; crates = Support; reasons are tooltip keys. */
export function toolAvailable(kind: PlacementKind, role: string, squadSize: number, alive: boolean): { ok: boolean; reasonKey?: string } {
  if (!alive) return { ok: false, reasonKey: PLACE_ERROR_KEY.NotAlive };
  if (kind === 'repair_station' || kind === 'at_gun') {
    return role === (kind === 'repair_station' ? 'engineer' : 'anti_tank')
      ? { ok: true } : { ok: false, reasonKey: PLACE_ERROR_KEY.WrongRole };
  }
  if (kind === 'garrison' || kind === 'outpost') {
    if (role !== 'officer') return { ok: false, reasonKey: 'ui.officer.tool.needOfficer' };
    if (squadSize < (kind === 'outpost' ? 3 : 2)) return { ok: false, reasonKey: 'ui.officer.tool.needSquad' };
    return { ok: true };
  }
  return role === 'support' ? { ok: true } : { ok: false, reasonKey: 'ui.officer.tool.needSupport' };
}

/** Garrison supply cost: 50 in own/neutral ground, 100 in enemy ground (GD §10.2, values from server rules in practice). */
export const garrisonCost = (zone: 'own' | 'neutral' | 'enemy', costs = { friendly: 50, enemy: 100 }): number => (zone === 'enemy' ? costs.enemy : costs.friendly);

/** Ammo crate cooldown left (30 s per placement). */
export const crateCooldownLeftS = (lastPlacedS: number | undefined, nowS: number, cooldownS = 30): number => (lastPlacedS === undefined ? 0 : Math.max(0, Math.ceil(lastPlacedS + cooldownS - nowS)));

/* ---- vehicle seat input ---- */
export const EXIT_HOLD_S = 0.8;
export const SEAT_MENU_HOLD_S = 0.35;
export type FAction = 'none' | 'enterNearest' | 'seatMenu' | 'exit';

/** `F` press resolved on release/hold: outside a vehicle a tap enters the nearest free seat and a hold opens the seat menu; inside, a hold of 0.8 s exits. */
export function fKeyAction(inVehicle: boolean, heldS: number, released: boolean): FAction {
  if (inVehicle) return heldS >= EXIT_HOLD_S ? 'exit' : 'none';
  if (heldS >= SEAT_MENU_HOLD_S) return 'seatMenu';
  return released ? 'enterNearest' : 'none';
}
export const exitProgress = (heldS: number): number => Math.min(1, Math.max(0, heldS / EXIT_HOLD_S));

/** Digit keys 1–3 map to driver/gunner/commander (1–5 reserved for larger vehicles). */
export function seatForKey(key: string, seatsInVehicle: readonly ('driver' | 'gunner' | 'commander')[]): 'driver' | 'gunner' | 'commander' | undefined {
  if (!/^[1-5]$/.test(key)) return undefined;
  return seatsInVehicle[Number(key) - 1];
}

/** Free seats ordered by preference (driver, gunner, commander) for "enter nearest free seat". */
export function nearestFreeSeat(occupied: Readonly<Record<'driver' | 'gunner' | 'commander', boolean>>): 'driver' | 'gunner' | 'commander' | undefined {
  return (['driver', 'gunner', 'commander'] as const).find((s) => !occupied[s]);
}
