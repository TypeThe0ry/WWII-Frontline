// Core of the /data validator: file discovery, JSON Schema (ajv 2020-12) + custom rules.
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import { checkUnits } from './rules/units.mjs';
import { checkTunable } from './rules/tunable.mjs';
import { checkRanges } from './rules/ranges.mjs';
import { checkHistorical } from './rules/historical.mjs';
import { checkRefs } from './rules/refs.mjs';

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = resolve(here, '..', '..');
<<<<<<< HEAD
export const KINDS = ['weapon', 'explosive', 'melee', 'ammo', 'material', 'gadget', 'role', 'loadout', 'vehicle', 'economy', 'resource_node', 'map_meta', 'spawns', 'rules', 'rule_capture', 'rule_match', 'rule_spawn', 'rule_supplies', 'rule_squads', 'rule_aoi', 'rule_anticheat', 'rule_lagcomp', 'rule_server', 'bots', 'audio_params', 'comms', 'scoring'];
=======
export const KINDS = ['weapon', 'explosive', 'melee', 'ammo', 'material', 'gadget', 'role', 'loadout', 'vehicle', 'map_meta', 'spawns', 'rules', 'rule_capture', 'rule_match', 'rule_spawn', 'rule_supplies', 'rule_squads', 'rule_aoi', 'rule_ordnance', 'rule_anticheat', 'rule_lagcomp', 'rule_server', 'bots', 'audio_params', 'comms', 'scoring'];
>>>>>>> 9a2b458 (B6-mines: WIP (stopped by user, unmerged))
const DIR_KIND = { weapons: 'weapon', explosives: 'explosive', melee: 'melee', ammo: 'ammo', materials: 'material', gadgets: 'gadget', roles: 'role', loadouts: 'loadout', vehicles: 'vehicle' };
const RULE_FILE_KIND = { 'rules/capture.json': 'rule_capture', 'rules/match_m1.json': 'rule_match', 'rules/spawn.json': 'rule_spawn', 'rules/supplies.json': 'rule_supplies', 'rules/squads.json': 'rule_squads', 'rules/aoi.json': 'rule_aoi', 'rules/ordnance_m2.json': 'rule_ordnance', 'rules/anticheat.json': 'rule_anticheat', 'rules/lagcomp.json': 'rule_lagcomp', 'rules/bots.json': 'bots' };
const MAP_FILE_KIND = { 'meta.json': 'map_meta', 'spawns.json': 'spawns' };
const SLUG = /^[a-z][a-z0-9]*(_[a-z0-9]+)*$/;

const toDotted = (p) => p.split('/').filter(Boolean).join('.');

/** Compile the schemas found in `<root>/schemas`. */
export function loadValidators(root) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  const out = {};
  for (const kind of KINDS) {
    const schema = JSON.parse(readFileSync(join(root, 'schemas', `${kind}.schema.json`), 'utf8'));
    out[kind] = ajv.compile(schema);
  }
  return out;
}

/** Map one ajv error to our `{pointer, message}` form. */
export function ajvToError(e) {
  const p = e.instancePath;
  switch (e.keyword) {
    case 'required': {
      const base = toDotted(p);
      const prop = e.params.missingProperty;
      return { pointer: p, message: `missing: ${base ? `${base}.` : ''}${prop}` };
    }
    case 'additionalProperties':
      return { pointer: p, message: `unknown property: ${e.params.additionalProperty}` };
    case 'enum':
      return { pointer: p, message: `must be one of the allowed values: ${e.params.allowedValues.join(', ')}` };
    case 'const':
      return { pointer: p, message: `must be equal to constant ${JSON.stringify(e.params.allowedValue)}` };
    case 'pattern':
      return { pointer: p, message: `must match pattern ${e.params.pattern}` };
    default:
      return { pointer: p, message: e.message ?? e.keyword };
  }
}

/** Per-file checks (schema + rules). Returns `{pointer,message}[]`. */
export function checkFile(kind, doc, validators) {
  const errs = [];
  const validate = validators[kind];
  if (!validate(doc)) errs.push(...validate.errors.map(ajvToError));
  if (doc && typeof doc === 'object' && !Array.isArray(doc)) {
    errs.push(...checkUnits(doc, kind));
    errs.push(...checkTunable(doc, kind));
    errs.push(...checkRanges(doc, kind));
    errs.push(...checkHistorical(doc, kind));
  }
  return errs;
}

function walkJson(dir) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walkJson(p));
    else if (name.endsWith('.json')) out.push(p);
  }
  return out;
}

/** Discover checkable files under a data root. Others are counted as `unchecked`. */
export function discover(root) {
  const files = [];
  const unchecked = [];
  for (const abs of walkJson(root)) {
    const rel = relative(root, abs).split('\\').join('/');
    const parts = rel.split('/');
    if (parts[0] === 'schemas') continue;
    let kind = null;
    if (parts.length === 2 && DIR_KIND[parts[0]]) kind = DIR_KIND[parts[0]];
    else if (parts.length === 2 && rel.startsWith('structures/resource_node_')) kind = 'resource_node';
    else if (parts.length === 2 && parts[0] === 'rules') kind = RULE_FILE_KIND[rel] ?? 'rules';
    else if (rel === 'economy.json') kind = 'economy';
    else if (rel === 'server_rules.json') kind = 'rule_server';
    else if (rel === 'comms.json') kind = 'comms';
    else if (rel === 'audio/params.json') kind = 'audio_params';
    else if (rel === 'scoring.json') kind = 'scoring';
    else if (parts.length === 3 && parts[0] === 'maps' && MAP_FILE_KIND[parts[2]]) kind = MAP_FILE_KIND[parts[2]];
    if (kind) {
      const stemId = DIR_KIND[parts[0]] || kind === 'resource_node' ? basename(rel, '.json') : null;
      files.push({ kind, file: rel, abs, stemId });
    }
    else unchecked.push(rel);
  }
  return { files, unchecked };
}

/** Fixtures are flat files named `<kind>.<name>.json`. */
export function discoverFixtures(dir) {
  const files = [];
  for (const abs of walkJson(dir)) {
    const name = basename(abs);
    const kind = name.split('.')[0];
    files.push({ kind, file: name, abs, stemId: null });
  }
  return files;
}

function load(f) {
  try {
    f.doc = JSON.parse(readFileSync(f.abs, 'utf8'));
    return null;
  } catch (e) {
    f.doc = undefined;
    return { file: f.file, pointer: '', message: `invalid JSON: ${e.message}` };
  }
}

/**
 * Validate a set of files together (refs resolve within the set).
 * @param {{kind:string,file:string,abs?:string,doc?:any,stemId?:string|null}[]} files
 */
export function validateSet(files, validators) {
  const errors = [];
  for (const f of files) {
    if (f.doc === undefined && f.abs) {
      const e = load(f);
      if (e) {
        errors.push(e);
        continue;
      }
    }
    if (!KINDS.includes(f.kind)) {
      errors.push({ file: f.file, pointer: '', message: `unknown file kind '${f.kind}'` });
      f.doc = undefined;
      continue;
    }
    for (const e of checkFile(f.kind, f.doc, validators)) errors.push({ file: f.file, ...e });
    if (f.stemId && f.doc && typeof f.doc.id === 'string' && f.doc.id !== f.stemId) {
      errors.push({ file: f.file, pointer: '/id', message: `id '${f.doc.id}' must equal file name '${f.stemId}'` });
    }
    if (f.stemId && !SLUG.test(f.stemId)) {
      errors.push({ file: f.file, pointer: '', message: `file name '${f.stemId}' is not a snake_case slug` });
    }
  }
  errors.push(...checkRefs(files.filter((f) => f.doc !== undefined)));
  // dedupe (schema and rules can report the same thing)
  const seen = new Set();
  return errors.filter((e) => {
    const k = `${e.file}|${e.pointer}|${e.message}`;
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}

export function validateRoot(root) {
  const validators = loadValidators(root);
  const { files, unchecked } = discover(root);
  return { errors: validateSet(files, validators), checked: files.length, unchecked };
}

/**
 * Run the fixture suite: valid/* must pass together; every invalid/* (validated in the
 * context of the valid set) must fail and mention its `_expect_error` substring.
 */
export function runFixtures(root) {
  const validators = loadValidators(root);
  const fx = join(root, 'schemas', 'fixtures');
  const problems = [];
  const valid = discoverFixtures(join(fx, 'valid'));
  for (const e of validateSet(valid, validators)) {
    problems.push({ file: `valid/${e.file}`, pointer: e.pointer, message: `valid fixture rejected: ${e.message}` });
  }
  const invalid = discoverFixtures(join(fx, 'invalid'));
  for (const inv of invalid) {
    const raw = JSON.parse(readFileSync(inv.abs, 'utf8'));
    const expect = raw._expect_error;
    if (typeof expect !== 'string' || !expect) {
      problems.push({ file: `invalid/${inv.file}`, pointer: '', message: 'fixture lacks _expect_error' });
      continue;
    }
    const ctx = discoverFixtures(join(fx, 'valid')).map((v) => ({ ...v }));
    delete raw._expect_error;
    const errs = validateSet([...ctx, { ...inv, doc: raw }], validators).filter((e) => e.file === inv.file);
    if (errs.length === 0) {
      problems.push({ file: `invalid/${inv.file}`, pointer: '', message: `expected error '${expect}' but file passed` });
    } else if (!errs.some((e) => `${e.file}:${e.pointer}: ${e.message}`.includes(expect))) {
      problems.push({
        file: `invalid/${inv.file}`,
        pointer: '',
        message: `expected error containing '${expect}', got: ${errs.map((e) => e.message).join(' | ')}`,
      });
    }
  }
  return { problems, valid: valid.length, invalid: invalid.length };
}
