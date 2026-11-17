import { describe, expect, it } from 'vitest';
import { assertAllowed, ExitCode, GateError, parseLedger, setDefaultLedger } from '../src/index.ts';
import { ledgerText } from './helpers.ts';

const l = parseLedger(
  ledgerText([
    { asset_id: 'AST-V-A-01', raw_path: 'assets/incoming/a', allowed_processing: 'CONVERT,LOD' },
    { asset_id: 'AST-V-B-01', raw_path: 'assets/incoming/b', allowed_processing: '—' },
    { asset_id: 'AST-V-C-01', raw_path: 'assets/incoming/c', allowed_processing: 'CONVERT', status: 'PENDING' },
    { asset_id: 'AST-A-D-01', raw_path: 'assets/incoming/d', allowed_processing: 'TRANSCODE,LOUDNORM', type: 'sfx' },
  ]),
);

function code(fn: () => void): number {
  try {
    fn();
  } catch (e) {
    return (e as GateError).code;
  }
  return 0;
}

describe('assertAllowed (D-MOD)', () => {
  it('allows listed actions and refuses the others with exit 8', () => {
    expect(code(() => assertAllowed('AST-V-A-01', 'CONVERT', l))).toBe(0);
    expect(code(() => assertAllowed('AST-V-A-01', 'LOD', l))).toBe(0);
    expect(code(() => assertAllowed('AST-V-A-01', 'REBIND', l))).toBe(ExitCode.ProcessingNotAllowed);
    expect(code(() => assertAllowed('AST-A-D-01', 'EDIT', l))).toBe(ExitCode.ProcessingNotAllowed);
    expect(code(() => assertAllowed('AST-A-D-01', 'TRANSCODE', l))).toBe(0);
  });
  it('refuses unknown codes, forbidden D-MOD actions, unknown assets and non-OK rows', () => {
    expect(code(() => assertAllowed('AST-V-A-01', 'SCULPT', l))).toBe(ExitCode.ProcessingNotAllowed);
    expect(code(() => assertAllowed('AST-V-NOPE-01', 'CONVERT', l))).toBe(ExitCode.Unregistered);
    expect(code(() => assertAllowed('AST-V-C-01', 'CONVERT', l))).toBe(ExitCode.NotOk);
  });
  it('an empty allowed_processing column permits every D-MOD code (LICENSES §6.5)', () => {
    expect(code(() => assertAllowed('AST-V-B-01', 'RETARGET', l))).toBe(0);
    expect(code(() => assertAllowed('AST-V-B-01', 'SCULPT', l))).toBe(ExitCode.ProcessingNotAllowed);
  });
  it('uses the default ledger set by ingest/CLI when none is passed', () => {
    setDefaultLedger(l);
    expect(code(() => assertAllowed('AST-V-A-01', 'REBIND'))).toBe(ExitCode.ProcessingNotAllowed);
    setDefaultLedger(undefined);
    expect(code(() => assertAllowed('AST-V-A-01', 'CONVERT'))).toBe(ExitCode.Usage);
  });
});
