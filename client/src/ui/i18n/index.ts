/**
 * i18n core (M1-053a): `t(key, params)`, ICU-style `{n, plural, one {..} other {..}}` and `{name}` interpolation,
 * runtime language switch without reload. Missing keys render `⟦key⟧` (and are collected for CI/dev overlay).
 */
import { signal } from '@preact/signals';

export type Locale = 'en' | 'zh-Hans';
export type Messages = Readonly<Record<string, string>>;
export type Params = Readonly<Record<string, string | number>>;

export const locale = signal<Locale>('en');
const catalogs = new Map<Locale, Messages>();
export const missingKeys = new Set<string>();

export function registerMessages(l: Locale, m: Messages): void {
  catalogs.set(l, m);
}

export function setLocale(l: Locale): void {
  locale.value = l;
}

/** Plural category per CLDR for the two shipped languages (zh has only `other`). */
export function pluralCategory(l: Locale, n: number): 'one' | 'other' {
  return l === 'en' && n === 1 ? 'one' : 'other';
}

/** Index of the `}` matching the `{` at `open`, or -1. */
function matchBrace(s: string, open: number): number {
  let depth = 0;
  for (let i = open; i < s.length; i++) {
    if (s[i] === '{') depth++;
    else if (s[i] === '}' && --depth === 0) return i;
  }
  return -1;
}

export function format(l: Locale, msg: string, params: Params = {}): string {
  let out = '';
  for (let i = 0; i < msg.length; i++) {
    if (msg[i] !== '{') {
      out += msg[i];
      continue;
    }
    const end = matchBrace(msg, i);
    if (end < 0) {
      out += msg.slice(i);
      break;
    }
    const body = msg.slice(i + 1, end);
    const comma = body.indexOf(',');
    if (comma < 0) {
      const v = params[body.trim()];
      out += v === undefined ? `{${body}}` : String(v);
    } else {
      const name = body.slice(0, comma).trim();
      const rest = body.slice(comma + 1).trim();
      if (rest.startsWith('plural,')) {
        const n = Number(params[name] ?? 0);
        const arms = new Map<string, string>();
        let p = rest.indexOf(',') + 1;
        while (p < rest.length) {
          const o = rest.indexOf('{', p);
          if (o < 0) break;
          const c = matchBrace(rest, o);
          if (c < 0) break;
          arms.set(rest.slice(p, o).trim(), rest.slice(o + 1, c));
          p = c + 1;
        }
        const arm = arms.get(`=${n}`) ?? arms.get(pluralCategory(l, n)) ?? arms.get('other') ?? '';
        out += format(l, arm.replace(/#/g, String(n)), params);
      } else out += `{${body}}`;
    }
    i = end;
  }
  return out;
}

export function t(key: string, params?: Params): string {
  const l = locale.value;
  const msg = catalogs.get(l)?.[key];
  if (msg === undefined) {
    missingKeys.add(key);
    return `⟦${key}⟧`;
  }
  return format(l, msg, params);
}
