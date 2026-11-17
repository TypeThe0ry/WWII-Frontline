import { ExitCode, GateError } from './errors.ts';
import { PROCESSING_CODES, type Ledger, type LedgerRow, type ProcessingCode } from './ledger.ts';
import { assertStatusOk } from './gate.ts';

export type { ProcessingCode };

let defaultLedger: Ledger | undefined;

/** Register the ledger used by `assertAllowed(assetId, action)` when no ledger is passed (set by ingest and the CLI). */
export function setDefaultLedger(ledger: Ledger | undefined): void {
  defaultLedger = ledger;
}

function isProcessingCode(v: string): v is ProcessingCode {
  return (PROCESSING_CODES as readonly string[]).includes(v);
}

/**
 * D-MOD guard: throws GateError(ProcessingNotAllowed) unless `action` is within the asset's `allowed_processing`
 * (an empty column permits every D-MOD code, LICENSES §6.5) and the ledger row is OK.
 * Every later pipeline step calls this before it touches an asset.
 */
export function assertAllowed(assetId: string, action: ProcessingCode | string, ledger: Ledger | readonly LedgerRow[] | undefined = defaultLedger): void {
  if (!ledger) throw new GateError(ExitCode.Usage, `assertAllowed(${assetId}, ${action}): no ledger loaded`, { assetId });
  const row = Array.isArray(ledger) ? (ledger as readonly LedgerRow[]).find((r) => r.asset_id === assetId) : (ledger as Ledger).byAssetId.get(assetId);
  if (!row) throw new GateError(ExitCode.Unregistered, `${assetId}: not registered in the ledger`, { assetId });
  assertStatusOk(row);
  if (!isProcessingCode(action)) {
    throw new GateError(ExitCode.ProcessingNotAllowed, `${assetId}: unknown processing code "${action}"`, { assetId });
  }
  if (row.allowed_processing !== null && !row.allowed_processing.includes(action)) {
    throw new GateError(
      ExitCode.ProcessingNotAllowed,
      `${assetId}: processing ${action} is not in allowed_processing (${row.allowed_processing.join(',') || 'empty'}) [LICENSES.md line ${row.line}]`,
      { assetId },
    );
  }
}
