/** CI check: en/zh-Hans must have identical key sets and identical placeholder names per key. */
import type { Messages } from './index';

export interface I18nIssue {
  kind: 'missing' | 'extra' | 'placeholder';
  key: string;
  detail?: string;
}

/** Placeholder names (`{n}`, `{n, plural, …}` → n) plus nested arm placeholders, sorted & deduped. */
export function placeholders(msg: string): string[] {
  const names = new Set<string>();
  const re = /\{\s*([A-Za-z_][\w]*)\s*(?=[,}])/g;
  for (const m of msg.matchAll(re)) names.add(m[1]!);
  for (const w of ['one', 'other', 'few', 'many', 'zero', 'plural']) names.delete(w);
  return [...names].sort();
}

export function checkCatalogs(base: Messages, other: Messages): I18nIssue[] {
  const issues: I18nIssue[] = [];
  for (const k of Object.keys(base)) {
    if (!(k in other)) issues.push({ kind: 'missing', key: k });
    else {
      const a = placeholders(base[k]!).join(',');
      const b = placeholders(other[k]!).join(',');
      if (a !== b) issues.push({ kind: 'placeholder', key: k, detail: `${a} vs ${b}` });
    }
  }
  for (const k of Object.keys(other)) if (!(k in base)) issues.push({ kind: 'extra', key: k });
  return issues;
}
