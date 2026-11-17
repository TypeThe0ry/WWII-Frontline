#!/usr/bin/env node
// Source-level determinism scan for crates/sim (ARCHITECTURE §4.3.2).
// Regex based: it misses macro expansions and aliased imports, so it supplements review.
// Waive a line with a trailing `// purity-allow: <reason>`; waivers are listed in the report.
// Usage: node tools/ci/check-sim-purity.mjs [--dir <src-dir>]
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const BANNED = /\bHashMap\b|\bHashSet\b|\bInstant\b|\bSystemTime\b|\bthread_rng\b|\brayon\b|\bwasm_bindgen\b|\btokio\b/g;
const ALLOW = /\/\/\s*purity-allow:\s*(\S.*)$/;

function* rustFiles(dir) {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* rustFiles(p);
    else if (name.endsWith('.rs')) yield p;
  }
}

// Strips // comments and /* */ comments (tracking multi-line state); keeps the raw line for the waiver.
export function scanSource(text) {
  const violations = [];
  const waived = [];
  let inBlock = false;
  text.split('\n').forEach((raw, idx) => {
    const lineNo = idx + 1;
    let code = '';
    let rest = raw;
    while (rest.length) {
      if (inBlock) {
        const end = rest.indexOf('*/');
        if (end < 0) { rest = ''; break; }
        inBlock = false;
        rest = rest.slice(end + 2);
      } else {
        const lc = rest.indexOf('//');
        const bc = rest.indexOf('/*');
        if (bc >= 0 && (lc < 0 || bc < lc)) {
          code += rest.slice(0, bc);
          inBlock = true;
          rest = rest.slice(bc + 2);
        } else {
          code += lc >= 0 ? rest.slice(0, lc) : rest;
          rest = '';
        }
      }
    }
    const hits = [...code.matchAll(BANNED)].map((m) => m[0]);
    if (!hits.length) return;
    const allow = ALLOW.exec(raw);
    if (allow) waived.push({ line: lineNo, hits, reason: allow[1].trim(), text: raw.trim() });
    else violations.push({ line: lineNo, hits, text: raw.trim() });
  });
  return { violations, waived };
}

export function scanDir(dir) {
  const result = { violations: [], waived: [] };
  for (const file of rustFiles(dir)) {
    const r = scanSource(readFileSync(file, 'utf8'));
    for (const v of r.violations) result.violations.push({ file, ...v });
    for (const w of r.waived) result.waived.push({ file, ...w });
  }
  return result;
}

function main(argv) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
  const i = argv.indexOf('--dir');
  const dir = i >= 0 ? resolve(argv[i + 1]) : join(root, 'crates', 'sim', 'src');
  let r;
  try {
    r = scanDir(dir);
  } catch (e) {
    console.error(`check-sim-purity: cannot scan ${dir}: ${e.message}`);
    return 1;
  }
  const rel = (f) => relative(process.cwd(), f) || f;
  for (const w of r.waived) console.log(`purity-allow ${rel(w.file)}:${w.line}: ${w.hits.join(', ')} -- ${w.reason}`);
  if (r.violations.length) {
    console.error(`check-sim-purity: ${r.violations.length} violation(s) (ARCHITECTURE §4.3.2):`);
    for (const v of r.violations) console.error(`  ✗ ${rel(v.file)}:${v.line}: ${v.hits.join(', ')}  | ${v.text}`);
    return 1;
  }
  console.log(`check-sim-purity: OK (${r.waived.length} waived) in ${rel(dir)}`);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exit(main(process.argv.slice(2)));
}
