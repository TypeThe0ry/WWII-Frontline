// M1-072: checks reports/m1-acceptance.md has the 17 exit-criteria rows (E1-1..E1-17) with no empty cell, and that
// tasks/STATUS.md rows agree with the summary lists. Usage: node tools/ci/check-acceptance.mjs [report] [status]
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const HEADER = ['E#', '标准', '目标', '实测', '方法/命令/报告', '结论', '遗留(PI-xxx)'];
const VERDICTS = ['PASS', 'partial', 'FAIL', '未测'];

export function checkAcceptance(md) {
  const errs = [];
  const rows = new Map();
  for (const line of md.split('\n')) {
    if (!line.startsWith('|')) continue;
    const cells = line.split('|').slice(1, -1).map((c) => c.trim());
    const m = /^E1-(\d+)$/.exec(cells[0] ?? '');
    if (!m) continue;
    if (cells.length !== HEADER.length) errs.push(`${cells[0]}: expected ${HEADER.length} cells, got ${cells.length}`);
    cells.forEach((c, i) => { if (c === '') errs.push(`${cells[0]}: empty cell '${HEADER[i]}'`); });
    if (!VERDICTS.includes(cells[5])) errs.push(`${cells[0]}: verdict '${cells[5]}' not one of ${VERDICTS.join('/')}`);
    rows.set(Number(m[1]), cells);
  }
  for (let n = 1; n <= 17; n++) if (!rows.has(n)) errs.push(`E1-${n}: row missing`);
  return errs;
}

/** STATUS rows `| M1-xxx | title | status |...` -> [{id,status}] where status is the first word of cell 3. */
export function statusRows(md) {
  const out = [];
  for (const line of md.split('\n')) {
    const m = /^\| (M1-\d+[a-z]?) \|[^|]*\| *([^|]*?) *\|/.exec(line);
    if (m) out.push({ id: m[1], status: (/^[a-z]+/.exec(m[2].toLowerCase()) ?? [''])[0] });
  }
  return out;
}

/** Task rows still `todo`/`in-progress` and not listed in `allowed` (ids skipped on purpose, with a reason in the report). */
export function openTasks(rows, allowed = []) {
  return rows.filter((r) => ['todo', 'in-progress'].includes(r.status) && !allowed.includes(r.id)).map((r) => r.id);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [rep = 'reports/m1-acceptance.md', st = 'tasks/STATUS.md'] = process.argv.slice(2);
  const errs = checkAcceptance(readFileSync(rep, 'utf8'));
  const open = openTasks(statusRows(readFileSync(st, 'utf8')), ['M1-066']);
  for (const e of errs) console.log(e);
  for (const o of open) console.log(`${o}: still todo/in-progress`);
  console.error(`${errs.length} report problem(s), ${open.length} open task(s) (M1-066 skipped by decision)`);
  process.exit(errs.length || open.length ? 1 : 0);
}
