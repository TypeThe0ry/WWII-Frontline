import type { ServerFilters, SortKey } from '../../game/server-list';

const KEY = 'frontline.browser.v1';
export const sortOptions = ['recommended', 'players', 'name'] as const;
export const boolFilters = ['hideFull', 'hideEmpty', 'noBots', 'compatibleOnly'] as const;
export interface Preferences { filters: ServerFilters; sort: SortKey }

export function readPreferences(): Preferences {
  const fallback: Preferences = { filters: {}, sort: 'recommended' };
  try {
    const parsed = JSON.parse(localStorage.getItem(KEY) ?? 'null') as Partial<Preferences> | null;
    if (!parsed || typeof parsed !== 'object') return fallback;
    const sort = sortOptions.find((key) => key === parsed.sort) ?? fallback.sort;
    const filters: ServerFilters = {};
    for (const key of boolFilters) if (parsed.filters?.[key] === true) filters[key] = true;
    for (const key of ['map', 'mode', 'region', 'keyword'] as const) {
      const value = parsed.filters?.[key];
      if (typeof value === 'string') filters[key] = value.slice(0, 200);
    }
    return { filters, sort };
  } catch { return fallback; }
}

export function writePreferences(value: Preferences): void {
  try { localStorage.setItem(KEY, JSON.stringify(value)); } catch { /* browser privacy/storage policy */ }
}
