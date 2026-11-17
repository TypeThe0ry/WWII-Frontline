import { rmSync } from 'node:fs';
import { REPO_ROOT } from '../net-support';

/** A run starts from an empty matrix (results are merged into the file test by test). */
export default function globalSetup(): void {
  rmSync(`${REPO_ROOT}/reports/m0/browser-matrix.json`, { force: true });
  rmSync(`${REPO_ROOT}/reports/m0/browser-matrix.md`, { force: true });
}
