/**
 * Bundle size budgets (ARCHITECTURE 11.4, M1-005): per bundle the sum of its assets' entry bytes + pack index overhead against
 * `budgets.json`; more than 1 % over budget fails, growth above 20 % versus the base manifest needs the `budget-approved` label.
 */
import { readFileSync } from 'node:fs';
import { assetRefs, matchesAny, type Manifest } from './manifest.ts';

export type Budgets = Record<string, number>;

export interface BundleUsage {
  bundle: string;
  bytes: number;
  budget: number;
  ratio: number;
  /** Bytes in the base manifest (undefined when there is none or the bundle is new). */
  base?: number;
}

export interface BudgetResult {
  ok: boolean;
  usages: BundleUsage[];
  failures: string[];
  /** Bundles with a budget but no measurement (e.g. `core` without a size report). */
  skipped: string[];
}

/** Bundles sized by the client build (br size of JS+WASM+UI), never by the manifest. */
export const EXTERNAL_BUNDLES: readonly string[] = ['core'];
export const OVER_BUDGET_TOLERANCE = 0.01;
export const GROWTH_LIMIT = 0.2;

export function loadBudgets(path: string): Budgets {
  return JSON.parse(readFileSync(path, 'utf8')) as Budgets;
}

/** Bytes of one bundle: every asset whose id matches one of the globs, entries + index overhead. */
export function bundleBytes(m: Manifest, bundle: string): number {
  const globs = m.bundles[bundle] ?? [];
  const tier = /_(low|medium|high|ultra)$/.exec(bundle)?.[1];
  let n = 0;
  for (const [id, a] of Object.entries(m.assets)) {
    if (!matchesAny(id, globs)) continue;
    if (tier && !(a.tier as string[]).includes(tier)) continue; // `*_medium` counts only assets shipped on that tier
    for (const r of assetRefs(a)) n += r.bytes + r.index_bytes;
  }
  return n;
}

export interface CheckOptions {
  /** Measured bytes for bundles the manifest cannot size itself (`core`: br size of the client build). */
  external?: Record<string, number>;
  /** PR carries the `budget-approved` label. */
  approved?: boolean;
}

export function checkBudgets(manifest: Manifest, budgets: Budgets, base?: Manifest, opts: CheckOptions = {}): BudgetResult {
  const usages: BundleUsage[] = [];
  const failures: string[] = [];
  const skipped: string[] = [];
  for (const key of Object.keys(budgets).sort()) {
    if (!key.endsWith('_bytes')) continue;
    const bundle = key.slice(0, -'_bytes'.length);
    const budget = budgets[key] as number;
    const external = opts.external?.[bundle];
    if (external === undefined && (EXTERNAL_BUNDLES.includes(bundle) || !(bundle in manifest.bundles))) {
      skipped.push(bundle);
      continue;
    }
    const bytes = external ?? bundleBytes(manifest, bundle);
    const baseBytes = base && bundle in base.bundles && external === undefined ? bundleBytes(base, bundle) : undefined;
    const u: BundleUsage = { bundle, bytes, budget, ratio: bytes / budget, ...(baseBytes !== undefined ? { base: baseBytes } : {}) };
    usages.push(u);
    if (bytes > budget * (1 + OVER_BUDGET_TOLERANCE)) {
      failures.push(`${bundle}: ${bytes} B is ${((u.ratio - 1) * 100).toFixed(2)} % over the ${budget} B budget (limit +${OVER_BUDGET_TOLERANCE * 100} %)`);
    }
    if (baseBytes !== undefined && baseBytes > 0 && bytes - baseBytes > baseBytes * GROWTH_LIMIT && !opts.approved) {
      failures.push(`${bundle}: grew ${(((bytes - baseBytes) / baseBytes) * 100).toFixed(1)} % over the base (${baseBytes} -> ${bytes} B); needs the budget-approved label`);
    }
  }
  return { ok: failures.length === 0, usages, failures, skipped };
}

export function formatUsages(r: BudgetResult): string {
  const lines = ['bundle                          bytes        budget      used'];
  for (const u of r.usages) lines.push(`${u.bundle.padEnd(30)} ${String(u.bytes).padStart(11)} ${String(u.budget).padStart(12)} ${(u.ratio * 100).toFixed(1).padStart(7)} %`);
  for (const s of r.skipped) lines.push(`${s.padEnd(30)} skipped (no measurement)`);
  return lines.join('\n');
}

export interface CategoryUsage {
  category: string;
  assets: number;
  entries: number;
  bytes: number;
  fpBytes: number;
  tpBytes: number;
}

/** Per-category byte summary (M1-005 / ARCHITECTURE 11.4 & 11.8: helps M1-064 track dual-precision weapon/character budgets). */
export function categoryBreakdown(m: Manifest, tier?: string): CategoryUsage[] {
  const byCat = new Map<string, CategoryUsage>();
  for (const [id, a] of Object.entries(m.assets)) {
    if (tier && !(a.tier as string[]).includes(tier)) continue;
    const category = id.split('_')[0] || 'other';
    const row = byCat.get(category) ?? { category, assets: 0, entries: 0, bytes: 0, fpBytes: 0, tpBytes: 0 };
    row.assets++;
    for (const r of assetRefs(a)) {
      const sz = r.bytes + r.index_bytes;
      row.entries++;
      row.bytes += sz;
      if (r.entry.endsWith('_fp')) row.fpBytes += sz;
      else if (r.entry.endsWith('_tp')) row.tpBytes += sz;
    }
    byCat.set(category, row);
  }
  return [...byCat.values()].sort((a, b) => a.category.localeCompare(b.category));
}

export function formatCategoryBreakdown(rows: readonly CategoryUsage[]): string {
  if (rows.length === 0) return '';
  const lines = ['category    assets  entries        bytes     _fp bytes     _tp bytes'];
  for (const r of rows) {
    lines.push(
      `${r.category.padEnd(10)} ${String(r.assets).padStart(7)} ${String(r.entries).padStart(8)} ${String(r.bytes).padStart(12)} ${String(r.fpBytes).padStart(13)} ${String(r.tpBytes).padStart(13)}`,
    );
  }
  return lines.join('\n');
}
