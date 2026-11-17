#!/usr/bin/env node
// Checks the workspace crate dependency graph against ARCHITECTURE §4.5 (matrix lives in
// dep-matrix.mjs) and forbids tokio/wasm-bindgen/wtransport/rayon below sim and protocol.
// Usage: node tools/ci/check-deps.mjs [--metadata <cargo-metadata.json>]
// Exit code: 0 = clean, 1 = violations (or unreadable input).
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { ALLOWED, CRATES, PACKAGE_PREFIX, TRANSITIVE_FORBIDDEN } from './dep-matrix.mjs';

const kindName = (k) => (k === 'dev' ? 'dev-dependencies' : k === 'build' ? 'build-dependencies' : 'dependencies');

// Best-effort "path:line" for a dependency declaration in a manifest.
function locate(manifestPath, depName) {
  if (!manifestPath || !existsSync(manifestPath)) return manifestPath ?? '?';
  const lines = readFileSync(manifestPath, 'utf8').split('\n');
  const re = new RegExp(`^\\s*"?${depName.replace(/[-]/g, '[-_]')}"?\\s*(=|\\.)`);
  const i = lines.findIndex((l) => re.test(l));
  return i >= 0 ? `${manifestPath}:${i + 1}` : manifestPath;
}

const short = (name) => (name.startsWith(PACKAGE_PREFIX) ? name.slice(PACKAGE_PREFIX.length) : name);

export function checkMetadata(meta) {
  const violations = [];
  const pkgById = new Map(meta.packages.map((p) => [p.id, p]));
  const members = new Set(meta.workspace_members);
  const ws = new Map(); // short name -> package
  for (const id of members) {
    const p = pkgById.get(id);
    if (p) ws.set(short(p.name), p);
  }

  // 1. direct workspace edges (normal + dev + build) against the matrix.
  for (const [from, pkg] of ws) {
    for (const dep of pkg.dependencies) {
      if (!dep.name.startsWith(PACKAGE_PREFIX)) continue;
      const to = short(dep.name);
      if (!CRATES.includes(from)) {
        violations.push(`${from} -> ${to} (${kindName(dep.kind)}): crate "${from}" is not in the §4.5 matrix [${locate(pkg.manifest_path, dep.name)}]`);
        continue;
      }
      if (!ALLOWED[from].includes(to)) {
        violations.push(`${from} -> ${to} (${kindName(dep.kind)}): edge forbidden by ARCHITECTURE §4.5 [${locate(pkg.manifest_path, dep.name)}]`);
      }
    }
  }
  // Workspace members missing from the matrix.
  for (const name of ws.keys()) {
    if (!CRATES.includes(name)) violations.push(`${name}: workspace crate not in the §4.5 matrix (add it to tools/ci/dep-matrix.mjs and ARCHITECTURE)`);
  }

  // 2. forbidden packages, direct (any kind) and transitive (normal + build).
  const nodes = new Map((meta.resolve?.nodes ?? []).map((n) => [n.id, n]));
  for (const [from, banned] of Object.entries(TRANSITIVE_FORBIDDEN)) {
    const pkg = ws.get(from);
    if (!pkg) continue;
    for (const dep of pkg.dependencies) {
      if (banned.includes(dep.name)) {
        violations.push(`${from} -> ${dep.name} (${kindName(dep.kind)}): forbidden dependency [${locate(pkg.manifest_path, dep.name)}]`);
      }
    }
    // BFS over the resolved graph, remembering the path.
    const seen = new Set([pkg.id]);
    const queue = [[pkg.id, [from]]];
    const reported = new Set();
    while (queue.length) {
      const [id, path] = queue.shift();
      const node = nodes.get(id);
      if (!node) continue;
      for (const d of node.deps ?? []) {
        const kinds = (d.dep_kinds ?? [{ kind: null }]).map((k) => k.kind);
        // dev-dependencies only count for the root crate itself (handled above by name).
        if (id !== pkg.id && kinds.every((k) => k === 'dev')) continue;
        if (id === pkg.id && kinds.every((k) => k === 'dev')) continue;
        const target = pkgById.get(d.pkg);
        if (!target || seen.has(d.pkg)) continue;
        seen.add(d.pkg);
        const next = [...path, short(target.name)];
        if (banned.includes(target.name) && id !== pkg.id && !reported.has(target.name)) {
          reported.add(target.name);
          violations.push(`${from} -> ${target.name} (transitive via ${next.join(' -> ')}): forbidden dependency`);
        }
        queue.push([d.pkg, next]);
      }
    }
  }
  return violations;
}

export function loadMetadata(file) {
  if (file) return JSON.parse(readFileSync(file, 'utf8'));
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
  const out = execFileSync('cargo', ['metadata', '--format-version', '1', '--locked'], {
    cwd: root,
    maxBuffer: 256 * 1024 * 1024,
    encoding: 'utf8',
  });
  return JSON.parse(out);
}

function main(argv) {
  const i = argv.indexOf('--metadata');
  const file = i >= 0 ? argv[i + 1] : undefined;
  if (i >= 0 && !file) {
    console.error('check-deps: --metadata needs a file argument');
    return 1;
  }
  let meta;
  try {
    meta = loadMetadata(file);
  } catch (e) {
    console.error(`check-deps: cannot load cargo metadata: ${e.message}`);
    return 1;
  }
  const violations = checkMetadata(meta);
  if (violations.length) {
    console.error(`check-deps: ${violations.length} violation(s) of ARCHITECTURE §4.5:`);
    for (const v of violations) console.error(`  ✗ ${v}`);
    return 1;
  }
  console.log('check-deps: OK (workspace dependency graph matches ARCHITECTURE §4.5)');
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exit(main(process.argv.slice(2)));
}
