/** Font loading policy (UI_SPEC §2.3): self-hosted subsets, `font-display: swap`, system fallback flag, CJK shards on demand. */
export type Lang = 'en' | 'zh-Hans';

export interface FontFile {
  file: string;
  bytes: number;
  br_bytes: number;
  shard: 'latin' | 'cjk0' | 'cjk1' | 'cjk2' | 'cjk3';
}

/** Shards needed for the first screen of a language: Latin always, plus the UI vocabulary shard for Chinese. */
export function firstScreenShards(lang: Lang): FontFile['shard'][] {
  return lang === 'zh-Hans' ? ['latin', 'cjk0'] : ['latin'];
}

export function firstScreenBytes(files: readonly FontFile[], lang: Lang, size: 'bytes' | 'br_bytes' = 'bytes'): number {
  const need = new Set(firstScreenShards(lang));
  return files.filter((f) => need.has(f.shard)).reduce((a, f) => a + f[size], 0);
}

/** Build-time transfer budgets use the report's measured per-file Brotli sizes. */
export const BUDGET = { zhFirstScreen: 400 * 1024, latin: 200 * 1024 } as const;

export function checkBudget(files: readonly FontFile[]): string[] {
  const out: string[] = [];
  if (files.filter((f) => f.shard === 'latin').length !== 4 || files.filter((f) => f.shard === 'cjk0').length !== 1 ||
      new Set(files.map((f) => f.file)).size !== files.length) out.push('incomplete or duplicate first-screen font set');
  if (files.some((f) => !Number.isSafeInteger(f.br_bytes) || f.br_bytes <= 0)) out.push('invalid Brotli size');
  const latin = firstScreenBytes(files, 'en', 'br_bytes');
  const zh = firstScreenBytes(files, 'zh-Hans', 'br_bytes');
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

/** Install once at the UI root when a full bundle is deployed. CSS selects shards from visible text. */
export function installUiFonts(doc: Document = document): () => void {
  const link = doc.createElement('link');
  link.rel = 'stylesheet';
  link.href = '/fonts/fonts.css';
  const fallback = () => applyFallbackFlag(doc.documentElement, true);
  const families = new Set(['Barlow Semi Condensed', 'Frontline UI Sans', 'Frontline UI Mono', 'Noto Sans SC']);
  const failed = (event: FontFaceSetLoadEvent) => {
    if (event.fontfaces.some((face) => families.has(face.family.replace(/^["']|["']$/g, '')))) fallback();
  };
  link.addEventListener('error', fallback);
  doc.fonts.addEventListener('loadingerror', failed);
  doc.head.append(link);
  return () => {
    link.removeEventListener('error', fallback);
    doc.fonts.removeEventListener('loadingerror', failed);
    link.remove();
    applyFallbackFlag(doc.documentElement, false);
  };
}
