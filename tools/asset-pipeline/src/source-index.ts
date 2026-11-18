import { readFileSync } from 'node:fs';
import { ExitCode, GateError } from './errors.ts';
import { checkRow, repoRootOf } from './gate.ts';
import { INCOMING_PREFIX } from './ingest.ts';
import { isBlank, lookupByRawPath, type Ledger } from './ledger.ts';

interface IndexedFile {
  path: string;
  req_ids: string[];
  license: string;
  source_url: string;
  sha256: string;
}

export interface SourceInventory {
  files: number;
  candidates: ReadonlyMap<string, ReadonlySet<string>>;
  coverage: ReadonlyMap<string, 'real' | 'placeholder'>;
}

/** Validate INDEX.json records against exact ledger rows. No source bytes or build files are read or written. */
export function checkSourceIndex(path: string, ledger: Ledger, root = repoRootOf(ledger.path)): SourceInventory {
  function fail(detail: string): never {
    throw new GateError(ExitCode.Usage, `${path}: ${detail}`, { file: path });
  }
  let index: { files?: IndexedFile[]; coverage?: Record<string, unknown> } | null;
  try {
    index = JSON.parse(readFileSync(path, 'utf8')) as typeof index;
  } catch (e) {
    fail(`cannot read source index (${(e as Error).message})`);
  }
  if (!Array.isArray(index?.files)) fail('expected a files array');
  const candidates = new Map<string, Set<string>>();
  const paths = new Set<string>();
  for (const file of index.files) {
    if (typeof file?.path !== 'string' || !file.path.startsWith(`${INCOMING_PREFIX}/`)
      || file.path.includes('\\')
      || file.path.split('/').some((segment) => !segment || segment === '.' || segment === '..')) {
      fail('invalid incoming file path');
    }
    if (paths.has(file.path)) fail(`duplicate file ${file.path}`);
    paths.add(file.path);
    const row = lookupByRawPath(ledger, file.path);
    if (!row) throw new GateError(ExitCode.Unregistered, `${file.path}: no matching ledger row`, { file: file.path });
    checkRow(row, root, file.path);
    if (isBlank(row.license_text_path)) {
      throw new GateError(ExitCode.ArchiveMissing, `${row.asset_id}: no archived licence text`, { file: file.path, assetId: row.asset_id });
    }
    if (row.raw_path !== file.path
      || !/^(?:CC0 1\.0|CC(?:-| )BY (?:1\.0|2\.0|2\.5|3\.0|4\.0)|SIL OFL 1\.1)$/.test(file.license)
      || file.license !== row.license_name_version
      || typeof file.source_url !== 'string' || !/^https?:\/\//.test(file.source_url)
      || file.source_url !== row.fields.url
      || typeof file.sha256 !== 'string' || !/^[0-9a-f]{64}$/.test(file.sha256)
      || row.fields.notes?.match(/(?:^|\s)SHA-256=([0-9a-f]{64})(?=\s|$)/)?.[1] !== file.sha256
      || !Array.isArray(file.req_ids) || !file.req_ids.length
      || !file.req_ids.every((id) => typeof id === 'string' && /^[A-Z]-[A-Z0-9-]+$/.test(id))
      || (!isBlank(row.req_id) && !file.req_ids.includes(row.req_id))) {
      fail(`${file.path}: invalid or mismatched licence, source, hash, or requirements for ${row.asset_id}`);
    }
    for (const id of file.req_ids) {
      const assets = candidates.get(id) ?? new Set<string>();
      assets.add(row.asset_id);
      candidates.set(id, assets);
    }
  }
  const coverage = new Map<string, 'real' | 'placeholder'>();
  if (index.coverage !== undefined) {
    if (!index.coverage || typeof index.coverage !== 'object' || Array.isArray(index.coverage)) fail('invalid coverage map');
    for (const [id, status] of Object.entries(index.coverage)) {
      if ((status !== 'real' && status !== 'placeholder') || !candidates.has(id)) fail(`${id}: coverage must refer to an indexed candidate`);
      coverage.set(id, status);
    }
  }
  return { files: paths.size, candidates, coverage };
}
