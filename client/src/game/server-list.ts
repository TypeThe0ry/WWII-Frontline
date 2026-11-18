/** Server browser logic (UI_SPEC §4.3): filtering, sorting, ping rating, reject mapping. */
export interface ServerInfo {
  id: string;
  name: string;
  map: string;
  mode: string;
  humans: number;
  bots: number;
  cap: number;
  phase: string;
  pingMs?: number;
  region: string;
  version: number;
  locked: boolean;
  compatible: boolean;
}

export interface ServerFilters {
  map?: string;
  mode?: string;
  region?: string;
  maxPingMs?: number;
  hideFull?: boolean;
  hideEmpty?: boolean;
  noBots?: boolean;
  noPassword?: boolean;
  compatibleOnly?: boolean;
  keyword?: string;
}

export type SortKey = 'recommended' | 'ping' | 'players' | 'name';

export function applyFilters(list: readonly ServerInfo[], f: ServerFilters): ServerInfo[] {
  const kw = f.keyword?.trim().toLowerCase();
  return list.filter((s) => {
    if (f.map && s.map !== f.map) return false;
    if (f.mode && s.mode !== f.mode) return false;
    if (f.region && s.region !== f.region) return false;
    if (f.maxPingMs !== undefined && (s.pingMs === undefined || s.pingMs > f.maxPingMs)) return false;
    if (f.hideFull && s.humans + s.bots >= s.cap) return false;
    if (f.hideEmpty && s.humans === 0) return false;
    if (f.noBots && s.bots > 0) return false;
    if (f.noPassword && s.locked) return false;
    if (f.compatibleOnly && !s.compatible) return false;
    if (kw && !s.name.toLowerCase().includes(kw)) return false;
    return true;
  });
}

export type PingRating = 'good' | 'fair' | 'poor' | 'unknown';
export function pingRating(ms: number | undefined): PingRating {
  if (ms === undefined) return 'unknown';
  return ms <= 60 ? 'good' : ms <= 120 ? 'fair' : 'poor';
}

/** "Recommended": compatible first, then joinable, then good ping, then fuller servers (more humans) but not full. */
function recScore(s: ServerInfo): number {
  const total = s.humans + s.bots;
  const joinable = total < s.cap ? 1 : 0;
  const ping = { good: 2, fair: 1, poor: 0, unknown: 0 }[pingRating(s.pingMs)];
  return (s.compatible ? 1000 : 0) + joinable * 100 + ping * 10 + Math.min(9, Math.floor((9 * s.humans) / Math.max(1, s.cap)));
}

export function sortServers(list: readonly ServerInfo[], key: SortKey): ServerInfo[] {
  const out = [...list];
  const byName = (a: ServerInfo, b: ServerInfo) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id);
  switch (key) {
    case 'ping':
      return out.sort((a, b) => (a.pingMs ?? Infinity) - (b.pingMs ?? Infinity) || byName(a, b));
    case 'players':
      return out.sort((a, b) => b.humans - a.humans || byName(a, b));
    case 'name':
      return out.sort(byName);
    default:
      return out.sort((a, b) => recScore(b) - recScore(a) || byName(a, b));
  }
}

/** `72+18/100` style occupancy string. */
export const occupancy = (s: ServerInfo): string => `${s.humans}+${s.bots}/${s.cap}`;

/** Join button state: incompatible → disabled with reason key. */
export function joinState(s: ServerInfo): { enabled: boolean; reasonKey?: string } {
  if (!s.compatible) return { enabled: false, reasonKey: 'ui.browser.reason.incompatible' };
  if (s.humans + s.bots >= s.cap && s.humans >= s.cap) return { enabled: false, reasonKey: 'ui.browser.reason.full' };
  return { enabled: true };
}

/** Reject reason (ARCHITECTURE §6.8.1 order: version / full / banned / ticket invalid / data hash) → i18n key. */
export const REJECT_KEYS: readonly string[] = ['ui.reject.version', 'ui.reject.full', 'ui.reject.banned', 'ui.reject.ticket', 'ui.reject.dataHash'];
export const rejectKey = (code: number): string => REJECT_KEYS[code] ?? 'ui.reject.unknown';
