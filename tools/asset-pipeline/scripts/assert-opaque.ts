// usage: node tools/asset-pipeline/scripts/assert-opaque.ts <file.fpak>...  — lists, per mesh/texture entry, which generic tools recognise it
import { readFileSync } from 'node:fs';
import { BufferSource, FpakReader } from '../src/fpak/index.ts';
import { checkOpaque } from '../src/fpak/opaque.ts';

let bad = 0;
for (const f of process.argv.slice(2)) {
  const r = await FpakReader.open(new BufferSource(readFileSync(f)));
  for (const e of r.entries().filter((x) => x.kind === 'mesh' || x.kind === 'texture')) {
    const checks = await checkOpaque(await r.read(e.logical_id));
    const hit = checks.filter((c) => c.recognized);
    bad += hit.length;
    console.log(`${hit.length ? 'FAIL' : 'ok  '} ${f} :: ${e.logical_id} (${e.kind}) ` + checks.map((c) => `${c.tool}=${c.recognized ? 'RECOGNISED' : 'no'}`).join('; '));
  }
}
process.exitCode = bad ? 1 : 0;
