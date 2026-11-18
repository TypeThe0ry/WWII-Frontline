import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { ExitCode, GateError } from './errors.ts';
import { isBlank, type LedgerRow } from './ledger.ts';

/**
 * D-LIC channels (ARCHITECTURE §11.7, LICENSES §1 rule 5): paid / "use with caution" channels that need a written
 * confirmation from the seller before an asset may be processed.
 */
const DLIC_CHANNELS: readonly { name: string; re: RegExp }[] = [
  { name: 'TurboSquid', re: /turbo\s*squid/i },
  { name: 'CGTrader', re: /cg\s*trader/i },
  { name: 'Sketchfab (non-CC)', re: /sketchfab/i },
  { name: 'KitBash3D', re: /kit\s*bash\s*3d/i },
  { name: 'ActorCore', re: /actor\s*core/i },
  { name: 'A Sound Effect', re: /a\s*sound\s*effect/i },
];

/**
 * Which D-LIC channel (if any) applies to a row. Exemptions:
 *  - Sketchfab content under a Creative Commons licence (the rule says "non-CC");
 *  - free downloads (`order_id` starts with FREE, LICENSES §2/§6.2: simplified registration, no seller confirmation).
 * The second exemption is a reading of LICENSES §6 over §1 rule 5; see PI-032.
 */
export function dlicChannel(row: LedgerRow): string | undefined {
  if (/^free\b/i.test(row.order_id.trim())) return undefined;
  const hit = DLIC_CHANNELS.find((c) => c.re.test(row.source_channel));
  if (!hit) return undefined;
  if (hit.re.source.includes('sketchfab') && /^cc[\s0-9-]/i.test(row.license_name_version.trim())) return undefined;
  return hit.name;
}

/** Exit 3: the ledger row must be OK. FLAGGED counts as PENDING (LICENSES §6.2). */
export function assertStatusOk(row: LedgerRow, file?: string): void {
  if (row.status === 'OK') return;
  const where = file ? ` (${file})` : '';
  const hints: Record<string, string> = {
    PENDING: 'status PENDING: licence terms unconfirmed; keep the file out of the build until the ledger row is OK',
    FLAGGED: 'status FLAGGED: waiting for the owner to accept or reject it (LICENSES §6.2); treated like PENDING',
    INTERNAL: 'status INTERNAL (internal-only): excluded from ingest and public builds',
    REJECTED: 'status REJECTED: the asset is excluded; delete the file from assets/incoming/ (LICENSES §1 rule 2)',
  };
  throw new GateError(ExitCode.NotOk, `${row.asset_id}${where}: ${hints[row.status] ?? row.status} [LICENSES.md line ${row.line}]`, {
    file,
    assetId: row.asset_id,
  });
}

/** Exit 4: D-LIC channel rows need `confirmation_path`. */
export function assertConfirmation(row: LedgerRow, file?: string): void {
  const ch = dlicChannel(row);
  if (ch && isBlank(row.confirmation_path)) {
    throw new GateError(
      ExitCode.ConfirmationMissing,
      `${row.asset_id}${file ? ` (${file})` : ''}: D-LIC channel ${ch} requires a written seller confirmation but confirmation_path is empty [LICENSES.md line ${row.line}]`,
      { file, assetId: row.asset_id },
    );
  }
}

/** Exit 5: archived evidence files named by the row must exist (LICENSES §4.4). Paths are relative to `repoRoot`. */
export function assertArchives(row: LedgerRow, repoRoot: string, file?: string): void {
  for (const col of ['license_text_path', 'confirmation_path'] as const) {
    const v = row[col];
    if (isBlank(v)) continue;
    // Only the first token is a path; trailing prose ("（未入库…）") is ignored.
    const p = v.trim().split(/[（(\s]/)[0] as string;
    if (p === '' || p.startsWith('—')) continue;
    if (!existsSync(resolve(repoRoot, p))) {
      throw new GateError(
        ExitCode.ArchiveMissing,
        `${row.asset_id}${file ? ` (${file})` : ''}: ${col} "${p}" does not exist [LICENSES.md line ${row.line}]`,
        { file, assetId: row.asset_id },
      );
    }
  }
}

/** Repository root implied by a ledger path (the directory holding LICENSES.md). */
export function repoRootOf(ledgerPath: string): string {
  return dirname(resolve(ledgerPath));
}

/** All per-asset checks in gate order: status (3) -> D-LIC confirmation (4) -> archive files (5). */
export function checkRow(row: LedgerRow, repoRoot: string, file?: string): void {
  assertStatusOk(row, file);
  assertConfirmation(row, file);
  assertArchives(row, repoRoot, file);
}
