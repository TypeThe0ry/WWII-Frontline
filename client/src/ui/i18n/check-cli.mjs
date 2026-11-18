// `npm run i18n:check` — exits non-zero on missing/extra keys or placeholder mismatch.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const dir = fileURLToPath(new URL('.', import.meta.url));
const load = (f) => JSON.parse(readFileSync(dir + f, 'utf8'));
const en = load('en.json');
const zh = load('zh-Hans.json');
const ph = (m) => [...new Set([...m.matchAll(/\{\s*([A-Za-z_]\w*)\s*(?=[,}])/g)].map((x) => x[1]).filter((w) => !['one', 'other', 'few', 'many', 'zero', 'plural'].includes(w)))].sort().join(',');
let bad = 0;
for (const k of Object.keys(en)) {
  if (!(k in zh)) (console.error(`missing zh-Hans: ${k}`), bad++);
  else if (ph(en[k]) !== ph(zh[k])) (console.error(`placeholder mismatch: ${k}`), bad++);
}
for (const k of Object.keys(zh)) if (!(k in en)) (console.error(`extra zh-Hans: ${k}`), bad++);
console.log(`i18n:check ${Object.keys(en).length} keys, ${bad} issue(s)`);
process.exit(bad ? 1 : 0);
