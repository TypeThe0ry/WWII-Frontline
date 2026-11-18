// M1-070: audit of `historical.*` provenance for the M1 weapons/explosives/vehicle (REFERENCE_MEASUREMENTS 6).
// Usage: node tools/data-validate/source-audit.mjs [--root data] [--write docs/measurements/hist-review-m1.md]
// A field counts as sourced only when `_source[]` has a record for it with a non-empty locator; otherwise it is
// a gap ("未核实"). Exit 1 if a file is `reviewed` while it still has gaps (a reviewed file may not hide them).
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIRS = ['weapons', 'explosives', 'vehicles'];

/** @returns {{file:string,id:string,status:string,rows:{field:string,state:'sourced'|'gap',source:string}[]}} */
export function auditDoc(file, doc) {
  const hist = doc.historical && typeof doc.historical === 'object' ? doc.historical : {};
  const src = Array.isArray(hist._source) ? hist._source : [];
  const unverified = new Set(Array.isArray(doc._meta?.unverified) ? doc._meta.unverified : []);
  const rows = [];
  for (const key of Object.keys(hist)) {
    if (key.startsWith('_')) continue;
    const rec = src.find((r) => r && r.field === key && typeof r.locator === 'string' && r.locator.trim() !== '');
    const gap = !rec || unverified.has(`historical.${key}`) || hist[key] === '未核实';
    rows.push({ field: key, state: gap ? 'gap' : 'sourced', declared: gap && (unverified.has(`historical.${key}`) || hist[key] === '未核实'), source: rec ? `${rec.source_id} ${rec.locator}`.slice(0, 120) : '未核实' });
  }
  return { file, id: doc.id ?? file, status: doc._meta?.status ?? 'initial', rows };
}

export function auditRoot(root) {
  const out = [];
  for (const d of DIRS) {
    for (const name of readdirSync(join(root, d)).filter((n) => n.endsWith('.json')).sort()) {
      out.push(auditDoc(`${d}/${name}`, JSON.parse(readFileSync(join(root, d, name), 'utf8'))));
    }
  }
  return out;
}

/** Problems: reviewed files with a gap that is not declared (`_meta.unverified` or the 未核实 value). */
export const problems = (audit) => audit.filter((a) => a.status === 'reviewed' && a.rows.some((r) => r.state === 'gap' && !r.declared)).map((a) => `${a.file}: reviewed but has undeclared unsourced fields`);

export function toMarkdown(audit) {
  const gaps = audit.reduce((n, a) => n + a.rows.filter((r) => r.state === 'gap').length, 0);
  const total = audit.reduce((n, a) => n + a.rows.length, 0);
  const L = ['# M1 史实参数复核记录（M1-070）', '', `由 \`tools/data-validate/source-audit.mjs\` 生成。字段 ${total} 个，已有出处 ${total - gaps}，未核实 ${gaps}。只记录数值事实与来源 ID，不抄手册原文；未核实的字段一律保持 \`initial\`，不凭记忆填充。`, '', '| 文件 | 状态 | 字段 | 复核结果 | 来源 |', '| --- | --- | --- | --- | --- |'];
  for (const a of audit) for (const r of a.rows) L.push(`| ${a.file} | ${a.status} | ${r.field} | ${r.state === 'sourced' ? '一致（见来源）' : '未核实'} | ${r.source} |`);
  return L.join('\n') + '\n';
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const a = process.argv.slice(2);
  const root = resolve(a.includes('--root') ? a[a.indexOf('--root') + 1] : join(fileURLToPath(import.meta.url), '..', '..', '..', 'data'));
  const audit = auditRoot(root);
  const probs = problems(audit);
  if (a.includes('--write')) writeFileSync(a[a.indexOf('--write') + 1], toMarkdown(audit));
  const gaps = audit.reduce((n, x) => n + x.rows.filter((r) => r.state === 'gap').length, 0);
  console.error(`${audit.length} files, ${gaps} unsourced field(s), ${probs.length} problem(s)`);
  for (const p of probs) console.log(p);
  process.exit(probs.length ? 1 : 0);
}
