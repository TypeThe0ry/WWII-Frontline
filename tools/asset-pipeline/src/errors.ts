/** Process exit codes of the asset-pipeline CLI (M1-001). 2/3/4 are fixed by the task card. */
export const ExitCode = {
  Ok: 0,
  /** Usage error or unexpected internal failure. */
  Usage: 1,
  /** A file under assets/incoming/ matches no ledger row (LICENSES §1 rule 1). */
  Unregistered: 2,
  /** The matching ledger row is not OK (PENDING / FLAGGED / REJECTED). */
  NotOk: 3,
  /** D-LIC channel without a confirmation_path (ARCHITECTURE §11.7). */
  ConfirmationMissing: 4,
  /** license_text_path / confirmation_path points at a file that does not exist (LICENSES §4.4). */
  ArchiveMissing: 5,
  /** LICENSES.md section 3 cannot be parsed (missing column, duplicate id, bad status ...). */
  LedgerInvalid: 6,
  /** Output logical ID violates ARCHITECTURE §11.2. */
  Naming: 7,
  /** Requested processing is outside allowed_processing (D-MOD). */
  ProcessingNotAllowed: 8,
} as const;
export type ExitCodeValue = (typeof ExitCode)[keyof typeof ExitCode];

/** A gate refusal: carries the CLI exit code and the offending file / asset. */
export class GateError extends Error {
  readonly code: ExitCodeValue;
  readonly file: string | undefined;
  readonly assetId: string | undefined;
  constructor(code: ExitCodeValue, message: string, opts: { file?: string; assetId?: string } = {}) {
    super(message);
    this.name = 'GateError';
    this.code = code;
    this.file = opts.file;
    this.assetId = opts.assetId;
  }
}
