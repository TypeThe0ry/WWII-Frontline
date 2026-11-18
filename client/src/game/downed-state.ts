/** Downed / dead UI logic (UI_SPEC §4.10, GAMEDESIGN §7.2). Numbers come from server rules; defaults mirror GAMEDESIGN. */
export type DownedPhase = 'alive' | 'downed' | 'dead' | 'spectating';

export interface DownedRules {
  downedHp: number;
  bleedHpPerS: number;
  giveUpAfterS: number; // give-up becomes available this long after going down
  giveUpHoldS: number;
  callCooldownS: number;
}
export const DEFAULT_DOWNED_RULES: DownedRules = { downedHp: 40, bleedHpPerS: 0.5, giveUpAfterS: 5, giveUpHoldS: 3, callCooldownS: 5 };

export type DownedEvent = 'down' | 'revive' | 'die' | 'spectate' | 'respawn';

export function downedNext(p: DownedPhase, e: DownedEvent): DownedPhase {
  switch (p) {
    case 'alive':
      return e === 'down' ? 'downed' : e === 'die' ? 'dead' : p;
    case 'downed':
      return e === 'revive' ? 'alive' : e === 'die' ? 'dead' : p;
    case 'dead':
      return e === 'spectate' ? 'spectating' : e === 'respawn' ? 'alive' : p;
    case 'spectating':
      return e === 'respawn' ? 'alive' : p;
  }
}

/** Remaining bleed-out seconds from the server's current downed HP. */
export const bleedoutRemainingS = (hp: number, r: DownedRules = DEFAULT_DOWNED_RULES): number => Math.max(0, hp / r.bleedHpPerS);
export const bleedoutFraction = (hp: number, r: DownedRules = DEFAULT_DOWNED_RULES): number => Math.min(1, Math.max(0, hp / r.downedHp));

/** Give-up gate: disabled for the first `giveUpAfterS` after going down, and while the link is unstable. */
export function giveUpState(sinceDownS: number, connected: boolean, r: DownedRules = DEFAULT_DOWNED_RULES): { enabled: boolean; reasonKey?: string; waitS: number } {
  if (!connected) return { enabled: false, reasonKey: 'ui.downed.reason.reconnecting', waitS: 0 };
  const waitS = Math.max(0, r.giveUpAfterS - sinceDownS);
  return waitS > 0 ? { enabled: false, reasonKey: 'ui.downed.reason.giveUpWait', waitS: Math.ceil(waitS) } : { enabled: true, waitS: 0 };
}
/** Hold progress 0..1 for the give-up button. */
export const giveUpProgress = (heldS: number, r: DownedRules = DEFAULT_DOWNED_RULES): number => Math.min(1, Math.max(0, heldS / r.giveUpHoldS));

/** Call-medic cooldown: remaining whole seconds (0 = ready). */
export const callCooldownLeftS = (lastCallS: number | undefined, nowS: number, r: DownedRules = DEFAULT_DOWNED_RULES): number =>
  lastCallS === undefined ? 0 : Math.max(0, Math.ceil(lastCallS + r.callCooldownS - nowS));

/* ---- nearest medic ---- */
export interface MedicInfo {
  id: number;
  x: number;
  z: number;
  alive: boolean;
  downed: boolean;
  enRoute: boolean;
}
export function nearestMedic(px: number, pz: number, medics: readonly MedicInfo[]): { medic: MedicInfo; distM: number; bearingRad: number } | null {
  let best: { medic: MedicInfo; distM: number; bearingRad: number } | null = null;
  for (const m of medics) {
    if (!m.alive || m.downed) continue;
    const d = Math.hypot(m.x - px, m.z - pz);
    if (!best || d < best.distM) best = { medic: m, distM: d, bearingRad: Math.atan2(m.x - px, m.z - pz) };
  }
  return best;
}

/* ---- killer info display rules ---- */
export interface KillerInfo {
  name?: string;
  weapon: number;
  distM: number;
  isBot: boolean;
  friendly: boolean;
  hidden: boolean;
}
export interface KillerView {
  name?: string; // absent when hidden
  weapon: number;
  distM: number;
  tagKeys: string[];
}
/** `null` while downed (no killer node at all), details only once dead. `hideNames` is the player's setting. */
export function killerView(phase: DownedPhase, info: KillerInfo | undefined, hideNames: boolean): KillerView | null {
  if (phase !== 'dead' && phase !== 'spectating') return null;
  if (!info) return null;
  const tagKeys: string[] = [];
  if (info.isBot) tagKeys.push('ui.downed.tag.ai');
  if (info.friendly) tagKeys.push('ui.downed.tag.friendlyFire');
  const hide = hideNames || info.hidden;
  return { ...(hide ? {} : { name: info.name }), weapon: info.weapon, distM: Math.round(info.distM), tagKeys };
}

/* ---- spectate: cycle through teammates present in AOI ---- */
export function nextSpectateTarget(current: number | undefined, candidates: readonly number[], dir: 1 | -1 = 1): number | undefined {
  if (candidates.length === 0) return undefined;
  const i = current === undefined ? -1 : candidates.indexOf(current);
  if (i < 0) return candidates[dir === 1 ? 0 : candidates.length - 1];
  return candidates[(i + dir + candidates.length) % candidates.length];
}
/** Spectating is only offered after death, never while downed. */
export const canSpectate = (p: DownedPhase): boolean => p === 'dead' || p === 'spectating';
