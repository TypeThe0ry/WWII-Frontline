/** Font loading policy (UI_SPEC §2.3): self-hosted subsets, `font-display: swap`, system fallback flag, CJK shards on demand. */
export type Lang = 'en' | 'zh-Hans';

export interface FontFile {
  file: string;
  bytes: number;
  shard: 'latin' | 'cjk0' | 'cjk1' | 'cjk2' | 'cjk3';
}

/** Shards needed for the first screen of a language: Latin always, plus the UI vocabulary shard for Chinese. */
export function firstScreenShards(lang: Lang): FontFile['shard'][] {
  return lang === 'zh-Hans' ? ['latin', 'cjk0'] : ['latin'];
}

export function firstScreenBytes(files: readonly FontFile[], lang: Lang): number {
  const need = new Set(firstScreenShards(lang));
  return files.filter((f) => need.has(f.shard)).reduce((a, f) => a + f.bytes, 0);
}

/** Budgets (UI_SPEC appendix B-5): zh-Hans first screen ≤ 400 KB, Latin ≤ 200 KB (woff2 is already Brotli-compressed). */
export const BUDGET = { zhFirstScreen: 400 * 1024, latin: 200 * 1024 } as const;

export function checkBudget(files: readonly FontFile[]): string[] {
  const out: string[] = [];
  const latin = firstScreenBytes(files, 'en');
  const zh = firstScreenBytes(files, 'zh-Hans');
  if (latin > BUDGET.latin) out.push(`latin ${latin} B > ${BUDGET.latin} B`);
  if (zh > BUDGET.zhFirstScreen) out.push(`zh first screen ${zh} B > ${BUDGET.zhFirstScreen} B`);
  return out;
}

/** Characters not covered by the first-screen CJK shard → the other shards are fetched on demand (browser does this via unicode-range). */
export function needsMoreShards(text: string, firstScreenChars: ReadonlySet<number>): boolean {
  for (const ch of text) {
    const cp = ch.codePointAt(0)!;
    if (cp >= 0x2e80 && !firstScreenChars.has(cp)) return true;
  }
  return false;
}

/** Sets `data-font-fallback` on <html> when a face fails to load (adjusts line-height, UI_SPEC §2.3). */
export function applyFallbackFlag(root: { setAttribute(k: string, v: string): void; removeAttribute(k: string): void }, failed: boolean): void {
  if (failed) root.setAttribute('data-font-fallback', '');
  else root.removeAttribute('data-font-fallback');
}
